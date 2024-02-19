#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]

use std::{net::UdpSocket, time::Duration};

use anyhow::Result;
use derive_config::DeriveTomlConfig;
use loader::{ChatMessage, Config as LoaderConfig};
use rosc::{OscMessage, OscPacket, OscType};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, DeriveTomlConfig, Deserialize, Serialize)]
pub struct Config {
    pub message:   ChatMessage,
    pub send_once: bool,
    pub polling:   u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            message:   (
                "📻 {song} - {artists}".into(),
                "📻 {song} - {artists}".into(),
            ),
            send_once: false,
            polling:   1,
        }
    }
}

#[no_mangle]
#[allow(clippy::needless_pass_by_value)]
#[tokio::main(flavor = "current_thread")]
async extern "Rust" fn load(socket: UdpSocket) -> Result<()> {
    let config = Config::load().unwrap_or_default();
    let plugin_names = loader::get_plugin_names()?;
    let loader_config = LoaderConfig::load()?;

    config.save()?;

    let mut previous_message: (String, String) = Default::default();
    loop {
        tokio::time::sleep(Duration::from_secs(config.polling)).await;

        let message = loader::chat_message(&config.message, &plugin_names, &loader_config).await?;
        if message != previous_message {
            println!("{}", message.1);
        } else if config.send_once {
            continue;
        }

        previous_message = message.clone();

        let packet = OscPacket::Message(OscMessage {
            addr: "/chatbox/input".into(),
            args: vec![OscType::String(message.0), OscType::Bool(true)],
        });

        let msg_buf = rosc::encoder::encode(&packet)?;
        socket.send(&msg_buf)?;
    }
}
