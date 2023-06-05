use std::net::UdpSocket;

use anyhow::Result;
use enigo::{Enigo, Key, KeyboardControllable};
use rosc::{decoder::MTU, OscPacket};

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
pub async fn load(socket: UdpSocket) -> Result<()> {
    let mut enigo = Enigo::new();

    let mut buf = [0u8; MTU];
    loop {
        let size = socket.recv(&mut buf)?;
        let (_buf, packet) = rosc::decoder::decode_udp(&buf[..size])?;
        let OscPacket::Message(packet) = packet else {
            continue; // I don't think VRChat uses bundles
        };

        let addr = packet.addr.replace("/avatar/parameters/VRCOSC/Media/", "");
        match addr.as_ref() {
            "Play" => enigo.key_click(Key::MediaPlayPause),
            "Next" => enigo.key_click(Key::MediaNextTrack),
            "Previous" => enigo.key_click(Key::MediaPrevTrack),
            // "Shuffle" => continue,
            // Seeking is not required because position is not used multiple times
            // "Seeking" => continue,
            "Muted" => enigo.key_click(Key::VolumeMute),
            // "Repeat" => continue,
            // "Volume" => continue,
            // "Position" => continue,
            _ => continue,
        };
    }
}
