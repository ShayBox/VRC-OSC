<div align="center">
  <a href="https://discord.shaybox.com">
    <img alt="Discord" src="https://img.shields.io/discord/824865729445888041?color=404eed&label=Discord&logo=Discord&logoColor=FFFFFF">
  </a>
  <a href="https://github.com/shaybox/vrc-osc/releases/latest">
    <img alt="Downloads" src="https://img.shields.io/github/downloads/shaybox/vrc-osc/total?color=3fb950&label=Downloads&logo=github&logoColor=FFFFFF">
  </a>
</div>

# VRC-OSC

Dynamically loaded cross-platform VRChat OSC plugins written in Rust.

## Plugins:

- [`plugin-time`](/plugin-clock): Sends the time via avatar parameters for watch prefabs.
- [`plugin-debug`](/plugin-debug): Log all received OSC packets to stdout for debugging.
- [`plugin-spotify`](/plugin-spotify): Displays the currently playing song and controls media playback via avatar
  parameters.

## Planned:

- [`plugin-controls`](/plugin-controls): Controls system media playback via avatar parameters.
- [`plugin-lastfm`](/plugin-lastfm): Displays the currently playing LastFM song.
- [`plugin-librefm`](/plugin-librefm): Displays the currently playing LibreFM song.

## Configuration Documentation:

This is the default configuration file generated at first-run with additional comments for documentation,  
You may copy this as a default configuration, but comments will be wiped on first write.

```toml
# Every plugin has it's own configuration section.

# The Clock plugin sends the time via avatar parameters for watch prefabs.
[clock]
enable = true
# Enable 24 hour mode. (12 = false | 24 = true)
mode = false
# Smooth results using milliseconds, smoother animations.
smooth = false
# How often to send OSC packets. Milliseconds
# Set this between 10-100 when using smoothing.
polling = 1000

# The Debug plugin logs all received OSC packets to stdout for debugging.
# I wouldn't recommend enabling this unless you know what you're doing.
[debug]
enable = false

# The built-in plugin loader handles starting every plugin and relaying incoming OSC packets.
# Every plugin is responsible for sending outgoing OSC packets to the provided address.
# If you're running this on the same computer as VRChat, you won't need to change this.
# If you're using a different PC or Quest you can change the addresses below.
# To receive incoming OSC packets you will need to set the launch option below.
# https://docs.vrchat.com/docs/osc-overview#vrchat-ports
[osc]
bind_addr = "0.0.0.0:9001"
send_addr = "127.0.0.1:9000"

# You must create a Spotify Developer Application
# Follow the instructions on GitHub
# https://github.com/ShayBox/VRC-OSC/tree/master/plugin-spotify
[spotify]
client_id = ""
client_secret = ""
# Displays the currently playing song via in-game chatbox.
enable_chatbox = true
# Controls media playback via avatar parameters.
enable_control = true
# Use Spotify PKCE authentication
# PKCE is the appropriate authentication method for a desktop program
# But it asks for in-browser oauth approval for already approved apps
# Non-PKCE requires a Client Secret but doesn't ask for approval
pkce = false
# Seconds between polling Spotify
# Default VRChat Chatbox Timeout is 30 seconds
polling = 10
# The Spotify callback redirect URI must match on the Spotify Developer Application.
redirect_uri = "http://127.0.0.1:2345"
# Spotify Refresh Token - Saved to reduce in-browser oauth approval
refresh_token = ""
# When enabled, only sends the currently playing song to in-game chatbox once per song.
send_once = false
```