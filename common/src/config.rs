use crate::error::VrcError;
use error_stack::{IntoReport, Result, ResultExt};
use serde::{Deserialize, Serialize};
use std::{
    fs::OpenOptions,
    io::{Read, Write},
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
    pub enable: bool,
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
                enable: true,
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
    pub fn get_path() -> Result<PathBuf, VrcError> {
        let mut config_path = std::env::current_exe()
            .into_report()
            .change_context(VrcError::Io)?;

        config_path.set_file_name("config");
        config_path.set_extension("toml");

        Ok(config_path)
    }

    pub fn load() -> Result<Self, VrcError> {
        let config_path = Self::get_path()?;
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&config_path)
            .into_report()
            .change_context(VrcError::Io)
            .attach_printable(format!("Failed to open {:?}", &config_path))?;

        let mut content = String::new();
        file.read_to_string(&mut content)
            .into_report()
            .change_context(VrcError::Io)
            .attach_printable(format!("Failed to read {:?}", &config_path))?;

        match toml::from_str(&content) {
            Ok(config) => Ok(config),
            Err(_) => {
                let config = VrcConfig::default();
                let text = toml::to_string(&config)
                    .into_report()
                    .change_context(VrcError::Toml)?;

                file.write_all(text.as_bytes())
                    .into_report()
                    .change_context(VrcError::Io)?;

                Ok(config)
            }
        }
    }

    pub fn save(&mut self) -> Result<(), VrcError> {
        let config_path = Self::get_path()?;
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&config_path)
            .into_report()
            .change_context(VrcError::Io)
            .attach_printable(format!("Failed to open {:?}", &config_path))?;

        let text = toml::to_string(&self)
            .into_report()
            .change_context(VrcError::Toml)?;

        file.write_all(text.as_bytes())
            .into_report()
            .change_context(VrcError::Toml)?;

        Ok(())
    }
}
