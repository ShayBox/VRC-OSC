use std::{
    collections::HashMap,
    ffi::OsStr,
    net::{SocketAddr, UdpSocket},
};

use anyhow::Result;
use libloading::{Library, Symbol};
use path_absolutize::Absolutize;
use walkdir::{DirEntry, WalkDir};

use crate::config::LoaderConfig;

pub mod config;

pub fn get_libraries() -> Result<HashMap<String, Library>> {
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

    let mut libraries = HashMap::new();
    for path in paths {
        // libloading doesn't support relative paths on Linux
        let path = path.absolutize()?;
        let Some(filename) = path.to_str() else {
            continue; // No file name
        };
        let Some(lib_name) = path.file_name().and_then(OsStr::to_str) else {
            continue; // No file name
        };

        let extension = path.extension().and_then(OsStr::to_str);
        let Some(extension) = extension else {
            continue; // No file extension
        };
        if !matches!(extension, "dll" | "dylib" | "so") {
            continue; // Not a dynamic library
        }

        unsafe {
            let library = Library::new(filename)?;
            libraries.insert(lib_name.to_string(), library);
        }
    }

    Ok(libraries)
}

pub fn load_plugins(
    libraries: HashMap<String, Library>,
    loader_config: &LoaderConfig,
) -> Result<Vec<SocketAddr>> {
    let mut addrs = Vec::new();
    for (filename, library) in libraries {
        if !loader_config.enabled.contains(&filename) {
            continue; // Skip disabled libraries
        }

        let plugin_socket = UdpSocket::bind("127.0.0.1:0")?; // Dynamic port
        let loader_addr = loader_config.bind_addr.replace("0.0.0.0", "127.0.0.1");
        plugin_socket.connect(loader_addr)?;

        let plugin_addr = plugin_socket.local_addr()?;
        addrs.push(plugin_addr);

        tokio::spawn(async move {
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
