

use std::collections::HashMap;

use oauth2::{
    AuthorizationCode,
    AuthUrl,
    ClientId,
    ClientSecret,
    CsrfToken,
    PkceCodeChallenge,
    RedirectUrl,
    Scope,
    TokenResponse,
    TokenUrl
};
use oauth2::basic::BasicClient;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use worker::{console_log, Error, Response};
use reqwest::Response as ReqwestResponse;

#[derive(Deserialize, Debug, Serialize)]
pub struct AuthURL{
    pub url: Url,
    pub csrf_token: CsrfToken
}

#[derive(Deserialize, Debug, Serialize)]
pub struct AuthResponse {
    pub name: String,
    pub login: String,
    pub id: u64,
}


#[derive(Deserialize, Debug, Serialize)]
pub struct AuthToken {
    pub access_token: String,
    pub token_type: String,
    pub scope: String,
}

pub async fn github_auth_url_builder(client_id: &str, client_secret: &str ) -> Result<AuthURL, Error> {
    let client = BasicClient::new(
        ClientId::new(client_id.to_string()),
        Some(ClientSecret::new(client_secret.to_string())),
        AuthUrl::new("https://github.com/login/oauth/authorize".to_string())?,
        Some(TokenUrl::new("https://github.com/login/oauth/access_token".to_string())?)
    )
    .set_redirect_uri(RedirectUrl::new("https://api.shivadityas.com/callback/github".to_string())?);
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    // Generate the full authorization URL.
    let (auth_url, csrf_token) = client
                        .authorize_url(CsrfToken::new_random)
                        // Set the desired scopes.
                        .add_scope(oauth2::Scope::new("user:email".to_string()))
                        .add_scope(oauth2::Scope::new("read:user".to_string()))
                        .add_scope(oauth2::Scope::new("read:org".to_string()))
                        .add_scope(oauth2::Scope::new("public_repo".to_string()))
                        // Set the PKCE code challenge.
                        .set_pkce_challenge(pkce_challenge)
                        .url();
    let auth_body = AuthURL {
        url: auth_url,
        csrf_token
    };
    Ok(auth_body)
 }

 pub async fn github_auth_profile(code: String, client_id: &str, client_secret: &str) -> Result<AuthToken, Response> {
    let mut params = HashMap::new();
    params.insert("client_id", client_id);
    params.insert("client_secret", client_secret);
    params.insert("code", &code);
    let client = reqwest::Client::new();
    let response = client
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .form(&params)
        .send().await;
    match response {
        Ok(mut response) => {
            let body = response.text().await.unwrap();
            let auth_token: AuthToken = match serde_json::from_str::<AuthToken>(&body){
                Ok(auth_token) => auth_token,
                Err(e) => {
                    return Err(Response::error(&format!("Error: {}", e), 500).unwrap());
                }
            };
            Ok(auth_token)
        }
        Err(e) => Err(Response::error(&format!("Error: {}", e), 500).unwrap()),
    }
 }    
    
pub async fn get_github_profile_details(token: AuthToken) -> Result<AuthResponse, Response>{
    let client = reqwest::Client::new();
    let response = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("{} {}", token.token_type, token.access_token))
        .header("User-Agent", "reqwest".to_string())
        .send().await;
    match response {
        Ok(mut response) => {
            let body = response.text().await.unwrap();
            console_log!("{:?}", body);
            let auth_response = match serde_json::from_str::<AuthResponse>(&body) {
                Ok(auth_response) => auth_response,
                Err(e) => {
                    return Err(Response::error(&format!("Error: {}", e), 500).unwrap());
                }
            };
            Ok(auth_response)
        }
        Err(e) => {
            Err(Response::error(&format!("Errorss: {}", e), 500).unwrap())
        }
    }
}   

pub async fn get_details_from_id(id: u64) -> Result<AuthResponse, Response>{
    let client = reqwest::Client::new();
    let response = client
        .get(format!("https://api.github.com/user/{}", id))
        .header("User-Agent", "reqwest".to_string())
        .send().await;
    match response {
        Ok(mut response) => {
            let body = response.text().await.unwrap();
            console_log!("{:?}", body);
            let auth_response = match serde_json::from_str::<AuthResponse>(&body) {
                Ok(auth_response) => auth_response,
                Err(e) => {
                    return Err(Response::error(&format!("Error: {}", e), 500).unwrap());
                }
            };
            Ok(auth_response)
        }
        Err(e) => {
            Err(Response::error(&format!("Errorss: {}", e), 500).unwrap())
        }
    }
}