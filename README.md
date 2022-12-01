<div align="center">
  <a href="https://discord.shaybox.com">
    <img alt="Discord" src="https://img.shields.io/discord/824865729445888041?color=404eed&label=Discord&logo=Discord&logoColor=FFFFFF">
  </a>
  <a href="https://github.com/shaybox/vrc-osc/releases/latest">
    <img alt="Downloads" src="https://img.shields.io/github/downloads/shaybox/vrc-osc/total?color=3fb950&label=Downloads&logo=github&logoColor=FFFFFF">
  </a>
</div>

# VRC-OSC

Dynamically loaded cross-platform VRChat OSC plugins in Rust.

### Plugins:
- [`plugin-debug`](/plugin-debug): Print all received packets, enable debug in `config.toml`.
- [`plugin-spotify`](/plugin-spotify): Sends the currently playing track from Spotify to the chatbox.

### Ideas
- [`plugin-controls`](/plugin-controls): Controls various system functions via avatar parameters.
- [`plugin-lastfm`](/plugin-lastfm): Sends the currently playing track from LastFM to the chatbox.
- [`plugin-librefm`](/plugin-librefm): Sends the currently playing track from LibreFM to the chatbox.

I'd like to expand this project with a shared config and socket, and add more plugins.  
If you want to help please feel free to join the [Discord](https://discord.shaybox.com)

## Config:
The configuration file is generated on first-run  
Here's an example with comments
```toml
[debug]
# Log all OSC messages from VRChat to stdout
enable = false

[osc]
# Address to listen for OSC messages
bind_addr = "0.0.0.0:9001"
# Address to send OSC messages
send_addr = "127.0.0.1:9000"

# You must create a Spotify Developer Application
# Follow the instructions on GitHub
# https://github.com/ShayBox/VRC-OSC/tree/master/plugin-spotify
[spotify]
# Spotify Client ID
client_id = ""
# Spotify Client Secret (Non-PKCE)
client_secret = ""
# Enable Spotify Plugin
enable = true
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