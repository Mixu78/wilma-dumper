use tokio::runtime::Handle;

use super::{Interface, InterfaceOptions};

pub struct GuiInterface {
    rt: Handle,
}

impl Interface for GuiInterface {
    fn new(handle: Handle) -> Self {
        Self {
            rt: handle,
        }
    }

    fn start(self, options: InterfaceOptions) -> anyhow::Result<()> {
        println!("gui stuff");
        std::thread::sleep(std::time::Duration::from_secs(2));
        Ok(())
    }
}