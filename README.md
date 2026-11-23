# ClickThrough for Horizon

I made this Horizon port with Codex. The [original ClickThrough](https://github.com/gbl/ClickThrough) is by Giselbaer (Guntram Blohm) and is licensed under MIT.

Right-click an item frame, glow item frame, wall sign, wall hanging sign, or wall banner attached to a supported container to use the container directly. Hold your Sneak key while right-clicking to use the marker normally. Supported containers include chests, barrels, shulker boxes, hoppers, dispensers, droppers, furnaces, brewing stands, ender chests, and crafters.

This version always targets containers. It has no settings screen or original sign-text filters. Only Survival and Creative players are redirected; spectators and adventure players keep vanilla behavior.

## Install

1. Download `clickthrough-0.3.0.hmod` from Releases.
2. In the Horizon Launcher, open Mods, install the `.hmod` from file, and add it to the selected instance. (Manual install also works: copy it into the game folder's `horizon-mods/` and add `mod.clickthrough:true` to `horizon-mods.txt`.)
3. Play. The mod is active whenever it is installed; removing it from the instance disables it. There is no Mods-menu toggle.

This release targets Horizon Nightly 2026-11-22 (`f1bb66f353cf67f2aa4665293fc75c8d91186fc7`), Minecraft 26.3. It is a sandboxed WebAssembly (`.hmod`) mod: it runs in the game's sandbox with no compiler needed on your machine, in single-player and on servers. The retired `.hnmod` native format and the Mods-menu toggle are gone.

## Package from source

Install the mod tool (`cargo install --locked --git https://github.com/horizon-rust/horizon cargo-horizon-mod`) with Rust 1.95 or newer, then run `cargo horizon-mod build --wasm`. It writes the mod to `target/horizon-mod/`. Verify with `cargo horizon-mod check target/horizon-mod/clickthrough.hmod`.

## Differences from the native versions

- The click is redirected server-side (it opens the container's menu directly), so there is no Main-hand swing prediction like the native mixin build had.
- Clicking a marker while riding with a steering stick (carrot or warped fungus on a stick) now opens the container instead of boosting the mount.
- LAN guests get click-through too; the native build skipped them.

## License

[MIT](LICENSE).
