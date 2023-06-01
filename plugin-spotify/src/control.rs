use std::{net::UdpSocket, sync::Arc};

use anyhow::Result;
use ferrispot::{
    client::authorization_code::SyncAuthorizationCodeUserClient,
    model::playback::RepeatState,
    prelude::*,
};
use rosc::{OscPacket, OscType};

pub fn thread_control(
    socket: Arc<UdpSocket>,
    spotify: SyncAuthorizationCodeUserClient,
) -> Result<()> {
    let mut buf = [0u8; rosc::decoder::MTU];

    let mut previous_volume = None;
    loop {
        let (size, _addr) = socket.recv_from(&mut buf).unwrap();
        let (_buf, packet) = rosc::decoder::decode_udp(&buf[..size]).unwrap();
        let OscPacket::Message(packet) = packet else {
            continue; // I don't think VRChat uses bundles
        };

        let addr = packet.addr.replace("/avatar/parameters/VRCOSC/Media/", "");
        let Some(arg) = packet.args.first() else {
            continue;
        };

        let Some(playback_state) = spotify.playback_state().send_sync()? else {
            continue;
        };

        if previous_volume.is_none() {
            previous_volume = Some(playback_state.device().volume_percent());
        }

        let request = match addr.as_ref() {
            "Play" => spotify.pause(),
            "Next" => spotify.next(),
            "Previous" => spotify.previous(),
            "Shuffle" => {
                if let OscType::Bool(state) = arg.to_owned() {
                    spotify.shuffle(state)
                } else {
                    continue;
                }
            }
            "Muted" => {
                if let OscType::Bool(mute) = arg.to_owned() {
                    if mute {
                        previous_volume = Some(playback_state.device().volume_percent());
                        spotify.volume(0)
                    } else {
                        spotify.volume(previous_volume.expect("Failed to get previous volume"))
                    }
                } else {
                    continue;
                }
            }
            "Repeat" => {
                if let OscType::Int(state) = arg.to_owned() {
                    let repeat_state = match state {
                        0 => RepeatState::Off,
                        1 => RepeatState::Track,
                        2 => RepeatState::Context,
                        _ => continue,
                    };
                    spotify.repeat_state(repeat_state)
                } else {
                    continue;
                }
            }
            "Volume" => {
                if let OscType::Float(volume) = arg.to_owned() {
                    spotify.volume((volume * 100.0) as u8)
                } else {
                    continue;
                }
            }
            _ => continue,
        };
        request.send_sync()?;
    }
}
