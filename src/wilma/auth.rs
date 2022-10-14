use reqwest::{Client, Url, IntoUrl};

use anyhow::Result;

use serde::Deserialize;
use serde_json::from_slice;

use webbrowser;

use std::collections::HashMap;

use base64_url;
use rand::{distributions::Alphanumeric, Rng};
use sha2::{Digest, Sha256};

use crate::ipc::{self, IPCMessage};
use crate::structs::{OpenIDConfiguration, OpenIDProvider};

#[derive(Deserialize)]
pub struct TokenData {
    pub access_token: String,
    pub id_token: String,
}

fn generate_code() -> (String, String) {
    let code_verifier: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect();

    let mut hasher = Sha256::new();
    hasher.update(&code_verifier);

    let code_challenge = base64_url::encode(&hasher.finalize());

    (code_challenge, code_verifier)
}

pub async fn oauth_authorize(
    client: &Client,
    provider: &OpenIDProvider
) -> Result<()> {
    let configuration: OpenIDConfiguration =
        from_slice(&client.get(&provider.configuration).send().await?.bytes().await?)?;

    let (code_challenge, code_verifier) = generate_code();

    let state: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect();

    let mut auth_url = Url::parse(configuration.authorization_endpoint.as_str())?;
    auth_url
        .query_pairs_mut()
        .append_pair("client_id", provider.client_id.as_str())
        .append_pair("response_type", "code")
        .append_pair("scope", "openid+email")
        .append_pair("redirect_uri", "wilma://oauth")
        .append_pair("state", state.as_str())
        .append_pair("code_challenge_method", "S256")
        .append_pair("code_challenge", code_challenge.as_str())
        .finish();

    webbrowser::open(auth_url.as_str())?;
    ipc::send_data(IPCMessage::TokenRequest {
        state,
        client_id: provider.client_id.clone(),
        token_endpoint: configuration.token_endpoint,
        code_verifier,
    })
    .await?;
    Ok(())
}

pub async fn oauth_authenticate(
    client: &Client,
    protocol_url: Url,
    token_url: impl IntoUrl,
    client_id: String,
    code_verifier: String,
) -> Result<TokenData> {
    assert!(protocol_url.scheme() == "wilma", "Invalid url scheme");

    let params: HashMap<String, String> = protocol_url
        .query_pairs()
        .map(|(a, b)| (a.into_owned(), b.into_owned()))
        .collect();

    let code = params.get("code".into()).expect("Missing code");

    let params = [
        ("client_id", client_id.as_str()),
        ("grant_type", "authorization_code"),
        ("code", code.as_str()),
        ("code_verifier", code_verifier.as_str()),
    ];

    let response = client.post(token_url).form(&params).send().await?;
    Ok(from_slice(&response.bytes().await?)?)
}
