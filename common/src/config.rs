use std::{
    fs::OpenOptions,
    io::{Read, Seek, Write},
    path::PathBuf,
};

use anyhow::Result;
use serde::{Deserialize, Serialize};

structstruck::strike! {
    #[strikethrough[derive(Debug, Clone, Serialize, Deserialize)]]
    pub struct VrcConfig {
        pub osc: struct {
            pub bind_addr: String,
            pub send_addr: String,
        },
        pub clock: struct {
            pub enable: bool,
            pub mode: bool,
            pub smooth: bool,
            pub polling: u64,
        },
        pub debug: struct {
            pub enable: bool,
        },
        pub lastfm: struct {
            pub api_key: String,
            pub username: String,
            pub format: String,
            pub enable: bool,
            pub send_once: bool,
            pub polling: u64,
        },
        pub spotify: struct {
            pub client_id: String,
            pub client_secret: String,
            pub redirect_uri: String,
            pub format: String,
            pub refresh_token: String,
            pub enable_chatbox: bool,
            pub enable_control: bool,
            pub pkce: bool,
            pub send_once: bool,
            pub polling: u64,
        },
        pub steamvr: struct {
            pub enable: bool,
            pub register: bool,
        },
    }
}

impl Default for VrcConfig {
    fn default() -> Self {
        Self {
            osc: Osc {
                bind_addr: "0.0.0.0:9001".into(),
                send_addr: "127.0.0.1:9000".into(),
            },
            clock: Clock {
                enable: false,
                mode: false,
                smooth: false,
                polling: 1000,
            },
            debug: Debug { enable: false },
            lastfm: Lastfm {
                api_key: env!("LASTFM_API_KEY").into(),
                username: env!("LASTFM_USERNAME").into(),
                format: "📻 {song} - {artists}".into(),
                enable: false,
                send_once: false,
                polling: 10,
            },
            spotify: Spotify {
                client_id: env!("SPOTIFY_CLIENT").into(),
                client_secret: env!("SPOTIFY_SECRET").into(),
                redirect_uri: env!("SPOTIFY_CALLBACK").into(),
                format: "📻 {song} - {artists}".into(),
                refresh_token: "".into(),
                enable_chatbox: false,
                enable_control: false,
                pkce: false,
                send_once: false,
                polling: 10,
            },
            steamvr: Steamvr {
                enable: false,
                register: true,
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
