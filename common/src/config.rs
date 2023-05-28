use std::{
    fs::OpenOptions,
    io::{Read, Seek, Write},
    path::PathBuf,
};

use anyhow::Result;
use dialoguer::{Confirm, Input};
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
        let mut config = Self {
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
        };

        let prompt = "Would you like to use the setup wizard? You can manually edit the config.toml file later.";
        if Confirm::new().with_prompt(prompt).interact().unwrap() {
            // SteamVR
            let prompt = "Would you like VRC-OSC to auto-start with SteamVR? This will open SteamVR once to register as a plugin.";
            config.steamvr.enable = Confirm::new().with_prompt(prompt).interact().unwrap();

            // Clock
            let prompt = "Would you like to use the Clock plugin? This requires your avatar use a compatible prefab.";
            config.clock.enable = Confirm::new().with_prompt(prompt).interact().unwrap();

            // LastFM Chatbox
            let prompt = "Would you like to use the LastFM plugin? This is the most versatile and easy to setup scrobbler.";
            if Confirm::new().with_prompt(prompt).interact().unwrap() {
                config.lastfm.enable = true;

                // LastFM Username
                let prompt = "\nWhat is your LastFM username?";
                config.lastfm.username = Input::new()
                    .with_prompt(prompt)
                    .default("".into())
                    .interact_text()
                    .unwrap();

                println!("Please setup one of the scrobbler apps or services if you haven't");
                println!("https://last.fm/about/trackmymusic");
            } else {
                // Spotify Chatbox
                let prompt = "Would you like to use the Spotify plugin? This requires manually setting up a Spotify Developer Application.";
                if Confirm::new().with_prompt(prompt).interact().unwrap() {
                    config.spotify.enable_chatbox = true;
                    println!("Please follow the guide at the link below to setup Spotify");
                    println!("https://github.com/ShayBox/VRC-OSC/tree/master/plugin-spotify#how-to-setup");
                }
            }

            config
        } else {
            config
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
