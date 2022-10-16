use std::collections::HashMap;

use reqwest::{Client, Url};
use tokio::runtime::Runtime;

use anyhow::{anyhow, Result};

use windows::Win32::System::Console::GetConsoleProcessList;

use flexi_logger::{Duplicate, FileSpec, Logger};
use log::*;

use interfaces::{Interface, InterfaceContext};

mod dump;
mod interfaces;
mod ipc;
mod reg;
mod wilma;

const DEFAULT_LOGGER_LEVEL: LevelFilter = if cfg!(debug_assertions) {
    LevelFilter::Debug
} else {
    LevelFilter::Info
};

const DEFAULT_LOGGER_LEVEL_STR: &str = match DEFAULT_LOGGER_LEVEL {
    LevelFilter::Trace => "wilma_dumper=trace",
    LevelFilter::Debug => "wilma_dumper=debug",
    LevelFilter::Info => "wilma_dumper=info",
    LevelFilter::Warn => "wilma_dumper=warn",
    LevelFilter::Error => "wilma_dumper=error",
    LevelFilter::Off => "off",
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

fn init_logger(discriminant: &str, level: Option<&str>) -> Result<flexi_logger::LoggerHandle> {
    let exe_path = std::env::current_exe()?;
    let path = exe_path
        .parent()
        .unwrap()
        .to_str()
        .ok_or_else(|| anyhow!("Path contains non-unicode characters"))?
        .replace(r"\\?\", "");

    let handle = Logger::try_with_env_or_str(level.unwrap_or(DEFAULT_LOGGER_LEVEL_STR))
        .unwrap()
        .format_for_files(flexi_logger::detailed_format)
        .format_for_stderr(flexi_logger::colored_detailed_format)
        .format_for_stdout(flexi_logger::colored_detailed_format)
        .log_to_file(
            FileSpec::default()
                .directory(path)
                .suppress_timestamp()
                .discriminant(discriminant),
        )
        .duplicate_to_stderr(Duplicate::Error)
        .duplicate_to_stdout(Duplicate::All)
        .start()?;

    Ok(handle)
}

//TODO move elsewhere? Maybe wilma::auth?
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
                    if *given != state {
                        return Err(anyhow!("State mismatch"));
                    }
                }
                None => return Err(anyhow!("State missing")),
            }

            let client = get_client()?;

            let token_url = Url::parse(token_url.as_str())?;

            trace!("Starting token request");
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

    Ok(())
}

fn run_interface(interface: impl Interface) -> Result<()> {
    interface.start(InterfaceContext::new(get_client()?))
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).unwrap_or(&String::default()).as_str() == "__OAUTH" {
        std::env::set_var("RUST_BACKTRACE", "1"); //windows decides to ignore environment variables apparently
        init_logger("oauth", Some("trace"))?;

        debug!("Called with __OAUTH, assuming from protocol handler");
        let res = Runtime::new()
            .expect("Failed to create runtime")
            .block_on(handle_oauth(args));

        if res.is_err() {
            error!("Oauth handler failed: {:?}", res);
        }

        return res;
    }

    let has_parent = unsafe {
        let parents = GetConsoleProcessList(&mut [0]);
        parents > 1
    };

    let logger_discr = if has_parent { "no-terminal" } else { "" };
    init_logger(logger_discr, None)?;

    debug!("Registering protocol handler");
    reg::register_wilma_handler()?;

    let rt = Runtime::new().expect("Failed to create runtime");
    let _guard = rt.enter();

    let res = if has_parent && std::env::var("FORCE_GUI").is_err() {
        debug!("Starting CLI interface");
        run_interface(interfaces::CliInterface::new(rt.handle().clone()))
    } else {
        debug!("Starting Gui interface");
        run_interface(interfaces::GuiInterface::new(rt.handle().clone()))
    };

    debug!("Shutting down runtime");
    rt.shutdown_background();
    debug!("Unregistering protocol handler");
    reg::unregister_wilma_handler()?;

    res
}
