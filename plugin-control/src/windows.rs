use std::{collections::HashMap, net::UdpSocket, time::Instant};

use anyhow::Result;
use rosc::{decoder::MTU, OscMessage, OscPacket, OscType};
use windows::Media::{
    Control::{
        GlobalSystemMediaTransportControlsSession,
        GlobalSystemMediaTransportControlsSessionManager as GSMTCSM,
    },
    MediaPlaybackAutoRepeatMode,
};

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
pub async fn load(socket: UdpSocket) -> Result<()> {
    let manager = GSMTCSM::RequestAsync()?.await?;
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

        let Ok(session) = manager.GetCurrentSession() else {
            continue; // No media is currently playing
        };

        let _ = try_sync_media_state_to_vrchat_menu_parameters(&socket, &session);

        if Instant::now().duration_since(instant).as_millis() < 100 {
            continue; // Debounce VRChat menu buttons
        }

        match addr.as_ref() {
            "Play" => {
                let OscType::Bool(play) = arg.to_owned() else {
                    continue;
                };

                if play {
                    session.TryPlayAsync()
                } else {
                    session.TryPauseAsync()
                }
            }
            "Next" => session.TrySkipNextAsync(),
            "Previous" => session.TrySkipPreviousAsync(),
            "Shuffle" => {
                let OscType::Bool(shuffle) = arg.to_owned() else {
                    continue;
                };

                session.TryChangeShuffleActiveAsync(shuffle)
            }
            // Seeking is not required because position is not used multiple times
            // "Seeking" => continue,
            // Muted is removed in newer prefab versions but I still intend to support it
            // "Muted" => continue,
            "Repeat" => {
                let OscType::Int(repeat) = arg.to_owned() else {
                    continue;
                };

                let repeat_mode = MediaPlaybackAutoRepeatMode(repeat);
                session.TryChangeAutoRepeatModeAsync(repeat_mode)
            }
            // The Windows crate doesn't currently support master system volume adjustment
            // "Volume" => continue,
            "Position" => {
                let OscType::Float(position) = arg.to_owned() else {
                    continue;
                };

                let timeline = session.GetTimelineProperties()?;
                let min = timeline.MinSeekTime()?.Duration;
                let max = timeline.MaxSeekTime()?.Duration;
                let playback_position = (min + (max - min) * (position * 100.0) as i64) / 100;

                session.TryChangePlaybackPositionAsync(playback_position)
            }
            _ => continue,
        }?
        .await?;

        instant = Instant::now();
    }
}

/// Try to synchronize the media session state to the VRChat menu parameters
fn try_sync_media_state_to_vrchat_menu_parameters(
    socket: &UdpSocket,
    session: &GlobalSystemMediaTransportControlsSession,
) -> Result<()> {
    let playback_info = session.GetPlaybackInfo()?;
    let mut parameters = HashMap::new();

    if let Ok(playback_status) = playback_info.PlaybackStatus() {
        parameters.insert("Play", OscType::Bool(playback_status.0 == 4));
    }

    if let Ok(shuffle_ref) = playback_info.IsShuffleActive() {
        if let Ok(shuffle) = shuffle_ref.Value() {
            parameters.insert("Shuffle", OscType::Bool(shuffle));
        }
    }

    if let Ok(repeat_mode_ref) = playback_info.AutoRepeatMode() {
        if let Ok(repeat_mode) = repeat_mode_ref.Value() {
            parameters.insert("Repeat", OscType::Int(repeat_mode.0));
        }
    }

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
