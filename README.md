# ClickThrough for Horizon

I made this Horizon port with Codex. The [original ClickThrough](https://github.com/gbl/ClickThrough) is by Giselbaer (Guntram Blohm) and is licensed under MIT.

Right-click an item frame, glow item frame, wall sign, wall hanging sign, or wall banner attached to a supported container to use the container directly. Hold your Sneak key while right-clicking to use the marker normally. Supported containers include chests, barrels, shulker boxes, hoppers, dispensers, droppers, furnaces, brewing stands, ender chests, and crafters.

This version always targets containers. It has no settings screen or original sign-text filters.

## Install

1. Download `clickthrough-0.1.0.hnmod` from Releases.
2. Put it in `horizon-native-mods/` in the selected Horizon instance's game folder. Add `mod.clickthrough:true` to that instance's `horizon-mods.txt`.
3. In Horizon Launcher, build the instance's native mods, then play the resulting build.

This release targets Horizon 26.3 Nightly 2026-11-13 (`9f42064`). The launcher compiles the Rust code into the client. The source uses Horizon's normal local and multiplayer block interaction paths.

## Package from source

Run `python3 package.py` with Python 3.11 or newer. It writes the native mod to `dist/`.

## License

[MIT](LICENSE).
