use std::net::UdpSocket;

use anyhow::Result;
use rosc::OscPacket;

#[no_mangle]
#[allow(clippy::needless_pass_by_value)]
#[tokio::main(flavor = "current_thread")]
async extern "Rust" fn load(socket: UdpSocket) -> Result<()> {
    println!("Debug Enabled");

    let mut buf = [0u8; rosc::decoder::MTU];
    loop {
        let size = socket.recv(&mut buf).unwrap();
        let (_buf, packet) = rosc::decoder::decode_udp(&buf[..size]).unwrap();
        let OscPacket::Message(packet) = packet else {
            continue; // I don't think VRChat uses bundles
        };

        println!("{} | {:?}", packet.addr, packet.args);
    }
}
