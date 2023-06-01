use std::{collections::HashMap, ffi::OsStr, net::UdpSocket, thread::Builder};

use anyhow::Result;
use libloading::{Library, Symbol};
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
        let Some(filename) = path.file_name().and_then(OsStr::to_str) else {
            continue; // This shouldn't be possible
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
            libraries.insert(filename.to_string(), library);
        }
    }

    Ok(libraries)
}

pub fn load_plugins(
    libraries: HashMap<String, Library>,
    loader_config: &LoaderConfig,
) -> Result<Vec<String>> {
    let mut local_addrs = Vec::new();
    for (filename, library) in libraries {
        if !loader_config.enabled.contains(&filename) {
            continue; // Skip disabled libraries
        }

        let socket = UdpSocket::bind("127.0.0.1:0")?;
        socket.connect(&loader_config.send_addr)?;

        let local_addr = socket.local_addr()?;
        local_addrs.push(local_addr.to_string());

        Builder::new().name(filename).spawn(move || unsafe {
            let main_fn: Symbol<fn(socket: UdpSocket) -> Result<()>> = library
                .get(b"main")
                .expect("Failed to get the main function");
            main_fn(socket).expect("Failed to call the main function")
        })?;
    }

    Ok(local_addrs)
}
