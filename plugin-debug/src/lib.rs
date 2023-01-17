use std::{net::UdpSocket, thread::Builder};

use abi_stable::{
    export_root_module,
    prefix_type::PrefixTypeTrait,
    sabi_extern_fn,
    sabi_trait::TD_Opaque,
};
use anyhow::Result;
use common::{config::VrcConfig, CommonState_TO, OSCMod, OSCMod_Ref, OscState, StateBox};
use rosc::OscPacket;

#[export_root_module]
fn instantiate_root_module() -> OSCMod_Ref {
    OSCMod { new }.leak_into_prefix()
}

#[sabi_extern_fn]
pub fn new() -> StateBox {
    let config = VrcConfig::load().unwrap();
    let osc = UdpSocket::bind("127.0.0.1:0").unwrap();
    let local_addr = osc.local_addr().unwrap();

    let state = OscState {
        bind_addr: local_addr.to_string().into(),
        send_messages: config.debug.enable,
    };

    if config.debug.enable {
        println!("Debug is enabled");
        Builder::new()
            .name("Debug Plugin".to_string())
            .spawn(move || thread_debug(config, &osc).expect("thread_debug"))
            .expect("Debug Plugin failed");
    } else {
        println!("Debug is disabled");
    }

    CommonState_TO::from_value(state, TD_Opaque)
}

fn thread_debug(_config: VrcConfig, osc: &UdpSocket) -> Result<()> {
    let mut buf = [0u8; rosc::decoder::MTU];
    loop {
        let (size, _addr) = osc.recv_from(&mut buf).unwrap();
        let (_buf, packet) = rosc::decoder::decode_udp(&buf[..size]).unwrap();
        let OscPacket::Message(packet) = packet else {
            continue; // I don't think VRChat uses bundles
        };

        println!("{} | {:?}", packet.addr, packet.args);
    }
}
