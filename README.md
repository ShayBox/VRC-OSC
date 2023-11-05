<div align="center">
  <a href="https://discord.shaybox.com">
    <img alt="Discord" src="https://img.shields.io/discord/824865729445888041?color=404eed&label=Discord&logo=Discord&logoColor=FFFFFF">
  </a>
  <a href="https://github.com/shaybox/vrc-osc/releases/latest">
    <img alt="Downloads" src="https://img.shields.io/github/downloads/shaybox/vrc-osc/total?color=3fb950&label=Downloads&logo=github&logoColor=FFFFFF">
  </a>
</div>

# VRC-OSC

Dynamically loaded VRChat OSC plugins written in Rust

## Plugins:

- [`plugin-clock`](/plugin-clock): Sends the time to avatar prefabs
- [`plugin-control`](/plugin-control): Control media playback via avatar parameters[^1]
- [`plugin-debug`](/plugin-debug): Log received OSC packets for debugging
- [`plugin-lastfm`](/plugin-lastfm): Sends the current song to the chatbox
- [`plugin-spotify`](/plugin-spotify): Sends the current song and lyrics to the chatbox and control playback via avatar prefabs
- [`plugin-steamvr`](/plugin-steamvr): Registers VRC-OSC as a SteamVR overlay for auto-start/stop[^1]

## Planned:

- `plugin-caption`: Live captions your speech to the chatbox[^2]

[^1]: These plugins are Windows and Linux only
[^2]: This plugin waiting for `whisper-rs`'s stream example