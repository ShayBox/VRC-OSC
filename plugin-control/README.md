<div align="center">
  <a href="https://discord.shaybox.com">
    <img alt="Discord" src="https://img.shields.io/discord/824865729445888041?color=404eed&label=Discord&logo=Discord&logoColor=FFFFFF">
  </a>
  <a href="https://github.com/shaybox/vrc-osc/releases/latest">
    <img alt="Downloads" src="https://img.shields.io/github/downloads/shaybox/vrc-osc/total?color=3fb950&label=Downloads&logo=github&logoColor=FFFFFF">
  </a>
</div>

# Control Plugin

Control media playback via avatar parameters  
This plugin is Windows and Linux only because macOS doesn't support Media keys

## Avatar Parameters

This plugin **partially**[^1][^2] supports the [VRCOSC Media Prefab]  
Support for additional prefabs are welcome

| Parameter             | Type     | 🪟    | 🐧    |
|-----------------------|----------|-------|-------|
| VRCOSC/Media/Play     | Bool[^2] | ✅     | ✅     |
| VRCOSC/Media/Next     | None     | ✅     | ✅     |
| VRCOSC/Media/Previous | None     | ✅     | ✅     |
| VRCOSC/Media/Shuffle  | Bool     | ✅     | ❌[^2] |
| VRCOSC/Media/Seeking  | Bool     | ✅     | ❌[^2] |
| VRCOSC/Media/Muted    | Bool     | ❌[^1] | ✅     |
| VRCOSC/Media/Repeat   | Int      | ✅     | ❌[^2] |
| VRCOSC/Media/Volume   | Float    | ❌[^1] | ❌[^2] |
| VRCOSC/Media/Position | Float    | ✅     | ❌[^2] |

[^1]: The `windows` crate doesn't currently support volume  
[^2]: Support for Linux is lacking, issues and pull requests are welcome
[VRCOSC Media Prefab]: https://github.com/VolcanicArts/VRCOSC/releases/latest
