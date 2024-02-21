use std::{collections::HashMap, net::UdpSocket};

use anyhow::Result;
use rosc::{decoder::MTU, OscMessage, OscPacket, OscType};
use windows::Media::{
    Control::GlobalSystemMediaTransportControlsSessionManager as GSMTCSM,
    MediaPlaybackAutoRepeatMode,
};

/// # Errors
///
/// # Panics
#[no_mangle]
#[allow(clippy::needless_pass_by_value)]
#[tokio::main(flavor = "current_thread")]
pub async extern "Rust" fn load(socket: UdpSocket) -> Result<()> {
    let manager = GSMTCSM::RequestAsync()?.await?;
    let mut previous_parameters = HashMap::new();
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

                #[allow(clippy::cast_possible_truncation)]
                let playback_position = (min + (max - min) * (position * 100.0) as i64) / 100;
                session.TryChangePlaybackPositionAsync(playback_position)
            }
            _ => {
                let playback_info = session.GetPlaybackInfo()?;
                let mut parameters = HashMap::new();

                if let Ok(playback_status) = playback_info.PlaybackStatus() {
                    let play = OscType::Bool(playback_status.0 == 4);
                    if previous_parameters.get("Play") != Some(&play) {
                        parameters.insert("Play", play.clone());
                        previous_parameters.insert("Play", play);
                    }
                }

                if let Ok(shuffle_ref) = playback_info.IsShuffleActive() {
                    if let Ok(shuffle) = shuffle_ref.Value() {
                        let shuffle = OscType::Bool(shuffle);
                        if previous_parameters.get("Shuffle") != Some(&shuffle) {
                            parameters.insert("Shuffle", shuffle.clone());
                            previous_parameters.insert("Shuffle", shuffle);
                        }
                    }
                }

                if let Ok(repeat_mode_ref) = playback_info.AutoRepeatMode() {
                    if let Ok(repeat_mode) = repeat_mode_ref.Value() {
                        let repeat = OscType::Int(repeat_mode.0);
                        if previous_parameters.get("Repeat") != Some(&repeat) {
                            parameters.insert("Repeat", repeat.clone());
                            previous_parameters.insert("Repeat", repeat);
                        }
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

                continue;
            }
        }?
        .await?;
    }
}
