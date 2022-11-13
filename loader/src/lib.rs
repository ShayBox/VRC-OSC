use common::{Error, OSCMod_Ref};
use error_stack::{bail, IntoReport, Result, ResultExt};
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    fs::OpenOptions,
    io::{Read, Write},
    result::Result as StdResult,
};

const CONFIG_PATH: &str = "config.toml";

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub bind_addr: String,
    pub osc_addr: String,
    pub verbose: bool,
}
impl Default for Config {
    fn default() -> Config {
        Config {
            bind_addr: "0.0.0.0:9001".into(),
            osc_addr: "127.0.0.1:9000".into(),
            verbose: false,
        }
    }
}

pub fn load_config() -> Result<Config, Error> {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(CONFIG_PATH)
        .into_report()
        .change_context(Error::IOError)
        .attach_printable(format!("Failed to open {CONFIG_PATH}"))?;
    let mut content = String::new();
    file.read_to_string(&mut content)
        .into_report()
        .change_context(Error::IOError)
        .attach_printable(format!("Failed to read {CONFIG_PATH}"))?;
    match toml::from_str(&content) {
        Ok(config) => Ok(config),
        Err(_) => {
            let config = Config::default();
            let text = toml::to_string(&config)
                .into_report()
                .change_context(Error::TOMLError)?;
            file.write_all(text.as_bytes())
                .into_report()
                .change_context(Error::IOError)?;
            Ok(config)
        }
    }
}

pub fn load_plugins() -> Result<Vec<OSCMod_Ref>, Error> {
    let current_exe = std::env::current_exe()
        .into_report()
        .change_context(Error::IOError)?;

    let current_dir = current_exe.parent();
    let Some(current_dir) = current_dir else {
        bail!(Error::None);
    };

    let entries = current_dir
        .read_dir()
        .into_report()
        .change_context(Error::IOError)
        .attach_printable(format!("Failed to read {}", current_dir.display()))?
        .filter_map(StdResult::ok);

    let mut plugins = vec![];
    for entry in entries {
        let path = entry.path();
        let extension = path.extension().and_then(OsStr::to_str);
        let Some(extension) = extension else {
            continue;
        };
        if !matches!(extension, "dll" | "dylib" | "so") {
            continue;
        }

        let file_name = path.file_name().and_then(OsStr::to_str);
        let Some(file_name) = file_name else {
            continue;
        };

        println!("Loading {file_name}");
        let plugin = abi_stable::library::lib_header_from_path(path.as_path())
            .and_then(|x| x.init_root_module::<OSCMod_Ref>())
            .into_report()
            .change_context(Error::LibraryError)?;

        plugins.push(plugin);
    }

    Ok(plugins)
}
