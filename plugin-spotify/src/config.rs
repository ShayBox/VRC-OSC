use std::{
    fs::File,
    io::{Read, Seek, Write},
    path::PathBuf,
};

use anyhow::Result;
use dialoguer::{Confirm, Input};
use serde::{Deserialize, Serialize};

const CARGO_CRATE_NAME: &str = env!("CARGO_CRATE_NAME");
const SPOTIFY_CLIENT: &str = env!("SPOTIFY_CLIENT");
const SPOTIFY_SECRET: &str = env!("SPOTIFY_SECRET");
const SPOTIFY_CALLBACK: &str = env!("SPOTIFY_CALLBACK");

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SpotifyConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub format: String,
    pub refresh_token: String,
    pub enable_chatbox: bool,
    pub enable_control: bool,
    pub pkce: bool,
    pub send_once: bool,
    pub send_lyrics: bool,
    pub polling: u64,
}

impl Default for SpotifyConfig {
    fn default() -> Self {
        Self {
            client_id: SPOTIFY_CLIENT.into(),
            client_secret: SPOTIFY_SECRET.into(),
            redirect_uri: SPOTIFY_CALLBACK.into(),
            format: "📻 {song} - {artists}".into(),
            refresh_token: String::new(),
            enable_chatbox: false,
            enable_control: false,
            pkce: false,
            send_once: true,
            send_lyrics: true,
            polling: 1,
        }
    }
}

impl SpotifyConfig {
    pub fn get_path() -> Result<PathBuf> {
        let mut path = std::env::current_exe()?;
        path.set_file_name(CARGO_CRATE_NAME);
        path.set_extension("toml");

        Ok(path)
    }

    pub fn load() -> Result<Self> {
        let path = Self::get_path()?;
        let mut file = File::options()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        let mut text = String::new();
        file.read_to_string(&mut text)?;
        file.rewind()?;

        match toml::from_str(&text) {
            Ok(config) => Ok(config),
            Err(_) => {
                let mut config = SpotifyConfig::default();
                config.setup_wizard()?;

                let text = toml::to_string_pretty(&config)?;
                file.write_all(text.as_bytes())?;

                Ok(config)
            }
        }
    }

    pub fn setup_wizard(&mut self) -> Result<()> {
        let prompt = "Would you like to enable Spotify Chatbox?";
        self.enable_chatbox = Confirm::new().with_prompt(prompt).interact()?;

        let prompt = "Would you like to enable Spotify Controls? (Requires Spotify Premium)";
        self.enable_control = Confirm::new().with_prompt(prompt).interact()?;

        if self.enable_chatbox || self.enable_control {
            println!("The Spotify plugin requires you to create a Spotify Developer Application");
            println!("https://github.com/ShayBox/VRC-OSC/tree/master/plugin-spotify#how-to-setup");
            println!("https://developer.spotify.com/dashboard");

            let prompt = "Spotify Client ID: ";
            self.client_id = Input::new()
                .with_prompt(prompt)
                .default(self.client_id.to_owned())
                .interact_text()?;

            let prompt = "Spotify Client secret: ";
            self.client_secret = Input::new()
                .with_prompt(prompt)
                .default(self.client_secret.to_owned())
                .interact_text()?;

            let prompt = "Spotify Redirect URI: ";
            self.redirect_uri = Input::new()
                .with_prompt(prompt)
                .default(self.redirect_uri.to_owned())
                .interact_text()?;
        }

        Ok(())
    }

    pub fn save(&mut self) -> Result<()> {
        let path = Self::get_path()?;
        let mut file = File::options()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;

        let content = toml::to_string_pretty(&self)?;
        file.write_all(content.as_bytes())?;

        Ok(())
    }
}
