//! ClickThrough as a sandboxed WebAssembly mod.
//!
//! Right-click an item frame, wall sign, wall hanging sign, or wall banner
//! attached to a container to open the container, on either logical side:
//! the server hook cancels the marker's own use and opens the container's
//! menu for the clicking player instead. Hold Sneak to use the marker
//! normally. Only Survival and Creative players are redirected.
use mod_api::{
    self, EntityInteractSpecificEvent, Flow, GameMode, Hand, HostSide, Hooks, InteractionResult,
    PlayerRef, RightClickBlockEvent, ServerHost, TriState,
};

#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn right_click_block(&self, host: HostSide<'_>, event: &mut RightClickBlockEvent<'_>) -> Flow {
        let HostSide::Server(host) = host else {
            // Let the click through to the server; only the server redirects.
            return Flow::Continue;
        };
        if event.click.hand != Hand::Main {
            return Flow::Continue;
        }
        let Some(player) = host.player(event.click.player) else {
            return Flow::Continue;
        };
        if !player_clear(host, player) {
            return Flow::Continue;
        }
        let Some(level) = player.level(host) else {
            return Flow::Continue;
        };
        let Some(container) = container_for_marker(host, level, event.click.pos) else {
            return Flow::Continue;
        };
        let Some(entity) = host.block_entity(level, container) else {
            return Flow::Continue;
        };
        if player.open_menu(host, entity) != Ok(true) {
            return Flow::Continue;
        }
        *event.use_block = TriState::False;
        *event.use_item = TriState::False;
        *event.cancellation_result = InteractionResult::Fail;
        Flow::Cancel
    }

    fn entity_interact_specific(
        &self,
        host: &mut dyn ServerHost,
        event: &mut EntityInteractSpecificEvent<'_>,
    ) -> Flow {
        if !matches!(
            event.kind,
            "minecraft:item_frame" | "minecraft:glow_item_frame"
        ) {
            return Flow::Continue;
        }
        let Some(player) = host.players().acting_player() else {
            return Flow::Continue;
        };
        if !player_clear(host, player) {
            return Flow::Continue;
        }
        let Some(level) = player.level(host) else {
            return Flow::Continue;
        };
        let Ok(id) = u32::try_from(event.entity) else {
            return Flow::Continue;
        };
        let Some(frame) = host.entity(id) else {
            return Flow::Continue;
        };
        let Some(frame_pos) = frame.block_position(host) else {
            return Flow::Continue;
        };
        let Some(facing) = frame.hanging_direction(host) else {
            return Flow::Continue;
        };
        let normal = facing.normal();
        let container = [
            frame_pos[0] - normal[0],
            frame_pos[1] - normal[1],
            frame_pos[2] - normal[2],
        ];
        if !is_container_at(host, level, container) {
            return Flow::Continue;
        }
        let Some(entity) = host.block_entity(level, container) else {
            return Flow::Continue;
        };
        if player.open_menu(host, entity) != Ok(true) {
            return Flow::Continue;
        }
        Flow::Cancel
    }
}

/// Whether the mod redirects this player's clicks: Survival and Creative
/// players who are not sneaking. Spectators, adventure players and fake
/// players keep vanilla behavior.
fn player_clear(host: &dyn ServerHost, player: PlayerRef) -> bool {
    if !matches!(
        player.game_mode(host),
        Some(GameMode::Survival) | Some(GameMode::Creative)
    ) {
        return false;
    }
    if player.is_secondary_use_active(host) == Some(true) {
        return false;
    }
    true
}

/// The container behind a wall sign, wall hanging sign or wall banner at
/// `marker`, or `None` when the marker or what is behind it does not qualify.
fn container_for_marker(
    host: &dyn ServerHost,
    level: mod_api::LevelRef,
    marker: [i32; 3],
) -> Option<[i32; 3]> {
    let name = level.block_state(host, marker)?.block().id(host);
    if !is_marker(&name) {
        return None;
    }
    let facing = level.block_property(host, marker, "facing")?;
    let normal = face_normal(&facing)?;
    let container = [
        marker[0] - normal[0],
        marker[1] - normal[1],
        marker[2] - normal[2],
    ];
    is_container_at(host, level, container).then_some(container)
}

/// Whether the block at `pos` is a container the mod opens.
fn is_container_at(host: &dyn ServerHost, level: mod_api::LevelRef, pos: [i32; 3]) -> bool {
    level
        .block_state(host, pos)
        .is_some_and(|state| is_container(&state.block().id(host)))
}

fn is_marker(name: &str) -> bool {
    name.ends_with("_wall_sign")
        || name.ends_with("_wall_hanging_sign")
        || name.ends_with("_wall_banner")
}

fn is_container(name: &str) -> bool {
    name == "minecraft:chest"
        || name == "minecraft:trapped_chest"
        || name.ends_with("shulker_box")
        || matches!(
            name,
            "minecraft:ender_chest"
                | "minecraft:barrel"
                | "minecraft:hopper"
                | "minecraft:dispenser"
                | "minecraft:dropper"
                | "minecraft:crafter"
                | "minecraft:furnace"
                | "minecraft:blast_furnace"
                | "minecraft:smoker"
                | "minecraft:brewing_stand"
        )
}

fn face_normal(face: &str) -> Option<[i32; 3]> {
    match face {
        "north" => Some([0, 0, -1]),
        "south" => Some([0, 0, 1]),
        "west" => Some([-1, 0, 0]),
        "east" => Some([1, 0, 0]),
        _ => None,
    }
}
