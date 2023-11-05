<div align="center">
  <a href="https://discord.shaybox.com">
    <img alt="Discord" src="https://img.shields.io/discord/824865729445888041?color=404eed&label=Discord&logo=Discord&logoColor=FFFFFF">
  </a>
  <a href="https://github.com/shaybox/vrc-osc/releases/latest">
    <img alt="Downloads" src="https://img.shields.io/github/downloads/shaybox/vrc-osc/total?color=3fb950&label=Downloads&logo=github&logoColor=FFFFFF">
  </a>
</div>

# Spotify Plugin

Sends the current song and lyrics to the chatbox  
Control playback via avatar prefabs[^1]

## How to Setup

The Spotify plugin requires you to create a Spotify Developer Application

1. Create a [Developer Application](https://developer.spotify.com/dashboard)
2. Set the `Redirect URI` to `http://127.0.0.1:2345`
3. Click on the `Settings` button
4. Copy the `Client ID` and paste it into the Setup Wizard
5. Click on the `View client secret` button
6. Copy the `Client secret` and paste it into the Setup Wizard
7. Make sure the `Redirect URI` matches in the Setup Wizard

[If you need additional help you can contact me](https://shaybox.com)

## Avatar Parameters

This plugin fully supports the [VRCOSC Media Prefab]  
Support for additional prefabs are welcome

| Parameter             | Type  |
| --------------------- | ----- |
| VRCOSC/Media/Play     | Bool  |
| VRCOSC/Media/Next     | None  |
| VRCOSC/Media/Previous | None  |
| VRCOSC/Media/Shuffle  | Bool  |
| VRCOSC/Media/Seeking  | Bool  |
| VRCOSC/Media/Muted    | Bool  |
| VRCOSC/Media/Repeat   | Int   |
| VRCOSC/Media/Volume   | Float |
| VRCOSC/Media/Position | Float |

[^1]: This feature requires Spotify Premium
[VRCOSC Media Prefab]: https://github.com/VolcanicArts/VRCOSC/releases/latest
