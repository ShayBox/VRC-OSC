use anyhow::{bail, Result};
use common::{OSCMod_Ref, StateBox};
use std::{collections::HashMap, ffi::OsStr, result::Result as StdResult};

pub fn load_plugins() -> Result<HashMap<String, (OSCMod_Ref, StateBox)>> {
    let current_exe = std::env::current_exe()?;
    let current_dir = current_exe.parent();
    let Some(current_dir) = current_dir else {
        bail!("None");
    };

    let entries = current_dir.read_dir()?.filter_map(StdResult::ok);

    let mut plugins = HashMap::new();
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
            .and_then(|x| x.init_root_module::<OSCMod_Ref>())?;

        let Some(new_fn) = plugin.new() else {
            continue;
        };
        let state = new_fn();

        plugins.insert(file_name.into(), (plugin, state));
    }

    Ok(plugins)
}
