use crate::ipc::{self, IPCMessage};
use crate::wilma::{self, Wilma};

use super::{Interface, InterfaceOptions};

use clap::error::ErrorKind;
use clap::{CommandFactory, Parser, Subcommand};
use dialoguer::theme::ColorfulTheme;
use tokio::runtime::Handle;

use reqwest::Url;

use anyhow::{anyhow, Result};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(short, long, value_parser = Url::parse)]
    wilma: Option<Url>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {}

pub struct CliInterface {
    rt: Handle,
}

impl Interface for CliInterface {
    fn new(handle: Handle) -> Self {
        Self { rt: handle }
    }

    fn start(self, options: InterfaceOptions) -> Result<()> {
        self.rt.block_on(async {
            let cli = Cli::parse();

            let mut wilma = self.get_wilma(&options, &cli).await?;

            let auth_code = match wilma.get_providers(&options.client).await? {
                Some(mut providers) => {
                    let selection = dialoguer::FuzzySelect::with_theme(&ColorfulTheme::default())
                        .with_prompt("Select OpenID provider")
                        .items(
                            &providers
                                .iter()
                                .map(|p| p.name.as_str())
                                .collect::<Vec<&str>>(),
                        )
                        .default(0)
                        .interact()?;
                    let provider = providers.swap_remove(selection);

                    wilma::auth::oauth_authorize(&options.client, &provider).await?;
                    match ipc::receive_data().await? {
                        IPCMessage::TokenResponse {
                            access_token,
                            id_token,
                        } => wilma.openid_login(
                            &options.client,
                            provider.configuration.clone(),
                            provider.client_id.clone(),
                            access_token,
                            id_token,
                        ).await?,
                        _ => unreachable!(),
                    };

                    Ok(())
                }
                None => Err(anyhow!("Selected wilma does not support OpenID")),
            };

            println!("{auth_code:?}");

            auth_code
        })
    }
}

impl CliInterface {
    async fn get_wilma(&self, options: &InterfaceOptions, cli: &Cli) -> Result<Wilma> {
        let wilma = match &cli.wilma {
            Some(url) => {
                let wilma = Wilma::from_url(url.clone());
                match wilma.is_wilma(&options.client).await? {
                    true => wilma,
                    false => Cli::command()
                        .error(
                            ErrorKind::InvalidValue,
                            "Given url was not a valid wilma url",
                        )
                        .exit(),
                }
            }
            None => {
                let mut wilmas = wilma::get_wilmas(&options.client).await?;

                let selection = dialoguer::FuzzySelect::with_theme(&ColorfulTheme::default())
                    .with_prompt("Select wilma:")
                    .items(
                        &wilmas
                            .iter()
                            .map(|w| w.name.as_str())
                            .collect::<Vec<&str>>(),
                    )
                    .default(0)
                    .interact()?;

                wilmas.swap_remove(selection)
            }
        };

        Ok(wilma)
    }
}
