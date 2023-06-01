use std::net::UdpSocket;

use anyhow::Result;

use crate::{config::SteamVRConfig, manifest::OVRManifest};

mod config;
mod manifest;

#[no_mangle]
fn main(_socket: UdpSocket) -> Result<()> {
    let config = SteamVRConfig::load()?;
    let manifest = OVRManifest::load()?;
    let path = OVRManifest::get_path()?;

    let context = ovr_overlay::Context::init()?;
    let mngr = &mut context.applications_mngr();

    if mngr.is_application_installed(&manifest.applications[0].app_key)? {
        mngr.remove_application_manifest(&path)?;
    }

    if config.register {
        mngr.add_application_manifest(&path, false)?;
    }

    Ok(())
}
