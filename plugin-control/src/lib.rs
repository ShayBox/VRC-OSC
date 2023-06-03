use std::net::UdpSocket;

use anyhow::Result;
use enigo::{Enigo, Key, KeyboardControllable};
use rosc::{decoder::MTU, OscPacket};

#[no_mangle]
fn main(socket: UdpSocket) -> Result<()> {
    let mut buf = [0u8; MTU];
    let mut enigo = Enigo::new();
    loop {
        let (size, _addr) = socket.recv_from(&mut buf).unwrap();
        let (_buf, packet) = rosc::decoder::decode_udp(&buf[..size]).unwrap();
        let OscPacket::Message(packet) = packet else {
            continue; // I don't think VRChat uses bundles
        };

        let addr = packet.addr.replace("/avatar/parameters/VRCOSC/Media/", "");
        match addr.as_ref() {
            "Play" => enigo.key_click(Key::MediaPlayPause),
            "Next" => enigo.key_click(Key::MediaNextTrack),
            "Previous" => enigo.key_click(Key::MediaPrevTrack),
            "Muted" => enigo.key_click(Key::VolumeMute),
            _ => continue,
        };
    }
}
