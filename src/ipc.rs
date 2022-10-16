use tokio::net::windows::named_pipe::{ClientOptions, ServerOptions};
use tokio::time;
use std::io;
use std::time::Duration;

use anyhow::Result;

use serde::{Deserialize, Serialize};
use serde_json::{from_slice, to_string};

use log::*;

const CAPACITY: usize = 1024 * 4; // token response is ~2.5kb
const PIPE_NAME: &str = r"\\.\pipe\wilma-dumper";

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
    let read = loop {
        match server.try_read(&mut buf)? {
            0 => time::sleep(Duration::from_millis(100)).await,
            n => break n,
        }
    };

    trace!("IPC received {read} bytes");

    Ok(from_slice::<IPCMessage>(&buf[0..read])?)
}

pub async fn send_data(data: IPCMessage) -> Result<()> {
    trace!("IPC attempting to send data");
    let client = loop {
        if let Ok(client) = ClientOptions::new().open(PIPE_NAME) {
            break client;
        }

        time::sleep(Duration::from_millis(50)).await;
    };

    let written = loop {
        client.writable().await?;
        match client.try_write(to_string(&data)?.as_bytes()) {
            Ok(n) => break n,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
            Err(e) => return Err(e.into()),
        };
    };

    trace!("IPC sent {written} bytes of data");

    Ok(())
}