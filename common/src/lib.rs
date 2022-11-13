#![allow(repr_transparent_external_private_fields)]

use abi_stable::{
    declare_root_module_statics,
    library::RootModule,
    package_version_strings,
    sabi_types::VersionStrings,
    std_types::{RBox, RSliceMut},
    StableAbi,
};

pub mod config;
pub mod error;

#[abi_stable::sabi_trait]
pub trait State: Debug {
    fn is_enabled(&self) -> bool;
}
pub type StateBox = State_TO<'static, RBox<()>>;

#[repr(C)]
#[derive(StableAbi)]
#[sabi(kind(Prefix))]
pub struct OSCMod {
    pub new: extern "C" fn() -> StateBox,

    #[sabi(last_prefix_field)]
    pub message: extern "C" fn(state: &StateBox, size: usize, buf: RSliceMut<u8>) -> (),
}

impl RootModule for OSCMod_Ref {
    const BASE_NAME: &'static str = "osc";
    const NAME: &'static str = "OSC";
    const VERSION_STRINGS: VersionStrings = package_version_strings!();

    declare_root_module_statics! {OSCMod_Ref}
}
