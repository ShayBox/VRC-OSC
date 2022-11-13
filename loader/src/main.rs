use abi_stable::std_types::{RSliceMut, RString};
use common::Error;
use error_stack::{IntoReport, Result, ResultExt};
use std::net::UdpSocket;

fn main() -> Result<(), Error> {
    let config = vrc_osc::load_config()?;
    let osc = UdpSocket::bind(&config.bind_addr)
        .into_report()
        .change_context(Error::OscError)?;

    let plugins = vrc_osc::load_plugins()?;
    for plugin in &plugins {
        let new_fn = plugin.new();
        new_fn(RString::from(config.osc_addr.to_owned()), config.verbose);
    }

    loop {
        let mut buf = [0u8; rosc::decoder::MTU];
        loop {
            let (size, _addr) = osc
                .recv_from(&mut buf)
                .into_report()
                .change_context(Error::OscError)?;

            let (_buf, _packet) = rosc::decoder::decode_udp(&buf[..size])
                .into_report()
                .change_context(Error::OscError)?;

            for plugin in &plugins {
                let message_fn = plugin.message();
                message_fn(size, RSliceMut::from(&mut buf[..size]), config.verbose);
            }
        }
    }
}
