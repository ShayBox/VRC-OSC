use std::{net::UdpSocket, thread::Builder};

use abi_stable::{
    export_root_module,
    prefix_type::PrefixTypeTrait,
    sabi_extern_fn,
    sabi_trait::TD_Opaque,
};
use anyhow::Result;
use common::{config::VrcConfig, CommonState_TO, OSCMod, OSCMod_Ref, OscState, StateBox};

use crate::manifest::Manifest;

mod manifest;

#[export_root_module]
fn instantiate_root_module() -> OSCMod_Ref {
    OSCMod { new }.leak_into_prefix()
}

#[sabi_extern_fn]
pub fn new() -> StateBox {
    let config = VrcConfig::load().expect("Failed to load config");
    let _osc = UdpSocket::bind("127.0.0.1:0").expect("Failed to bind socket");
    let local_addr = _osc.local_addr().expect("Failed to parse local_addr");

    let state = OscState {
        bind_addr: local_addr.to_string().into(),
        send_messages: false,
    };

    if config.steamvr.enable {
        println!("SteamVR is enabled");
        Builder::new()
            .name("SteamVR Plugin".to_string())
            .spawn(move || thread_steamvr(config, &_osc).expect("thread_steamvr"))
            .expect("SteamVR Plugin failed");
    } else {
        println!("SteamVR is disabled");
    }

    CommonState_TO::from_value(state, TD_Opaque)
}

fn thread_steamvr(config: VrcConfig, _osc: &UdpSocket) -> Result<()> {
    let path = Manifest::get_path()?;
    let manifest = Manifest::load()?;

    let context = ovr_overlay::Context::init()?;
    let mngr = &mut context.applications_mngr();

    if mngr.is_application_installed(&manifest.applications[0].app_key)? {
        mngr.remove_application_manifest(&path)?;
    }

    if config.steamvr.register {
        mngr.add_application_manifest(&path, false)?;
    } else {
        println!("Removed from SteamVR");
    }

    Ok(())
}
