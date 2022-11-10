use common::{Error, OSCMod_Ref};
use error_stack::{bail, IntoReport, Result, ResultExt};
use std::{ffi::OsStr, result::Result as StdResult};

pub fn load_plugins() -> Result<(), Error> {
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

    for entry in entries {
        let path = entry.path();
        let extension = path.extension().and_then(OsStr::to_str);
        let Some(extension) = extension else {
            continue;
        };
        if !matches!(extension, "dll" | "so") {
            continue;
        }

        abi_stable::library::lib_header_from_path(path.as_path())
            .and_then(|x| x.init_root_module::<OSCMod_Ref>())
            .into_report()
            .change_context(Error::LibraryError)?
            .new()();
    }

    Ok(())
}
