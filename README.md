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
- [`plugin-debug`](/plugin-debug): Log all received OSC packets to stdout for debugging.
- [`plugin-spotify`](/plugin-spotify): Displays the currently playing song and controls media playback via avatar parameters.

## Planned:
- [`plugin-controls`](/plugin-controls): Controls system media playback via avatar parameters.
- [`plugin-lastfm`](/plugin-lastfm): Displays the currently playing LastFM song.
- [`plugin-librefm`](/plugin-librefm): Displays the currently playing LibreFM song.

## Configuration Documentation:
This is the default configuration file generated at first-run with additional comments for documentation,  
Do not include comments in your configuration file, overwriting a file with comments will cause corruption.  
```toml
[debug]
# This plugin will log all received OSC packets to stdout for debugging.
enable = false

[osc]
# Address to bind UdpSocket to listen for OSC messages.
# This listens for messages on everything, you likely won't need to change this setting.
# To receive messages from other devices than the local computer you will need to set a parameter.
# https://docs.vrchat.com/docs/osc-overview#vrchat-ports
bind_addr = "0.0.0.0:9001"
# Address to send OSC messages to, default is your local computer.
# You can set this to any device on your local network, such as a Quest.
send_addr = "127.0.0.1:9000"

# You must create a Spotify Developer Application
# Follow the instructions on GitHub
# https://github.com/ShayBox/VRC-OSC/tree/master/plugin-spotify
[spotify]
# Spotify Client ID
client_id = ""
# Spotify Client Secret (Non-PKCE)
client_secret = ""
# Enable Spotify Currently Playing Chatbox
enable_chatbox = true
# Enable Spotify Media Control (Avatar Parameters)
enable_control = true
# Use Spotify PKCE authentication
# PKCE is the appropriate authentication method for a desktop program
# But it asks for in-browser oauth approval for already approved apps
# Non-PKCE requires a Client Secret but doesn't ask for approval
pkce = false
# Seconds between polling Spotify
# Default VRChat Chatbox Timeout is 30 seconds
polling = 10
# Spotify Redirect URI
redirect_uri = "http://127.0.0.1:2345"
# Spotify Refresh Token - Saved to reduce in-browser oauth approval
refresh_token = ""
# Only send one chatbox message per song change
send_once = false
```