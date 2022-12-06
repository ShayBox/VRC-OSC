use common::{config::VrcConfig, error::VrcError};
use error_stack::{IntoReport, Result, ResultExt};
use std::net::UdpSocket;

fn main() -> Result<(), VrcError> {
    let config = VrcConfig::load()?;

    let osc = UdpSocket::bind(&config.osc.bind_addr)
        .into_report()
        .change_context(VrcError::Osc)?;

    let plugins = vrc_osc::load_plugins()?;
    loop {
        loop {
            let mut buf = [0u8; rosc::decoder::MTU];
            let Ok(size) = osc
                .recv(&mut buf)
                .into_report()
                .change_context(VrcError::Osc) 
            else {
                continue;
            };

            for (_plugin, state) in plugins.values() {
                let bind_addr = state.bind_addr().to_string();
                osc.send_to(&buf[..size], bind_addr)
                    .into_report()
                    .change_context(VrcError::Osc)?;
            }
        }
    }
}
