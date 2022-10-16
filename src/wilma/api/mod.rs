use std::collections::HashMap;

use async_trait::async_trait;

use anyhow::{anyhow, ensure, Result};
use log::debug;
use reqwest::Client;
use serde_json::{from_slice, to_string};

use super::Wilma;

pub mod courses;
pub mod models;

#[async_trait]
pub trait WilmaApi {
    async fn is_wilma(&self, client: &Client) -> Result<bool>;
    async fn get_index_json(&self, client: &Client) -> Result<models::WilmaIndexJson>;
    async fn get_providers(&self, client: &Client) -> Result<Option<Vec<models::OpenIDProvider>>>;
    async fn get_courses(&self, client: &Client) -> Result<Vec<models::Course>>;

    async fn openid_login(
        &mut self,
        client: &Client,
        configuration: String,
        client_id: String,
        access_token: String,
        id_token: String,
    ) -> Result<()>;
    async fn get_roles(&self, client: &Client) -> Result<Vec<models::WilmaRole>>;
    fn set_role(&mut self, role: &models::WilmaRole) -> Result<()>;
}

#[async_trait]
impl WilmaApi for Wilma {
    async fn get_index_json(&self, client: &Client) -> Result<models::WilmaIndexJson> {
        let response = client
            .get(self.base_url.join("/index_json")?)
            .send()
            .await?;

        Ok(from_slice(&response.bytes().await?)?)
    }

    async fn is_wilma(&self, client: &Client) -> Result<bool> {
        Ok(self.get_index_json(client).await.is_ok())
    }

    async fn get_providers(&self, client: &Client) -> Result<Option<Vec<models::OpenIDProvider>>> {
        let data = self.get_index_json(client).await?;

        Ok(data.oidc_providers)
    }

    async fn get_courses(&self, client: &Client) -> Result<Vec<models::Course>> {
        ensure!(self.is_logged_in(), "Not logged in");
        courses::get_courses(client, self).await
    }

    async fn openid_login(
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
            .post(self.base_url.join("/api/v1/external/openid/login")?)
            .form(&[("payload", to_string(&payload)?)])
            .send()
            .await?;

        let sid_header =
            response
                .headers()
                .get_all("Set-Cookie")
                .iter()
                .find(|c| match c.to_str() {
                    Ok(s) => s.starts_with("Wilma2SID="),
                    Err(_) => false,
                });

        let sid = sid_header
            .ok_or_else(|| anyhow!("No SID header"))?
            .to_str()?
            .split(';')
            .next()
            .ok_or_else(|| anyhow!("Malformed SID cookie"))?
            .split_at(10)
            .1;

        self.sid = Some(sid.to_string());

        Ok(())
    }

    async fn get_roles(&self, client: &Client) -> Result<Vec<models::WilmaRole>> {
        ensure!(self.sid.is_some(), "Session ID not set");

        let response = client
            .get(self.base_url.join("/api/v1/accounts/me/roles")?)
            .header(
                "Cookie",
                format!("Wilma2SID={};", self.sid.as_ref().unwrap()),
            )
            .send()
            .await?;

        let response: models::WilmaRoleResponse = from_slice(&response.bytes().await?)?;

        Ok(response.payload)
    }

    fn set_role(&mut self, role: &models::WilmaRole) -> Result<()> {
        debug!("Using role {role:?}");
        self.role = Some(role.clone());
        Ok(())
    }
}
