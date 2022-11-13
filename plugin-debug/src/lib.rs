use abi_stable::{
    export_root_module, prefix_type::PrefixTypeTrait, sabi_extern_fn, sabi_trait::TD_Opaque,
    std_types::RSliceMut,
};
use common::{config::VrcConfig, OSCMod, OSCMod_Ref, State, StateBox, State_TO};
use rosc::OscPacket;

#[derive(Clone, Debug)]
struct DebugState {
    enable: bool,
}
impl State for DebugState {
    fn is_enabled(&self) -> bool {
        self.enable
    }
}

#[export_root_module]
fn instantiate_root_module() -> OSCMod_Ref {
    OSCMod { new, message }.leak_into_prefix()
}

#[sabi_extern_fn]
pub fn new() -> StateBox {
    let config = VrcConfig::load().unwrap();
    let state = DebugState {
        enable: config.debug.enable,
    };
    State_TO::from_value(state, TD_Opaque)
}

#[sabi_extern_fn]
pub fn message(state: &StateBox, size: usize, buf: RSliceMut<u8>) -> () {
    let (_, packet) = rosc::decoder::decode_udp(&buf[..size]).unwrap();
    let OscPacket::Message(packet) = packet else {
        return (); // VRChat doesn't have bundles afaik
    };

    if state.is_enabled() {
        println!("{} | {:?}", packet.addr, packet.args);
    }
}
