use tokio::net::windows::named_pipe::{ClientOptions, ServerOptions};
use tokio::time;
use std::time::Duration;

use anyhow::Result;

use serde::{Deserialize, Serialize};
use serde_json::{from_slice, to_string};

use log::*;

const CAPACITY: usize = 1024 * 10;
const PIPE_NAME: &str = r"\\.\pipe\wilma-stuff";

#[derive(Deserialize, Serialize)]
pub enum IPCMessage {
    TokenRequest {
        state: String,
        client_id: String,
        token_endpoint: String,
        code_verifier: String,
    },

    TokenResponse {
        access_token: String,
        id_token: String,
    }
}

pub async fn receive_data() -> Result<IPCMessage> {
    trace!("IPC attempting to receive data");
    let server = ServerOptions::new().create(PIPE_NAME)?;

    server.connect().await?;
    server.readable().await?;

    let mut buf: [u8; CAPACITY] = [0; CAPACITY];
    server.try_read(&mut buf)?;

    trace!("IPC received data");

    Ok(from_slice::<IPCMessage>(&mut buf)?)
}

pub async fn send_data(data: IPCMessage) -> Result<()> {
    trace!("IPC attempting to send data");
    let client = loop {
        match ClientOptions::new().open(PIPE_NAME) {
            Ok(client) => break client,
            Err(_) => (),
        }

        time::sleep(Duration::from_millis(50)).await;
    };
    trace!("IPC client connected");

    client.writable().await?;
    client.try_write(to_string(&data)?.as_bytes())?;

    trace!("IPC sent data");

    Ok(())
}