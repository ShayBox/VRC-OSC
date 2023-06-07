use std::{
    fs::File,
    io::{Read, Seek, Write},
    path::PathBuf,
    process::exit,
};

use anyhow::Result;
use dialoguer::Confirm;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LoaderConfig {
    pub enabled: Vec<String>,
    pub bind_addr: String,
    pub send_addr: String,
}

impl Default for LoaderConfig {
    fn default() -> Self {
        Self {
            enabled: Default::default(),
            bind_addr: "0.0.0.0:9001".into(),
            send_addr: "127.0.0.1:9000".into(),
        }
    }
}

impl LoaderConfig {
    pub fn get_path() -> Result<PathBuf> {
        let mut path = std::env::current_exe()?;
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
                let mut config = LoaderConfig::default();
                config.setup_wizard()?;

                let text = toml::to_string_pretty(&config)?;
                file.write_all(text.as_bytes())?;

                Ok(config)
            }
        }
    }

    pub fn setup_wizard(&mut self) -> Result<()> {
        let mut filenames = crate::get_plugin_names()?;
        filenames.sort();

        for filename in filenames {
            let prompt = format!("Would you like to enable the {filename} plugin");
            if Confirm::new().with_prompt(prompt).interact()? {
                self.enabled.push(filename.to_owned());
            }
        }

        if self.enabled.is_empty() {
            println!("You must enable at least one plugin");
            exit(1);
        }

        Ok(())
    }
}
