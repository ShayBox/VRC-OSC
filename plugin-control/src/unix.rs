use std::net::UdpSocket;

use anyhow::Result;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use rosc::{OscPacket, decoder::MTU};

/// # Errors
/// Will return `Err` if `Enigo::new`, `UdpSocket::recv`, `decode_udp`, or `Enigo::key` fails.
///
/// # Panics
/// Will panic if the infinite loop exits.
#[unsafe(no_mangle)]
#[allow(clippy::needless_pass_by_value)]
#[tokio::main(flavor = "current_thread")]
pub async extern "Rust" fn load(socket: UdpSocket) -> Result<()> {
    let settings = Settings::default();
    let mut enigo = Enigo::new(&settings)?;

    let mut buf = [0u8; MTU];
    loop {
        let size = socket.recv(&mut buf)?;
        let (_buf, packet) = rosc::decoder::decode_udp(&buf[..size])?;
        let OscPacket::Message(packet) = packet else {
            continue; // I don't think VRChat uses bundles
        };

        let addr = packet.addr.replace("/avatar/parameters/VRCOSC/Media/", "");
        match addr.as_ref() {
            "Play" => enigo.key(Key::MediaPlayPause, Direction::Click),
            "Next" => enigo.key(Key::MediaNextTrack, Direction::Click),
            "Previous" => enigo.key(Key::MediaPrevTrack, Direction::Click),
            // "Shuffle" => continue,
            // Seeking is not required because position is not used multiple times
            // "Seeking" => continue,
            "Muted" => enigo.key(Key::VolumeMute, Direction::Click),
            // "Repeat" => continue,
            // "Volume" => continue,
            // "Position" => continue,
            _ => continue,
        }?;
    }
}
