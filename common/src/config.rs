use std::{
    fs::File,
    io::{Read, Seek, Write},
    path::PathBuf,
};

use anyhow::Result;
use dialoguer::{Confirm, Input};
#[cfg(feature = "dotenvy")]
use dotenvy_macro::dotenv;
use serde::{Deserialize, Serialize};

#[cfg(feature = "dotenvy")]
const LASTFM_API_KEY: &str = dotenv!("LASTFM_API_KEY");
#[cfg(not(feature = "dotenvy"))]
const LASTFM_API_KEY: &str = env!("LASTFM_API_KEY");

#[cfg(feature = "dotenvy")]
const LASTFM_USERNAME: &str = dotenv!("LASTFM_USERNAME");
#[cfg(not(feature = "dotenvy"))]
const LASTFM_USERNAME: &str = env!("LASTFM_USERNAME");

#[cfg(feature = "dotenvy")]
const SPOTIFY_CALLBACK: &str = dotenv!("SPOTIFY_CALLBACK");
#[cfg(not(feature = "dotenvy"))]
const SPOTIFY_CALLBACK: &str = env!("SPOTIFY_CALLBACK");

#[cfg(feature = "dotenvy")]
const SPOTIFY_CLIENT: &str = dotenv!("SPOTIFY_CLIENT");
#[cfg(not(feature = "dotenvy"))]
const SPOTIFY_CLIENT: &str = env!("SPOTIFY_CLIENT");

#[cfg(feature = "dotenvy")]
const SPOTIFY_SECRET: &str = dotenv!("SPOTIFY_SECRET");
#[cfg(not(feature = "dotenvy"))]
const SPOTIFY_SECRET: &str = env!("SPOTIFY_SECRET");

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
                api_key: LASTFM_API_KEY.into(),
                username: LASTFM_USERNAME.into(),
                format: "📻 {song} - {artists}".into(),
                enable: false,
                send_once: false,
                polling: 10,
            },
            spotify: Spotify {
                client_id: SPOTIFY_CLIENT.into(),
                client_secret: SPOTIFY_SECRET.into(),
                redirect_uri: SPOTIFY_CALLBACK.into(),
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
        let mut file = File::options()
            .read(true)
            .write(true)
            .create(true)
            .open(config_path)?;

        let mut content = String::new();
        file.read_to_string(&mut content)?;
        file.rewind()?;

        match toml::from_str(&content) {
            Ok(config) => Ok(config),
            Err(_) => {
                let mut config = VrcConfig::default();
                setup_wizard(&mut config)?;

                let text = toml::to_string(&config)?;
                file.write_all(text.as_bytes())?;

                Ok(config)
            }
        }
    }

    pub fn save(&mut self) -> Result<()> {
        let config_path = Self::get_path()?;
        let mut file = File::options()
            .write(true)
            .create(true)
            .truncate(true)
            .open(config_path)?;

        let text = toml::to_string(&self)?;
        file.write_all(text.as_bytes())?;

        Ok(())
    }
}

fn setup_wizard(config: &mut VrcConfig) -> Result<()> {
    let mut confirm = Confirm::new();

    // ! Setup Wizard
    let prompts = [
        "Would you like to use the setup wizard?",
        "You can manually edit the config.toml file later.",
    ];
    if !prompt(&mut confirm, prompts)? {
        return Ok(());
    }

    // ! SteamVR Plugin (plugin-steamvr)
    let prompts = [
        "Would you like VRC-OSC to auto-start with SteamVR?",
        "This will open SteamVR once to register as a plugin.",
    ];
    config.steamvr.enable = prompt(&mut confirm, prompts)?;

    // ! Clock Plugin (plugin-clock)
    let prompts = [
        "Would you like to use the Clock plugin?",
        "This requires your avatar use a compatible prefab.",
    ];
    config.clock.enable = prompt(&mut confirm, prompts)?;

    // ! LastFM Chatbox Plugin (plugin-lastfm)
    let prompts = [
        "Would you like to use the LastFM plugin?",
        "This is the most versatile and easy to setup scrobbler.",
    ];
    if prompt(&mut confirm, prompts)? {
        config.lastfm.enable = true;

        // ! LastFM Username
        let prompt = "What's your LastFM Username?";
        config.lastfm.username = Input::new()
            .with_prompt(prompt)
            .default("".into())
            .interact_text()
            .unwrap();

        let prompts = [
            "Please setup one of the scrobbler apps, extensions, or services.",
            "https://last.fm/about/trackmymusic",
        ];
        println!("{}", prompts.join("\n"));
    } else {
        // ! Spotify Chatbox Plugin (plugin-spotify)
        let prompts = [
            "Would you like to use the Spotify plugin?",
            "This requires manually setting up a Spotify Developer Application.",
        ];
        if prompt(&mut confirm, prompts)? {
            config.spotify.enable_chatbox = true;

            let prompts = [
                "Please follow the guide at the link below to setup Spotify",
                "https://github.com/ShayBox/VRC-OSC/tree/master/plugin-spotify#how-to-setup",
            ];
            println!("{}", prompts.join("\n"));
        }
    }

    Ok(())
}

fn prompt(confirm: &mut Confirm, prompts: [&str; 2]) -> Result<bool> {
    Ok(confirm.with_prompt(prompts.join(" ")).interact().unwrap())
}
