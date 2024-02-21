use std::{collections::HashMap, net::UdpSocket};

use anyhow::Result;
use ferrispot::{
    client::authorization_code::AsyncAuthorizationCodeUserClient,
    model::playback::RepeatState,
    prelude::*,
};
use rosc::{decoder::MTU, OscMessage, OscPacket, OscType};

#[allow(clippy::too_many_lines)]
pub async fn start_loop(
    socket: UdpSocket,
    spotify: AsyncAuthorizationCodeUserClient,
) -> Result<()> {
    let mut previous_parameters = HashMap::new();
    let mut muted_volume = None;
    let mut buf = [0u8; MTU];
    loop {
        let size = socket.recv(&mut buf)?;
        let (_buf, packet) = rosc::decoder::decode_udp(&buf[..size])?;
        let OscPacket::Message(packet) = packet else {
            continue; // I don't think VRChat uses bundles
        };

        let addr = packet.addr.replace("/avatar/parameters/VRCOSC/Media/", "");
        let Some(arg) = packet.args.first() else {
            continue; // No first argument was supplied
        };

        let Some(playback_state) = spotify.playback_state().send_async().await? else {
            continue; // No media is currently playing
        };

        if muted_volume.is_none() {
            muted_volume = Some(playback_state.device().volume_percent());
        }

        let request = match addr.as_ref() {
            "Play" => {
                let OscType::Bool(play) = arg.to_owned() else {
                    continue;
                };

                if play {
                    spotify.resume()
                } else {
                    spotify.pause()
                }
            }
            "Next" => spotify.next(),
            "Prev" | "Previous" => spotify.previous(),
            "Shuffle" => {
                let OscType::Bool(shuffle) = arg.to_owned() else {
                    continue;
                };

                spotify.shuffle(shuffle)
            }
            // Seeking is not required because position is not used multiple times
            // "Seeking" => continue,
            "Muted" => {
                let OscType::Bool(mute) = arg.to_owned() else {
                    continue;
                };

                let volume = if mute {
                    muted_volume = Some(playback_state.device().volume_percent());
                    0
                } else {
                    muted_volume.expect("Failed to get previous volume")
                };

                spotify.volume(volume)
            }
            "Repeat" => {
                let OscType::Int(repeat) = arg.to_owned() else {
                    continue;
                };

                let repeat_state = match repeat {
                    0 => RepeatState::Off,
                    1 => RepeatState::Track,
                    2 => RepeatState::Context,
                    _ => continue,
                };

                spotify.repeat_state(repeat_state)
            }
            "Volume" => {
                let OscType::Float(volume) = arg.to_owned() else {
                    continue;
                };

                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                spotify.volume((volume * 100.0) as u8)
            }
            "Position" => {
                let OscType::Float(position) = arg.to_owned() else {
                    continue;
                };

                let min = 0;
                let max = playback_state.currently_playing_item().timestamp();

                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                spotify.seek((min + (max - min) * (position * 100.0) as u64) / 100)
            }
            _ => {
                let Some(playback_state) = spotify.playback_state().send_async().await? else {
                    return Ok(()); // No media is currently playing
                };

                let mut parameters = HashMap::new();

                let play = OscType::Bool(playback_state.device().is_active());
                if previous_parameters.get("Play") != Some(&play) {
                    parameters.insert("Play", play.clone());
                    previous_parameters.insert("Play", play);
                }

                let shuffle = OscType::Bool(playback_state.shuffle_state());
                if previous_parameters.get("Shuffle") != Some(&shuffle) {
                    parameters.insert("Shuffle", shuffle.clone());
                    previous_parameters.insert("Shuffle", shuffle);
                }

                let repeat = OscType::Int(match playback_state.repeat_state() {
                    RepeatState::Off => 0,
                    RepeatState::Track => 1,
                    RepeatState::Context => 2,
                });
                if previous_parameters.get("Repeat") != Some(&repeat) {
                    parameters.insert("Repeat", repeat.clone());
                    previous_parameters.insert("Repeat", repeat);
                }

                for (param, arg) in parameters {
                    let packet = OscPacket::Message(OscMessage {
                        addr: format!("/avatar/parameters/VRCOSC/Media/{param}"),
                        args: vec![arg],
                    });

                    let msg_buf = rosc::encoder::encode(&packet)?;
                    socket.send(&msg_buf)?;
                }

                continue;
            }
        };

        if let Err(error) = request.send_async().await {
            eprintln!("Spotify Control Error: {error}");
        };
    }
}
