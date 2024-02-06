use std::borrow::Borrow;

use oauth::github_oauth::{AuthResponse, AuthToken};
use pages::pages::{home_page, oauth_home_page};
use routes::utils::{generate_client_id_and_secrets, ClientCredentials};
use scraper::scraper::google_news_scraper;
use serde_json::json;
use worker::*;

mod scraper;
mod routes;
mod utils;
mod pages;
mod oauth;

fn log_request(req: &Request) {
    console_log!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or("unknown region".into())
    );
}




#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    log_request(&req);
    utils::set_panic_hook();
    let router = Router::new();
    router
        .get("/", |_, _| Response::from_html(home_page()))
        .get_async("/api/scrape", |req, ctx| async move {
            match crate::routes::utils::token_middleware(req, ctx).await {
                Ok(_) => {
                    let result = google_news_scraper().await;
                    match result {
                        Ok(articles) => {
                            Response::from_json(&json!({ "Result": articles }))
                        }
                        Err(e) => {
                            Response::error(&format!("Error: {}", e), 500)
                        }
                    }
                },
                Err(e) => Ok(e)
            }
        })
        .get("/oauth", |_, _| Response::from_html(oauth_home_page()))
        .get_async("/oauth/github", |req, ctx| async move {
            let client_id = ctx.secret("github_client_id").unwrap().to_string();
            let client_secret = ctx.secret("github_client_secret").unwrap().to_string();
            let auth_url = crate::oauth::github_oauth::github_auth_url_builder(&client_id, &client_secret).await;
            match auth_url {
                Ok(auth_url) => {
                    let url = auth_url.url.to_string();
                    let csrf_token = auth_url.csrf_token.secret();
                    let mut req_header = Headers::new();
                    let mut csrf_string = format!("auth_token={}", csrf_token);
                    csrf_string.push_str("; Path=/");  
                    req_header.set("Set-Cookie", &csrf_string)?;
                    req_header.set("Location", &url)?;
                    let red_resp = Response::ok("Redirecting to Github for authorization").unwrap().with_headers(req_header).with_status(302);
                    Ok(red_resp)
                }
                Err(e) => {
                    Response::error(&format!("Error: {}", e), 500)
                }
            }
        })
        .get_async("/callback/github", |req, ctx| async move {
            let (state, code) = match req.url()?.query() {
                Some(query) => {
                    let mut state = None;
                    let mut code = None;
                    for pair in query.split("&") {
                        let mut key_value = pair.split("=");
                        let key = key_value.next().unwrap();
                        let value = key_value.next().unwrap();
                        match key {
                            "state" => state = Some(value.to_string()),
                            "code" => code = Some(value.to_string()),
                            "error" => {
                                return Response::error(&format!("Error: {}", value), 400);
                            }
                            _ => {}
                        } 
                    }
                    (state.unwrap_or_else(|| String::new()), code.unwrap_or_else(|| String::new()))
                }
                None => {
                    return Response::error("Invalid query", 400);
                }
            };
            if(code.is_empty() || state.is_empty()){
                return Response::error("Invalid code", 400);
            }
            let csrf_token = match req.headers().get("Cookie") {
                Ok(Some(cookie_header)) => {
                    let mut csrf_cookie = None;
                    for cookie in cookie_header.split(";") {
                        let mut key_value = cookie.split("=");
                        let key = key_value.next().unwrap();
                        let value = key_value.next().unwrap();
                        if key == " auth_token" {
                            csrf_cookie = Some(value.to_string());
                        }
                    }
                    csrf_cookie.unwrap_or_else(|| {
                        String::new()
                    })
                }
                Ok(None) => {
                    String::new()
                }
                Err(_) => { 
                    //Return an error
                    String::new()
                }
            };
            if(csrf_token != state){
                return Response::error("Invalid CSRF Token", 400);
            }
            let client_id = ctx.secret("github_client_id").unwrap().to_string();
            let client_secret = ctx.secret("github_client_secret").unwrap().to_string();
            let user_profile = match crate::oauth::github_oauth::github_auth_profile(code,&client_id, &client_secret).await {
                Ok(auth_token) => {
                    match crate::oauth::github_oauth::get_github_profile_details(auth_token).await {
                        Ok(profile) => {
                            profile
                        }
                        Err(e) => {
                            return Ok(e);
                        }
                    }
                }
                Err(e) => {
                    return Ok(e)
                }
            };
            let mut kv = ctx.kv("AUTH");
            let result = match kv?.get(&user_profile.borrow().id.to_string()).json::<ClientCredentials>().await{
                Ok(Some(creds)) => {
                    creds
                }
                Ok(None)=> {
                    let creds: ClientCredentials = generate_client_id_and_secrets(user_profile).await;
                    kv = ctx.kv("AUTH");
                    match kv {
                        Ok(kv) => {
                            let kv_copy = kv; // Create a separate variable for the moved value
                            match kv_copy.put::<String>(&creds.uuid.to_string(), serde_json::to_string(&creds).unwrap())?.execute().await{
                                Ok(_) => {
                                    Response::from_json(&json!({ "Result": creds }));
                                }
                                Err(e) => {
                                    Response::error(&format!("Error: {}", e), 500);
                                }
                            }
                        }
                        Err(_) => {
                            return Response::error(&format!("Error: Error in creating creds"), 500);
                        }
                    }
                    creds
                }
                Err(e) => {
                    return Response::error(&format!("Error: {}", e), 500);
                }
            };
            Response::from_json(&json!({ "Result": result }))
        })
        .post_async("/authorize", crate::routes::utils::authorize) 
        .run(req, env)
        .await
}
