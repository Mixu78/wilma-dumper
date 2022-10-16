use anyhow::{anyhow, Result};
use reqwest::{Client, Url};
use serde::Deserialize;
use serde_json::from_slice;

use api::models::WilmaHubWilma;

pub mod api;
pub mod auth;

pub use api::models;
pub use api::WilmaApi;

use self::models::WilmaRole;

const WILMA_HUB: &str = "https://wilmahub.service.inschool.fi/wilmat";

#[derive(Deserialize)]
struct WilmaHubResponse {
    wilmat: Vec<WilmaHubWilma>,
}

#[derive(Clone, Debug)]
pub struct Wilma {
    pub base_url: Url,
    pub name: String,

    sid: Option<String>,
    pub role: Option<WilmaRole>,
}

impl Wilma {
    fn new(base_url: Url, name: String) -> Self {
        Self {
            base_url,
            name,
            sid: None,
            role: None,
        }
    }

    pub fn from_url(base_url: Url) -> Self {
        Self::new(base_url, String::new())
    }

    pub fn is_authenticated(&self) -> bool {
        self.sid.is_some()
    }

    pub fn is_logged_in(&self) -> bool {
        self.sid.is_some() && self.role.is_some()
    }

    pub fn get_url(&self) -> Result<Url> {
        Ok(self.base_url.join(format!(
            "{}/",
            self.role
                .as_ref()
                .ok_or_else(|| anyhow!("No role selected"))?
                .slug
        ).as_str())?)
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
