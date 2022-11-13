use abi_stable::std_types::RSliceMut;
use common::{config::VrcConfig, error::VRCError};
use error_stack::{IntoReport, Result, ResultExt};
use std::net::UdpSocket;

fn main() -> Result<(), VRCError> {
    let config = VrcConfig::load()?;
    let osc = UdpSocket::bind(&config.osc.bind_addr)
        .into_report()
        .change_context(VRCError::OscError)?;
    let plugins = vrc_osc::load_plugins()?;
    loop {
        let mut buf = [0u8; rosc::decoder::MTU];
        loop {
            let (size, _addr) = osc
                .recv_from(&mut buf)
                .into_report()
                .change_context(VRCError::OscError)?;

            let (_buf, _packet) = rosc::decoder::decode_udp(&buf[..size])
                .into_report()
                .change_context(VRCError::OscError)?;

            for (_file_name, (plugin, state)) in &plugins {
                let message_fn = plugin.message();
                message_fn(state, size, RSliceMut::from(&mut buf[..size]));
            }
        }
    }
}
