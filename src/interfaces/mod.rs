use anyhow::Result;
use reqwest::Client;
use tokio::runtime::Handle;

mod cli;
mod gui;

pub use cli::CliInterface;
pub use gui::GuiInterface;

pub struct InterfaceOptions {
    client: Client,
}

impl InterfaceOptions {
    pub fn new(client: Client) -> Self {
        Self {
            client,
        }
    }
}

pub trait Interface {
    fn new(rt: Handle) -> Self;
    fn start(self, options: InterfaceOptions) -> Result<()>;
}