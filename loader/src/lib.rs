use std::{
    ffi::OsStr,
    net::{SocketAddr, UdpSocket},
};

use anyhow::Result;
use libloading::{Library, Symbol};
use path_absolutize::Absolutize;
use walkdir::{DirEntry, WalkDir};

use crate::config::LoaderConfig;

pub mod config;

pub const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const CARGO_PKG_HOMEPAGE: &str = env!("CARGO_PKG_HOMEPAGE");

pub fn get_plugin_names() -> Result<Vec<String>> {
    let current_exe = std::env::current_exe()?;
    let Some(current_dir) = current_exe.parent() else {
        panic!("This shouldn't be possible");
    };

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

pub fn load_plugins(
    plugin_names: Vec<String>,
    loader_config: &LoaderConfig,
) -> Result<Vec<SocketAddr>> {
    let current_exe = std::env::current_exe()?;
    let Some(current_dir) = current_exe.parent() else {
        panic!("This shouldn't be possible");
    };

    let mut addrs = Vec::new();
    for filename in plugin_names {
        if !loader_config.enabled.contains(&filename) {
            continue; // Skip disabled libraries
        }

        // libloading doesn't support relative paths on Linux
        let path = current_dir.join(filename);
        let path = path.absolutize()?;
        let Some(filename) = path.to_str() else {
            continue; // No file name
        };

        let plugin_socket = UdpSocket::bind("127.0.0.1:0")?; // Dynamic port
        let loader_addr = loader_config.bind_addr.replace("0.0.0.0", "127.0.0.1");
        plugin_socket.connect(loader_addr)?;

        let plugin_addr = plugin_socket.local_addr()?;
        addrs.push(plugin_addr);

        let filename = filename.to_owned();
        tokio::spawn(async move {
            let library = unsafe { Library::new(filename).expect("Failed to get the library") };
            let load_fn: Symbol<fn(socket: UdpSocket)> = unsafe {
                library
                    .get(b"load")
                    .expect("Failed to get the load function")
            };

            load_fn(plugin_socket);
        });
    }

    Ok(addrs)
}

pub async fn check_for_updates() -> Result<bool> {
    let response = reqwest::get(CARGO_PKG_HOMEPAGE).await?;
    let url = response.url();
    let path = url.path();
    let Some(remote_version) = path.split('/').last() else {
        return Ok(false)
    };

    Ok(remote_version > CARGO_PKG_VERSION)
}
