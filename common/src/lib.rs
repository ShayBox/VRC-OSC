#![allow(repr_transparent_external_private_fields)]

use abi_stable::{
    declare_root_module_statics,
    library::RootModule,
    package_version_strings,
    sabi_types::VersionStrings,
    std_types::{RBox, RString},
    StableAbi,
};

pub mod config;

#[abi_stable::sabi_trait]
pub trait CommonState: Debug {
    fn bind_addr(&self) -> RString;
    fn is_enabled(&self) -> bool;
}

#[derive(Clone, Debug)]
pub struct OscState {
    pub bind_addr: RString,
    pub send_messages: bool,
}

impl CommonState for OscState {
    fn bind_addr(&self) -> RString {
        self.bind_addr.to_owned()
    }

    fn is_enabled(&self) -> bool {
        self.send_messages
    }
}

pub type StateBox = CommonState_TO<'static, RBox<()>>;

#[repr(C)]
#[derive(StableAbi)]
#[sabi(kind(Prefix))]
pub struct OSCMod {
    pub new: extern "C" fn() -> StateBox,
}

impl RootModule for OSCMod_Ref {
    const BASE_NAME: &'static str = "osc";
    const NAME: &'static str = "OSC";
    const VERSION_STRINGS: VersionStrings = package_version_strings!();

    declare_root_module_statics! {OSCMod_Ref}
}
