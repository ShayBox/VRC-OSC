use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    fs::OpenOptions,
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VrcConfig {
    pub debug: DebugConfig,
    pub osc: OscConfig,
    pub spotify: SpotifyConfig,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OscConfig {
    pub bind_addr: String,
    pub send_addr: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DebugConfig {
    pub enable: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
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

impl Default for VrcConfig {
    fn default() -> Self {
        VrcConfig {
            osc: OscConfig {
                bind_addr: "0.0.0.0:9001".into(),
                send_addr: "127.0.0.1:9000".into(),
            },
            debug: DebugConfig { enable: false },
            spotify: SpotifyConfig {
                client_id: env!("SPOTIFY_CLIENT").into(),
                client_secret: env!("SPOTIFY_SECRET").into(),
                format: "📻 {song} - {artists}".into(),
                enable_chatbox: true,
                enable_control: true,
                pkce: false,
                polling: 10,
                redirect_uri: env!("SPOTIFY_CALLBACK").into(),
                refresh_token: "".into(),
                send_once: false,
            },
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
            .open(&config_path)?;

        let mut content = String::new();
        file.read_to_string(&mut content)?;

        match toml::from_str(&content) {
            Ok(config) => Ok(config),
            Err(_) => {
                let config = VrcConfig::default();
                let text = toml::to_string(&config)?;

                file.seek(SeekFrom::Start(0))?;
                file.write_all(text.as_bytes())?;

                Ok(config)
            }
        }
    }

    pub fn save(&mut self) -> Result<()> {
        let config_path = Self::get_path()?;
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&config_path)?;

        let text = toml::to_string(&self)?;
        file.write_all(text.as_bytes())?;

        Ok(())
    }
}
