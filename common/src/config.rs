use crate::error::VRCError;
use error_stack::{IntoReport, Result, ResultExt};
use serde::{Deserialize, Serialize};
use std::{
    fs::OpenOptions,
    io::{Read, Write},
};

const CONFIG_PATH: &str = "config.toml";

#[derive(Debug, Serialize, Deserialize)]
pub struct VrcConfig {
    pub debug: DebugConfig,
    pub osc: OscConfig,
    pub spotify: SpotifyConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OscConfig {
    pub bind_addr: String,
    pub send_addr: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DebugConfig {
    pub enable: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpotifyConfig {
    pub callback_uri: String,
    pub client_id: String,
    pub enable: bool,
    pub polling: u64,
    pub send_once: bool,
}

impl Default for VrcConfig {
    fn default() -> Self {
        VrcConfig {
            osc: OscConfig {
                bind_addr: "0.0.0.0:9001".into(),
                send_addr: "127.0.0.1:9000".into(),
            },
            debug: DebugConfig { enable: false },
            spotify: SpotifyConfig {
                callback_uri: env!("SPOTIFY_CALLBACK").to_string(),
                client_id: env!("SPOTIFY_CLIENT").to_string(),
                enable: true,
                polling: 5,
                send_once: true,
            },
        }
    }
}

impl VrcConfig {
    pub fn load() -> Result<Self, VRCError> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(CONFIG_PATH)
            .into_report()
            .change_context(VRCError::IOError)
            .attach_printable(format!("Failed to open {CONFIG_PATH}"))?;
        let mut content = String::new();
        file.read_to_string(&mut content)
            .into_report()
            .change_context(VRCError::IOError)
            .attach_printable(format!("Failed to read {CONFIG_PATH}"))?;
        match toml::from_str(&content) {
            Ok(config) => Ok(config),
            Err(_) => {
                let config = VrcConfig::default();
                let text = toml::to_string(&config)
                    .into_report()
                    .change_context(VRCError::TOMLError)?;
                file.write_all(text.as_bytes())
                    .into_report()
                    .change_context(VRCError::IOError)?;
                Ok(config)
            }
        }
    }
}
