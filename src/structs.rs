use reqwest::Url;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct WilmaIndexJson {
    #[serde(rename="LoginResult")]
    pub login_result: String,
    #[serde(rename="SessionID")]
    pub session_id: String,
    #[serde(rename="ApiVersion")]
    pub api_version: i32,
    pub oidc_test_mode: Option<bool>,
    pub oidc_providers: Option<Vec<OpenIDProvider>>
}

#[derive(Deserialize, Debug)]
pub struct OpenIDProvider {
    pub name: String,
    pub client_id: String,
    pub configuration: String,
    pub scope: String,
}

#[derive(Deserialize, Debug)]
pub struct OpenIDConfiguration {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
}

#[derive(Deserialize, Debug)]
pub struct WilmaHubWilma {
    pub url: String,
    pub name: String,
}