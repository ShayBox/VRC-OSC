use std::{
    collections::HashMap,
    fs::OpenOptions,
    io::{BufReader, BufWriter},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub source: String,
    pub applications: Vec<Application>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Application {
    pub app_key: String,
    pub launch_type: String,
    pub binary_path_windows: String,
    pub is_dashboard_overlay: bool,
    pub strings: HashMap<String, LocaleString>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocaleString {
    pub name: String,
    pub description: String,
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            source: "builtin".into(),
            applications: vec![Application::default()],
        }
    }
}

impl Default for Application {
    fn default() -> Self {
        Self {
            app_key: "com.shaybox.vrc-osc".into(),
            launch_type: "binary".into(),
            binary_path_windows: "vrc-osc.exe".into(),
            is_dashboard_overlay: true,
            strings: HashMap::from([(
                "en_us".into(),
                LocaleString {
                    name: "VRC-OSC".into(),
                    description: "VRChat OSC Overlay".into(),
                },
            )]),
        }
    }
}

impl Manifest {
    pub fn get_path() -> anyhow::Result<PathBuf> {
        let mut manifest_path = std::env::current_exe()?;

        manifest_path.set_file_name("vrc-osc");
        manifest_path.set_extension("vrmanifest");

        Ok(manifest_path)
    }

    pub fn load() -> anyhow::Result<Self> {
        let manifest_path = Self::get_path()?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(manifest_path)?;

        let reader = BufReader::new(&file);
        match serde_json::from_reader(reader) {
            Ok(config) => Ok(config),
            Err(_) => {
                let config = Manifest::default();
                let writer = BufWriter::new(&file);
                serde_json::to_writer_pretty(writer, &config)?;

                Ok(config)
            }
        }
    }
}
