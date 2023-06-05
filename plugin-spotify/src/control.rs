use std::{collections::HashMap, net::UdpSocket, sync::Arc, time::Instant};

use anyhow::Result;
use ferrispot::{
    client::authorization_code::AsyncAuthorizationCodeUserClient,
    model::playback::RepeatState,
    prelude::*,
};
use rosc::{decoder::MTU, OscMessage, OscPacket, OscType};

pub async fn task_control(
    socket: Arc<UdpSocket>,
    spotify: AsyncAuthorizationCodeUserClient,
) -> Result<()> {
    let mut muted_volume = None;
    let mut instant = Instant::now();
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

        let _ = try_sync_media_state_to_vrchat_menu_parameters(&socket, &spotify).await;

        if Instant::now().duration_since(instant).as_secs() < 1 {
            continue; // Debounce VRChat menu buttons
        }

        if muted_volume.is_none() {
            muted_volume = Some(playback_state.device().volume_percent());
        }

        match addr.as_ref() {
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
            "Previous" => spotify.previous(),
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

                spotify.volume((volume * 100.0) as u8)
            }
            "Position" => {
                let OscType::Float(position) = arg.to_owned() else {
                    continue;
                };

                let min = 0;
                let max = playback_state.currently_playing_item().timestamp();
                let playback_position = (min + (max - min) * (position * 100.0) as u64) / 100;

                spotify.seek(playback_position)
            }
            _ => continue,
        }
        .send_async()
        .await?;

        instant = Instant::now();
    }
}

/// Try to synchronize the media session state to the VRChat menu parameters
async fn try_sync_media_state_to_vrchat_menu_parameters(
    socket: &UdpSocket,
    spotify: &AsyncAuthorizationCodeUserClient,
) -> Result<()> {
    let Some(playback_state) = spotify.playback_state().send_async().await? else {
        return Ok(()); // No media is currently playing
    };

    let mut parameters = HashMap::new();

    let play = playback_state.device().is_active();
    parameters.insert("Play", OscType::Bool(play));

    let shuffle = playback_state.shuffle_state();
    parameters.insert("Shuffle", OscType::Bool(shuffle));

    let repeat = match playback_state.repeat_state() {
        RepeatState::Off => 0,
        RepeatState::Track => 1,
        RepeatState::Context => 2,
    };
    parameters.insert("Repeat", OscType::Int(repeat));

    for (param, arg) in parameters {
        let packet = OscPacket::Message(OscMessage {
            addr: format!("/avatar/parameters/VRCOSC/Media/{param}"),
            args: vec![arg],
        });

        let msg_buf = rosc::encoder::encode(&packet)?;
        socket.send(&msg_buf)?;
    }

    Ok(())
}
