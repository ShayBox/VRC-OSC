use std::net::UdpSocket;

use anyhow::Result;
use derive_config::DeriveTomlConfig;
use serde::{Deserialize, Serialize};

use crate::openvr::Manifest;

mod openvr;

#[derive(Clone, Debug, DeriveTomlConfig, Deserialize, Serialize)]
pub struct Config {
    pub register: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self { register: true }
    }
}

#[no_mangle]
#[allow(clippy::needless_pass_by_value)]
#[tokio::main(flavor = "current_thread")]
async extern "Rust" fn load(_socket: UdpSocket) -> Result<()> {
    if let Ok(context) = ovr_overlay::Context::init() {
        let manager = &mut context.applications_mngr();
        let config = Config::load()?;
        let manifest = Manifest::load()?;
        let path = Manifest::get_path()?;

        if manager.is_application_installed(&manifest.applications[0].app_key)? {
            manager.remove_application_manifest(&path)?;
        }

        if config.register {
            manager.add_application_manifest(&path, false)?;
        }
    }

    Ok(())
}
