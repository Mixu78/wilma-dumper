use reqwest::{Client, IntoUrl, Url};

use anyhow::{ensure, Context, Result};
use log::*;

use serde::Deserialize;
use serde_json::from_slice;

use webbrowser;

use std::collections::HashMap;

use base64_url;
use rand::{distributions::Alphanumeric, Rng};
use sha2::{Digest, Sha256};

use crate::ipc::{self, IPCMessage};
use super::models::{OpenIDConfiguration, OpenIDProvider};

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

pub async fn oauth_authorize(client: &Client, provider: &OpenIDProvider) -> Result<()> {
    let configuration: OpenIDConfiguration = from_slice(
        &client
            .get(&provider.configuration)
            .send()
            .await?
            .bytes()
            .await?,
    )?;

    let (code_challenge, code_verifier) = generate_code();

    let state: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect();

    let mut auth_url = Url::parse(configuration.authorization_endpoint.as_str())?;
    // query_pairs_mut does encoding which breaks the scope
    auth_url.set_query(Some(
        format!(
            "\
    client_id={}&\
    response_type=code&\
    scope=openid+email&\
    redirect_uri=wilma://oauth&\
    state={state}&\
    code_challenge_method=S256&\
    code_challenge={code_challenge}\
    ", provider.client_id
        )
        .as_str(),
    ));

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
    ensure!(protocol_url.scheme() == "wilma", "Invalid url scheme");

    let params: HashMap<String, String> = protocol_url
        .query_pairs()
        .map(|(a, b)| (a.into_owned(), b.into_owned()))
        .collect();

    debug!("{protocol_url:?}");

    let code = params.get(&"code".to_string()).context("Missing code")?;

    let params = [
        ("client_id", client_id.as_str()),
        ("grant_type", "authorization_code"),
        ("redirect_uri", "wilma://oauth"),
        ("code", code.as_str()),
        ("code_verifier", code_verifier.as_str()),
    ];

    let response = client.post(token_url).form(&params).send().await?;
    from_slice(&response.bytes().await?).context("Unexpected oauth token response")
}
