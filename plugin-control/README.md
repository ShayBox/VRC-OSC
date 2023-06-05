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

This plugin **partially**[^1] supports the [VRCOSC Media Prefab]  
Support for additional prefabs are welcome

| Parameter             | Type     | 🪟 | 🐧 |
|-----------------------|----------|----|----|
| VRCOSC/Media/Play     | Bool[^2] | ✅  | ✅  |
| VRCOSC/Media/Next     | None     | ✅  | ✅  |
| VRCOSC/Media/Previous | None     | ✅  | ✅  |
| VRCOSC/Media/Shuffle  | Bool     | ✅  | ❌  |
| VRCOSC/Media/Seeking  | Bool     | ✅  | ❌  |
| VRCOSC/Media/Muted    | Bool     | ❌  | ✅  |
| VRCOSC/Media/Repeat   | Int      | ✅  | ❌  |
| VRCOSC/Media/Volume   | Float    | ❌  | ❌  |
| VRCOSC/Media/Position | Float    | ✅  | ❌  |

[^1]: The Windows crate doesn't currently support Volume  
[^2]: Linux support is lacking, issues and pull requests are welcome  

[VRCOSC Media Prefab]: https://github.com/VolcanicArts/VRCOSC/releases/latest
