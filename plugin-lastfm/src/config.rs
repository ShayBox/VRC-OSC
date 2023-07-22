use std::{
    fs::File,
    io::{Read, Seek, Write},
    path::PathBuf,
};

use anyhow::Result;
use dialoguer::Input;
use serde::{Deserialize, Serialize};

const CARGO_CRATE_NAME: &str = env!("CARGO_CRATE_NAME");
const LASTFM_API_KEY: &str = env!("LASTFM_API_KEY");
const LASTFM_USERNAME: &str = env!("LASTFM_USERNAME");

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LastFMConfig {
    pub api_key: String,
    pub username: String,
    pub format: String,
    pub send_once: bool,
    pub polling: u64,
}

impl Default for LastFMConfig {
    fn default() -> Self {
        Self {
            api_key: LASTFM_API_KEY.into(),
            username: LASTFM_USERNAME.into(),
            format: "📻 {song} - {artists}".into(),
            send_once: false,
            polling: 10,
        }
    }
}

impl LastFMConfig {
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
                let mut config = LastFMConfig::default();
                config.setup_wizard()?;

                let text = toml::to_string_pretty(&config)?;
                file.write_all(text.as_bytes())?;

                Ok(config)
            }
        }
    }

    pub fn setup_wizard(&mut self) -> Result<()> {
        let mut input = Input::new();

        println!("The LastFM plugin requires you to setup a scrobbler app or service");
        println!("https://www.last.fm/about/trackmymusic");

        let prompt = "LastFM Username: ";
        self.username = input.with_prompt(prompt).interact_text()?;

        Ok(())
    }
}
