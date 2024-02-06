use std::f32::consts::E;

use jsonwebtoken::get_current_timestamp;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use worker::console_log;
use worker::Error;
use worker::RouteContext;
use worker::Request;
use worker::Response;
use once_cell::sync::Lazy;

use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use crate::oauth::github_oauth::AuthResponse;

enum AuthError {
    InvalidToken,
    MissingToken,
    ExpiredToken,
    TokenCreation,
    ClientIDOrSecret,
    MissingUserID,
}

impl AuthError {
    fn to_string(&self) -> String {
        match self {
            AuthError::InvalidToken => "Invalid Token".to_string(),
            AuthError::MissingToken => "Missing Token".to_string(),
            AuthError::ExpiredToken => "Expired Token".to_string(),
            AuthError::TokenCreation => "Token Creation Error".to_string(),
            AuthError::ClientIDOrSecret => "Invalid Client ID or Secret".to_string(),
            AuthError::MissingUserID => "Missing User ID".to_string(),
        }
    }
    fn status(&self) -> u16 {
        match self {
            AuthError::InvalidToken => 401,
            AuthError::MissingToken => 403,
            AuthError::ExpiredToken => 403,
            AuthError::TokenCreation => 500,
            AuthError::ClientIDOrSecret => 400,
            AuthError::MissingUserID => 400,
        }
    }
    fn response(&self) -> Response {
        Response::error(&self.to_string(), self.status()).unwrap()
    }
}


#[derive(Serialize, Deserialize, Debug)]
struct Claims {
    sub: String,
    company: String,
    exp: usize,
}

#[derive(Deserialize, Debug)]
struct AuthPayload {
    client_id: String,
    client_secret: String,
    user_id: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct AuthBody {
    token: String,
    token_type: String,
}

impl AuthBody {
    fn new_token(access_token: String) -> Self {
        Self {
            token: access_token,
            token_type: "Bearer".to_string(),
        }
    }
}

struct Keys {
    encoding: EncodingKey,
    decoding: DecodingKey,
}

impl Keys {
    fn new(secret: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
    }
}


//Must be obtained from the environment in PROD
const key: &str = "secret";
static KEYS: Lazy<Keys> = Lazy::new(|| Keys::new(key.as_bytes()));

pub async fn authorize(mut req: Request, ctx: RouteContext<()>) -> Result<Response, Error> {
    let client_id = match req.headers().get("client_id"){
        Ok(Some(client_id)) => client_id,
        Ok(None) => return Response::error(AuthError::ClientIDOrSecret.to_string(), AuthError::ClientIDOrSecret.status()),
        Err(_) => return Response::error(AuthError::ClientIDOrSecret.to_string(), AuthError::ClientIDOrSecret.status())
    };
    let client_secret = match req.headers().get("client_secret"){
        Ok(Some(client_secret)) => client_secret,
        Ok(None) => return Response::error(AuthError::ClientIDOrSecret.to_string(), AuthError::ClientIDOrSecret.status()),
        Err(_) => return Response::error(AuthError::ClientIDOrSecret.to_string(), AuthError::ClientIDOrSecret.status())
    };
    let user_id = match req.headers().get("user_id"){
        Ok(Some(user_id)) => user_id,
        Ok(None) => return Response::error(AuthError::MissingUserID.to_string(), AuthError::MissingUserID.status()),
        Err(_) => return Response::error(AuthError::MissingUserID.to_string(), AuthError::MissingUserID.status())
    };

    //Create AuthPayload
    let auth_payload = AuthPayload {
        client_id: client_id.to_string(),
        client_secret: client_secret.to_string(),
        user_id: user_id.to_string(),
    };
    //Check credentials
    let auth_reponse = match check_credentials(auth_payload, ctx).await {
        Ok(claims) => {
            let auth_response = crate::oauth::github_oauth::get_details_from_id(claims.uuid).await;
            match auth_response {
                Ok(auth_response) => auth_response,
                Err(e) => return Ok(e)
            }
        },
        Err(e) => return Response::error(e.to_string(), e.status())
    };

    //Cast AuthPayload to Header
    let header = Header::default();
    let claims = Claims {
        sub: auth_reponse.login,
        company: auth_reponse.name,
        exp: get_current_timestamp() as usize + 3600,

    };
    //Create a new token
    match encode(&header, &claims, &KEYS.encoding).map_err(|_| AuthError::TokenCreation) {
        Ok(token) => Response::from_json(&AuthBody::new_token(token)),
        Err(e) => Response::error(e.to_string(), 500)
    }
}


async fn check_credentials(auth_payload: AuthPayload, ctx: RouteContext<()>) -> Result<ClientCredentials, AuthError> {
    if auth_payload.client_id.is_empty() || auth_payload.client_secret.is_empty() {
        return Err(AuthError::ClientIDOrSecret);
    }
    let mut kv = match ctx.kv("AUTH") {
        Ok(kv) => {
            kv
        },
        Err(_) => return Err(AuthError::ClientIDOrSecret)
    };
    match kv.get(&auth_payload.user_id).json::<ClientCredentials>().await {
        Ok(Some(credentials)) => {
            console_log!("{:?}", credentials);
            if credentials.client_id == auth_payload.client_id && credentials.client_secret == auth_payload.client_secret {
                Ok(credentials)
            } else {
                Err(AuthError::ClientIDOrSecret)
            }
        }
        Ok(None) => {
            console_log!("No credentials found");
            Err(AuthError::ClientIDOrSecret)
        },
        Err(_) => Err(AuthError::ClientIDOrSecret)
    }
}

pub async fn token_middleware(req: Request, ctx: RouteContext<()>) -> Result<Request, Response> {
    let token = match req.headers().get("Authorization") {
        Ok(Some(token)) => token,
        Ok(None) => return Err(AuthError::MissingToken.response()),
        Err(_) => return Err(AuthError::MissingToken.response())
    };
    match validate_token(&token) {
        Ok(_) => Ok(req),
        Err(e) => Err(e.response())
    }
}

fn validate_token(token: &str) -> Result<&str, AuthError> {
    if !token.starts_with("Bearer ") {
        return Err(AuthError::InvalidToken);
    }

    let token_data = decode::<Claims>(
        &token[7..],
        &KEYS.decoding,
        &Validation::default(),
    )
    .map_err(|e| {
        console_log!("{:?}", e);
        AuthError::InvalidToken
    })?;

    if token_data.claims.exp < E as usize {
        return Err(AuthError::ExpiredToken);
    }

    Ok(&token[7..])
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClientCredentials {
    pub uuid: u64,
    pub client_id: String,
    pub client_secret: String,
}

pub async fn generate_client_id_and_secrets(mut resp: AuthResponse) -> ClientCredentials {
    let id = resp.id;
    // Generate a random client ID and client secret
    let client_id = generate_random_string(16);
    let client_secret = generate_random_string(32);

    ClientCredentials {
        uuid: id,
        client_id,
        client_secret,
    }
}

fn generate_random_string(length: usize) -> String {
    let mut rng = thread_rng();
    let random_string: String = rng
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect();

    random_string
}