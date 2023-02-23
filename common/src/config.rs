use std::{
    fs::OpenOptions,
    io::{Read, Seek, Write},
    path::PathBuf,
};

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VrcConfig {
    pub clock: ClockConfig,
    pub debug: DebugConfig,
    pub osc: OscConfig,
    pub spotify: SpotifyConfig,
    pub steamvr: SteamVRConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OscConfig {
    pub bind_addr: String,
    pub send_addr: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClockConfig {
    pub enable: bool,
    pub mode: bool,
    pub smooth: bool,
    pub polling: u64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DebugConfig {
    pub enable: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpotifyConfig {
    pub client_id: String,
    pub client_secret: String,
    pub enable_chatbox: bool,
    pub enable_control: bool,
    pub format: String,
    pub pkce: bool,
    pub polling: u64,
    pub redirect_uri: String,
    pub refresh_token: String,
    pub send_once: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SteamVRConfig {
    pub enable: bool,
    pub register: bool,
}

impl Default for OscConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:9001".into(),
            send_addr: "127.0.0.1:9000".into(),
        }
    }
}

impl Default for ClockConfig {
    fn default() -> Self {
        Self {
            enable: false,
            mode: false,
            smooth: false,
            polling: 1000,
        }
    }
}

impl Default for SpotifyConfig {
    fn default() -> Self {
        Self {
            client_id: env!("SPOTIFY_CLIENT").into(),
            client_secret: env!("SPOTIFY_SECRET").into(),
            format: "📻 {song} - {artists}".into(),
            enable_chatbox: false,
            enable_control: false,
            pkce: false,
            polling: 10,
            redirect_uri: env!("SPOTIFY_CALLBACK").into(),
            refresh_token: "".into(),
            send_once: false,
        }
    }
}

impl Default for SteamVRConfig {
    fn default() -> Self {
        Self {
            enable: false,
            register: true,
        }
    }
}

impl VrcConfig {
    pub fn get_path() -> Result<PathBuf> {
        let mut config_path = std::env::current_exe()?;

        config_path.set_file_name("config");
        config_path.set_extension("toml");

        Ok(config_path)
    }

    pub fn load() -> Result<Self> {
        let config_path = Self::get_path()?;
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(config_path)?;

        let mut content = String::new();
        file.read_to_string(&mut content)?;

        match toml::from_str(&content) {
            Ok(config) => Ok(config),
            Err(_) => {
                let config = VrcConfig::default();
                let text = toml::to_string(&config)?;

                file.rewind()?;
                file.write_all(text.as_bytes())?;

                Ok(config)
            }
        }
    }

    pub fn save(&mut self) -> Result<()> {
        let config_path = Self::get_path()?;
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(config_path)?;

        let text = toml::to_string(&self)?;
        file.write_all(text.as_bytes())?;

        Ok(())
    }
}
