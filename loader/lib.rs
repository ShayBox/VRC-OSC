use std::{
    ffi::OsStr,
    net::{SocketAddr, UdpSocket},
};

use anyhow::{Context, Result};
use async_ffi::FfiFuture;
use derive_config::DeriveTomlConfig;
use libloading::{Library, Symbol};
use path_absolutize::Absolutize;
use serde::{Deserialize, Serialize};
use tokio::runtime::Handle;
use walkdir::{DirEntry, WalkDir};

pub const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const CARGO_PKG_HOMEPAGE: &str = env!("CARGO_PKG_HOMEPAGE");

#[derive(Clone, Debug, DeriveTomlConfig, Deserialize, Serialize)]
pub struct Config {
    pub enabled:   Vec<String>,
    pub bind_addr: String,
    pub send_addr: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled:   Vec::default(),
            bind_addr: "0.0.0.0:9001".into(),
            send_addr: "127.0.0.1:9000".into(),
        }
    }
}

/// # Errors
///
/// Will return `Err` if couldn't get the current exe or dir path
pub fn get_plugin_names() -> Result<Vec<String>> {
    let current_exe = std::env::current_exe()?;
    let current_dir = current_exe.parent().context("This shouldn't be possible")?;

    let paths = WalkDir::new(current_dir)
        .max_depth(1)
        .into_iter()
        .filter_map(Result::ok)
        .map(DirEntry::into_path)
        .collect::<Vec<_>>();

    let mut libraries = Vec::new();
    for path in paths {
        let extension = path.extension().and_then(OsStr::to_str);
        let Some(extension) = extension else {
            continue; // No file extension
        };
        if !matches!(extension, "dll" | "dylib" | "so") {
            continue; // Not a dynamic library
        }

        let Some(filename) = path.file_name().and_then(OsStr::to_str) else {
            continue; // No file name
        };

        libraries.push(filename.to_owned());
    }

    Ok(libraries)
}

/// # Errors
///
/// Will return `Err` if couldn't get the current exe or dir path
pub fn get_plugin_path(path: String) -> Result<String> {
    // libloading doesn't support relative paths on Linux
    let current_exe = std::env::current_exe()?;
    let current_dir = current_exe.parent().context("This shouldn't be possible")?;
    let path = current_dir.join(path);
    let path = path.absolutize()?;

    Ok(path.to_str().context("None")?.to_owned())
}

/// # Errors
///
/// Will return `Err` if couldn't get the current exe or dir path
///
/// # Panics
///
/// Will panic if a plugin fails to load
pub fn load_plugins(names: Vec<String>, config: &Config) -> Result<Vec<SocketAddr>> {
    type LoadFn = fn(socket: UdpSocket);

    let mut addrs = Vec::new();
    for name in names {
        if !config.enabled.contains(&name) {
            continue; // Skip disabled plugins
        }

        let path = get_plugin_path(name)?;
        let socket = UdpSocket::bind("127.0.0.1:0")?; // Dynamic port
        let loader_addr = config.bind_addr.replace("0.0.0.0", "127.0.0.1");
        let plugin_addr = socket.local_addr()?;
        socket.connect(loader_addr)?;
        addrs.push(plugin_addr);

        tokio::spawn(async move {
            let plugin = unsafe { Library::new(path).expect("Failed to get the plugin") };
            let load_fn: Symbol<LoadFn> = unsafe {
                plugin
                    .get(b"load")
                    .expect("Failed to get the load function")
            };

            load_fn(socket);
        });
    }

    Ok(addrs)
}

pub type ChatMessage = (String, String);

/// # Errors
///
/// Will return `Err` if couldn't get the current exe or dir path
pub async fn chat_message(
    message: &ChatMessage,
    names: &[String],
    config: &Config,
) -> Result<ChatMessage> {
    type ChatFn =
        fn(chatbox: String, console: String, handle: Handle) -> FfiFuture<Result<ChatMessage>>;

    let mut message = message.clone();
    for name in names.iter().cloned() {
        if !config.enabled.contains(&name) {
            continue; // Skip disabled plugins
        }

        let path = get_plugin_path(name)?;
        let plugin = unsafe { Library::new(path.clone()) }?;
        let chat_fn = match unsafe { plugin.get(b"chat") } {
            Ok(chat_fn) => chat_fn as Symbol<ChatFn>,
            Err(_) => continue,
        };

        let (chatbox, console) = message.clone();
        if let Ok(new_message) = chat_fn(chatbox, console, Handle::current()).await {
            message = new_message;
        }

        // This appears to fix a random access violation?
        continue;
    }

    Ok(message)
}

/// # Errors
///
/// Will return `Err` if couldn't get the GitHub repository
pub fn check_for_updates() -> Result<bool> {
    let response = ureq::get(CARGO_PKG_HOMEPAGE).call()?;
    let Some(remote_version) = response.get_url().split('/').last() else {
        return Ok(false);
    };

    Ok(remote_version > CARGO_PKG_VERSION)
}
