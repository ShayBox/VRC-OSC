<div align="center">
  <a href="https://discord.shaybox.com">
    <img alt="Discord" src="https://img.shields.io/discord/824865729445888041?color=404eed&label=Discord&logo=Discord&logoColor=FFFFFF">
  </a>
  <a href="https://github.com/shaybox/spotify-vrc-osc/releases/latest">
    <img alt="Downloads" src="https://img.shields.io/github/downloads/shaybox/spotify-vrc-osc/total?color=3fb950&label=Downloads&logo=github&logoColor=FFFFFF">
  </a>
</div>

# Spotify-VRC-OSC

Sends the currently playing track on Spotify to the VRChat Chatbox Keyboard via OSC

#### Usage
```
$ spotify-vrc-osc
Usage: spotify-vrc-osc [OPTIONS]

Options:
  -o, --osc-addr <OSC_ADDR>  Address to connect to VRChat OSC device or computer [default: 127.0.0.1:9000]
  -p, --polling <POLLING>    Polling interval in seconds [default: 5]
  -v, --verbose...           More output per occurrence
  -q, --quiet...             Less output per occurrence
  -h, --help                 Print help information
```

#### Screenshot
![Screenshot](Screenshot.png)


#### Ideas
I would like to expand the project into a VRChat OSC core application with plugin loading possibly using [`abi_stable_crates`](https://github.com/rodrimati1992/abi_stable_crates) and add a web-socket and osc-server plugin  
If you want to help or have experience with this, please feel free to join [Discord](https://discord.shaybox.com)