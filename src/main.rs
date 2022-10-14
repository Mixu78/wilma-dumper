use std::{collections::HashMap, time::Duration};

use dialoguer;
use reqwest::{Client, Url};
use tokio::runtime::Runtime;

use anyhow::{anyhow, Result};

use windows::Win32::System::Console::GetConsoleProcessList;

use log::*;
use simple_logger::SimpleLogger;

use interfaces::{Interface, InterfaceOptions};

mod interfaces;
mod structs;
mod wilma;
mod ipc;
mod reg;

const DEFAULT_LOGGER_LEVEL: LevelFilter = if cfg!(debug_assertions) {
    LevelFilter::Debug
} else {
    LevelFilter::Info
};

fn get_client() -> Result<Client> {
    Ok(Client::builder()
        .user_agent(format!(
            "{}/{}",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        ))
        .build()?)
}

async fn handle_oauth(args: Vec<String>) -> Result<()> {
    let protocol_url = Url::parse(args.get(2).expect("Missing protocol url").as_str())?;
    assert!(protocol_url.scheme() == "wilma", "Invalid protocol url");
    let data = ipc::receive_data().await?;

    match data {
        ipc::IPCMessage::TokenRequest {
            state,
            client_id,
            token_endpoint: token_url,
            code_verifier,
        } => {
            let params: HashMap<String, String> = protocol_url
                .query_pairs()
                .map(|(a, b)| (a.into_owned(), b.into_owned()))
                .collect();
            match params.get("state") {
                Some(given) => {
                    if given.to_owned() != state {
                        return Err(anyhow!("State mismatch"));
                    }
                }
                None => return Err(anyhow!("State missing")),
            }

            let client = get_client()?;

            let token_url = Url::parse(token_url.as_str())?;

            let token_data = wilma::auth::oauth_authenticate(
                &client,
                protocol_url,
                token_url,
                client_id,
                code_verifier,
            )
            .await?;

            ipc::send_data(ipc::IPCMessage::TokenResponse {
                access_token: token_data.access_token,
                id_token: token_data.id_token,
            })
            .await?;
        }
        _ => unreachable!(),
    }

    Ok::<(), anyhow::Error>(())
}

fn run_interface(interface: impl Interface) -> Result<()> {
    interface.start(InterfaceOptions::new(get_client()?))
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).unwrap_or(&String::default()).as_str() == "__OAUTH" {
        debug!("Called with __OAUTH, assuming from protocol handler");
        Runtime::new().expect("Failed to create runtime").block_on(handle_oauth(args))?;
        return Ok(());
    }

    SimpleLogger::new()
        .with_level(DEFAULT_LOGGER_LEVEL)
        .env()
        .init()?;

    debug!("Registering wilma protocol handler");
    reg::register_wilma_handler()?;

    let in_terminal = unsafe {
        let parents = GetConsoleProcessList(&mut [0]);
        parents > 1
    };

    let rt = Runtime::new().expect("Failed to create runtime");
    let _guard = rt.enter();

    let res = if in_terminal {
        debug!("Starting CLI interface");
        run_interface(interfaces::CliInterface::new(rt.handle().clone()))
    } else {
        debug!("Starting Gui interface");
        run_interface(interfaces::GuiInterface::new(rt.handle().clone()))
    };

    debug!("Shutting down runtime");
    rt.shutdown_background();
    debug!("Unregistering wilma handler");
    reg::unregister_wilma_handler()?;
    
    res
}
