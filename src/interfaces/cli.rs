use crate::dump;
use crate::ipc::{self, IPCMessage};
use crate::wilma::{self, api::models::OpenIDProvider, Wilma, WilmaApi};

use super::{Interface, InterfaceContext};

use clap::error::ErrorKind;
use clap::{CommandFactory, Parser, Subcommand};
use dialoguer::theme::ColorfulTheme;
use tokio::runtime::Handle;

use reqwest::Url;

use anyhow::{anyhow, Result};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[arg(short, long, value_parser = Url::parse)]
    wilma: Option<Url>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Courses {
        #[command(subcommand)]
        subcommand: CourseOption,
    },
}

#[derive(Subcommand, Debug)]
enum CourseOption {
    StudyPoints,
    Dump {
        file: Option<String>,
        #[arg(long)]
        format: Option<String>,
    },
}

pub struct CliInterface {
    rt: Handle,
}

impl Interface for CliInterface {
    fn new(handle: Handle) -> Self {
        Self { rt: handle }
    }

    fn start(self, ctx: InterfaceContext) -> Result<()> {
        //Just block the thread, no gui here
        self.rt.block_on(async {
            let cli = Cli::parse();

            let mut wilma = self.get_wilma(&ctx, &cli).await?;
            self.login(&ctx, &mut wilma).await?;
            self.set_role(&ctx, &mut wilma).await?;

            match cli.command {
                Commands::Courses { subcommand } => {
                    let courses = wilma.get_courses(&ctx.client).await?;
                    match subcommand {
                        CourseOption::StudyPoints => {
                            let (selected, earned) =
                                dump::courses::calculate_study_points(&courses);

                            println!("Current credits: {earned}");
                            println!("Credits from selected courses: {selected}");
                        }
                        CourseOption::Dump { file, format } => {
                            let format = format
                                .unwrap_or_else(|| String::from("json"))
                                .to_lowercase();
                            let dump_format = match format.as_str() {
                                "json" => dump::courses::Format::Json,
                                "csv" => dump::courses::Format::Csv,
                                _ => {
                                    return Err(anyhow!("Invalid format: {}", format));
                                }
                            };

                            let path = match file {
                                Some(path) => path,
                                None => dialoguer::Input::with_theme(&ColorfulTheme::default())
                                    .with_prompt("Path to dump file")
                                    .default(format!("courses.{}", format))
                                    .interact_text()?,
                            };

                            let file = std::fs::File::create(path)?;
                            dump::courses::dump_to_writer(&courses, file, dump_format)?;
                        }
                    }
                }
            }

            Ok(())
        })
    }
}

impl CliInterface {
    async fn get_wilma(&self, ctx: &InterfaceContext, cli: &Cli) -> Result<Wilma> {
        let wilma = match &cli.wilma {
            Some(url) => {
                let wilma = Wilma::from_url(url.clone());
                match wilma.is_wilma(&ctx.client).await? {
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
                let mut wilmas = wilma::get_wilmas(&ctx.client).await?;

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

    async fn set_role(&self, ctx: &InterfaceContext, wilma: &mut Wilma) -> Result<()> {
        let roles = wilma.get_roles(&ctx.client).await?;

        let selection = dialoguer::Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select role:")
            .items(
                &roles
                    .iter()
                    .map(|r| format!("{} ({})", r.name, r.slug))
                    .collect::<Vec<String>>(),
            )
            .default(0)
            .interact()?;

        wilma.set_role(&roles[selection])?;

        Ok(())
    }

    fn get_provider(
        &self,
        _ctx: &InterfaceContext,
        providers: Vec<OpenIDProvider>,
    ) -> Result<OpenIDProvider> {
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
        Ok(providers[selection].clone())
    }

    async fn login(&self, ctx: &InterfaceContext, wilma: &mut Wilma) -> Result<()> {
        match wilma.get_providers(&ctx.client).await? {
            Some(providers) => {
                let provider = self.get_provider(ctx, providers)?;

                wilma::auth::oauth_authorize(&ctx.client, &provider).await?;
                match ipc::receive_data().await? {
                    IPCMessage::TokenResponse {
                        access_token,
                        id_token,
                    } => {
                        wilma
                            .openid_login(
                                &ctx.client,
                                provider.configuration.clone(),
                                provider.client_id.clone(),
                                access_token,
                                id_token,
                            )
                            .await?
                    }
                    _ => unreachable!(),
                };

                Ok(())
            }
            None => Err(anyhow!("Selected wilma does not support OpenID")),
        }
    }
}
