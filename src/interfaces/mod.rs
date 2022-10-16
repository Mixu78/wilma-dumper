use anyhow::Result;
use reqwest::Client;
use tokio::runtime::Handle;

mod cli;
mod gui;

pub use cli::CliInterface;
pub use gui::GuiInterface;

pub struct InterfaceContext {
    client: Client,
}

impl InterfaceContext {
    pub fn new(client: Client) -> Self {
        Self {
            client,
        }
    }
}

pub trait Interface {
    fn new(rt: Handle) -> Self;
    fn start(self, options: InterfaceContext) -> Result<()>;
}