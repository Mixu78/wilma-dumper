use std::collections::HashMap;

use anyhow::{anyhow, Result};
use reqwest::{Client, StatusCode, Url};
use serde::Deserialize;
use serde_json::{from_slice, to_string};

use crate::structs::{OpenIDConfiguration, OpenIDProvider, WilmaHubWilma, WilmaIndexJson};

pub mod auth;

const WILMA_HUB: &str = "https://wilmahub.service.inschool.fi/wilmat";

#[derive(Deserialize)]
struct WilmaHubResponse {
    wilmat: Vec<WilmaHubWilma>,
}

#[derive(Debug)]
pub struct Wilma {
    pub url: Url,
    pub name: String,

    sid: Option<String>,
}

impl Wilma {
    fn new(url: Url, name: String) -> Self {
        Self {
            url,
            name,
            sid: None,
        }
    }

    pub fn from_url(url: Url) -> Self {
        Self {
            url,
            name: "".into(),
            sid: None,
        }
    }

    async fn get_index_json(&self, client: &Client) -> Result<WilmaIndexJson> {
        let response = client.get(self.url.join("/index_json")?).send().await?;

        Ok(from_slice(&response.bytes().await?)?)
    }

    pub async fn is_wilma(&self, client: &Client) -> Result<bool> {
        Ok(self.get_index_json(client).await.is_ok())
    }

    pub async fn get_providers(&self, client: &Client) -> Result<Option<Vec<OpenIDProvider>>> {
        let data = self.get_index_json(client).await?;

        Ok(data.oidc_providers)
    }

    pub async fn openid_login(
        &mut self,
        client: &Client,
        configuration: String,
        client_id: String,
        access_token: String,
        id_token: String,
    ) -> Result<()> {
        let session_id = self.get_index_json(client).await?.session_id;

        let mut payload = HashMap::<&str, String>::with_capacity(5);
        payload.insert("configuration", configuration);
        payload.insert("clientId", client_id);
        payload.insert("accessToken", access_token);
        payload.insert("sessionId", session_id);
        payload.insert("idToken", id_token);

        let response = client
            .post(self.url.join("/api/v1/external/openid/login")?)
            .form(&[("payload", to_string(&payload)?)])
            .send()
            .await?;

        let sid = response
            .headers()
            .get("Set-Cookie")
            .ok_or(anyhow!("Response did not contain Wilma2SID cookie"))?
            .to_str()?;

        self.sid = Some(sid.to_string());

        Ok(())
    }
}

pub async fn get_wilmas(client: &Client) -> Result<Vec<Wilma>> {
    let response = client.get(WILMA_HUB).send().await?;

    let value: WilmaHubResponse = from_slice(&response.bytes().await?)?;

    let mut wilmas: Vec<Wilma> = Vec::with_capacity(value.wilmat.capacity());

    for w in value.wilmat {
        match Url::parse(w.url.as_str()) {
            Ok(url) => wilmas.push(Wilma::new(url, w.name)),
            Err(_) => continue,
        }
    }

    Ok(wilmas)
}
