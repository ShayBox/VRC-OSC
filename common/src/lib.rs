use abi_stable::{
    declare_root_module_statics, library::RootModule, package_version_strings,
    sabi_types::VersionStrings, StableAbi,
};
use error_stack::Context;
use std::fmt;

#[derive(Debug)]
pub enum Error {
    IOError,
    LibraryError,
    None,
    SerdeError,
    TOMLError,
}
impl Context for Error {}
impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            Error::IOError => fmt.write_str("IOError"),
            Error::LibraryError => fmt.write_str("LibraryError"),
            Error::None => fmt.write_str("None"),
            Error::SerdeError => fmt.write_str("SerdeError"),
            Error::TOMLError => fmt.write_str("TOMLError"),
        }
    }
}

#[repr(C)]
#[derive(StableAbi)]
#[sabi(kind(Prefix))]
pub struct OSCMod {
    #[sabi(last_prefix_field)]
    pub new: extern "C" fn() -> (),
}

impl RootModule for OSCMod_Ref {
    const BASE_NAME: &'static str = "osc";
    const NAME: &'static str = "OSC";
    const VERSION_STRINGS: VersionStrings = package_version_strings!();

    declare_root_module_statics! {OSCMod_Ref}
}
