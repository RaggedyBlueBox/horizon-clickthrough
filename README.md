# ClickThrough for Horizon

I made this Horizon port with Codex. The [original ClickThrough](https://github.com/gbl/ClickThrough) is by Giselbaer (Guntram Blohm) and is licensed under MIT.

Right-click a wall sign, wall hanging sign, or wall banner attached to a supported container to use the container directly. Hold your Sneak key while right-clicking to use the marker normally. Supported containers include chests, barrels, shulker boxes, hoppers, dispensers, droppers, furnaces, brewing stands, ender chests, and crafters.

Item frames keep vanilla behavior: the mod API exposes no way to resolve the clicking player's eye on the entity path, so the destination raycast the container requires cannot be built there, and unverifiable redirects are left alone.

This version always targets containers. It has no settings screen or original sign-text filters. Only Survival and Creative players are redirected; spectators and adventure players keep vanilla behavior.

## Install

1. Download `clickthrough-0.3.1.hmod` from Releases.
2. In the Horizon Launcher, open Mods, install the `.hmod` from file, and add it to the selected instance. (Manual install also works: copy it into the game folder's `horizon-mods/` and add `mod.clickthrough:true` to `horizon-mods.txt`.)
3. Play. The mod is active whenever it is installed; removing it from the instance disables it. There is no Mods-menu toggle.

This release targets Horizon Nightly 2026-11-22 (`f1bb66f353cf67f2aa4665293fc75c8d91186fc7`), Minecraft 26.3. It is a sandboxed WebAssembly (`.hmod`) mod: it runs in the game's sandbox with no compiler needed on your machine, in single-player and on servers. The Mods-menu toggle is gone.

## Package from source

Install the mod tool (`cargo install --locked --git https://github.com/horizon-rust/horizon cargo-horizon-mod`) with Rust 1.95 or newer, then run `cargo horizon-mod build --wasm`. It writes the mod to `target/horizon-mod/`.

## How the redirect is validated

The server replays the checks a direct use of the container would run before opening its menu: the marker is hit through a clear outline ray, the container is hit through a ray that may pass only the marker, the player may use both blocks, the container sits inside the world border, and a chest with a redstone conductor above it stays shut. Anything else falls through to vanilla behavior (for example, a chest with a cat sitting on it, which the mod API cannot see).

## License

[MIT](LICENSE).
