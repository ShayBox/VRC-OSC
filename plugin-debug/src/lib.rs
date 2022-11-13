use abi_stable::{
    export_root_module,
    prefix_type::PrefixTypeTrait,
    sabi_extern_fn,
    std_types::{RSliceMut, RString},
};
use common::{OSCMod, OSCMod_Ref};
use rosc::OscPacket;

#[export_root_module]
fn instantiate_root_module() -> OSCMod_Ref {
    OSCMod { new, message }.leak_into_prefix()
}

#[sabi_extern_fn]
pub fn new(_osc_addr: RString, _verbose: bool) -> () {}

#[sabi_extern_fn]
pub fn message(size: usize, buf: RSliceMut<u8>, verbose: bool) -> () {
    let (_, packet) = rosc::decoder::decode_udp(&buf[..size]).unwrap();
    let OscPacket::Message(packet) = packet else {
        return (); // VRChat doesn't have bundles afaik
    };

    if verbose {
        println!("{} | {:?}", packet.addr, packet.args);
    }
}
