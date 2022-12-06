use abi_stable::{
    export_root_module, prefix_type::PrefixTypeTrait, sabi_extern_fn, sabi_trait::TD_Opaque,
};
use common::{
    config::VrcConfig, error::VrcError, CommonState_TO, OSCMod, OSCMod_Ref, OscState, StateBox,
};
use error_stack::{IntoReport, ResultExt};
use rosc::OscPacket;
use std::{net::UdpSocket, thread};

#[export_root_module]
fn instantiate_root_module() -> OSCMod_Ref {
    OSCMod { new }.leak_into_prefix()
}

#[sabi_extern_fn]
pub fn new() -> StateBox {
    let config = VrcConfig::load().unwrap();

    let osc = UdpSocket::bind("127.0.0.1:0")
        .into_report()
        .change_context(VrcError::Osc)
        .unwrap();

    let local_addr = osc
        .local_addr()
        .into_report()
        .change_context(VrcError::Io)
        .unwrap();

    let state = OscState {
        bind_addr: local_addr.to_string().into(),
        send_messages: config.debug.enable,
    };

    if config.debug.enable {
        println!("Debug is enabled");
        thread::Builder::new()
            .name("Debug Plugin".to_string())
            .spawn(move || {
                let mut buf = [0u8; rosc::decoder::MTU];
                loop {
                    let (size, _addr) = osc
                        .recv_from(&mut buf)
                        .into_report()
                        .change_context(VrcError::Osc)
                        .unwrap();

                    let (_buf, packet) = rosc::decoder::decode_udp(&buf[..size])
                        .into_report()
                        .change_context(VrcError::Osc)
                        .unwrap();

                    let OscPacket::Message(packet) = packet else {
                    continue; // VRChat doesn't have bundles afaik
                };

                    println!("{} | {:?}", packet.addr, packet.args);
                }
            })
            .expect("Debug Plugin failed");
    } else {
        println!("Debug is disabled");
    }

    CommonState_TO::from_value(state, TD_Opaque)
}
