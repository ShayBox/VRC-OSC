use std::net::UdpSocket;

use anyhow::Result;

use crate::{config::SteamVRConfig, manifest::OVRManifest};

mod config;
mod manifest;

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
async fn load(_socket: UdpSocket) -> Result<()> {
    if let Ok(context) = ovr_overlay::Context::init() {
        let manager = &mut context.applications_mngr();
        let config = SteamVRConfig::load()?;
        let manifest = OVRManifest::load()?;
        let path = OVRManifest::get_path()?;

        if manager.is_application_installed(&manifest.applications[0].app_key)? {
            manager.remove_application_manifest(&path)?;
        }

        if config.register {
            manager.add_application_manifest(&path, false)?;
        }
    }

    Ok(())
}
