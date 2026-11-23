//! ClickThrough as a sandboxed WebAssembly mod.
//!
//! Right-click an item frame, wall sign, wall hanging sign, or wall banner
//! attached to a container to open the container, on either logical side:
//! the server hook cancels the marker's own use and opens the container's
//! menu for the clicking player instead. Hold Sneak to use the marker
//! normally. Only Survival and Creative players are redirected.
//!
//! Before opening, the server replays the checks a direct use of the
//! container would run: the marker and the container are both sensibly
//! reachable (outline ray hits that skip only the marker), the player may
//! use both blocks, the container sits inside the world border, and a chest
//! is not blocked from above. Anything else falls through to vanilla.
use mod_api::{
    self, BlockFace, ClipBlock, ClipFluid, EntityInteractSpecificEvent, Flow, GameMode, Hand,
    HostSide, Hooks, InteractionResult, LevelRef, PlayerRef, RightClickBlockEvent, ServerHost,
    TriState,
};

/// Ray length for the validation rays: past any legitimate interaction, so
/// only the hit conditions can refuse.
const RAY_LENGTH: f64 = 8.0;

#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn right_click_block(&self, host: HostSide<'_>, event: &mut RightClickBlockEvent<'_>) -> Flow {
        let HostSide::Server(host) = host else {
            // Let the click through to the server; only the server redirects.
            return Flow::Continue;
        };
        let click = event.click;
        if click.hand != Hand::Main {
            return Flow::Continue;
        }
        let Some(player) = host.player(click.player) else {
            return Flow::Continue;
        };
        if !player_clear(host, player) {
            return Flow::Continue;
        }
        let Some(level) = player.level(host) else {
            return Flow::Continue;
        };
        let Some(container) = container_for_marker(host, level, click.pos) else {
            return Flow::Continue;
        };
        if !destination_clear(
            host,
            level,
            player,
            click.player_entity_id,
            click.pos,
            container,
            click.stack,
            click.hit.point,
            click.hit.face,
        ) {
            return Flow::Continue;
        }
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
        if !frame_destination_clear(host, level, player, container, normal, event.stack) {
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
    level: LevelRef,
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
fn is_container_at(host: &dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> bool {
    level
        .block_state(host, pos)
        .is_some_and(|state| is_container(&state.block().id(host)))
}

/// The full destination validation for a marker click: the marker is hit
/// through a clear outline ray, the container is hit through a ray that may
/// pass only the marker, neither ray leaves the world border, the player may
/// use both blocks, and a chest is not blocked from above. Anything the mod
/// cannot verify falls through to vanilla.
fn destination_clear(
    host: &mut dyn ServerHost,
    level: LevelRef,
    player: PlayerRef,
    player_entity: i32,
    marker: [i32; 3],
    container: [i32; 3],
    stack: Option<&mod_api::ItemStack>,
    hit_point: [f64; 3],
    hit_face: [i32; 3],
) -> bool {
    let Ok(entity_id) = u32::try_from(player_entity) else {
        return false;
    };
    let Some(eye) = host
        .entity(entity_id)
        .and_then(|entity| entity.eye_position(host))
    else {
        return false;
    };
    // The marker's outline is hit with nothing in between.
    let marker_hit = level.clip(host, eye, ray_toward(eye, hit_point), ClipBlock::Outline, ClipFluid::None);
    let Some(marker_hit) = marker_hit.filter(|hit| {
        !hit.miss && !hit.world_border && hit.block == marker && within_reach(eye, hit.location)
    }) else {
        return false;
    };
    let _ = marker_hit;
    // The container's outline is hit with nothing in between but the marker.
    let container_hit = level.clip(
        host,
        eye,
        ray_toward(eye, block_center(container)),
        ClipBlock::Outline,
        ClipFluid::None,
    );
    let Some(container_hit) = container_hit.filter(|hit| {
        !hit.miss
            && !hit.world_border
            && (hit.block == marker || hit.block == container)
            && within_reach(eye, hit.location)
    }) else {
        return false;
    };
    // The normal block interaction checks, on both blocks.
    if let Some(stack) = stack {
        let Some(marker_face) = face_of(hit_face) else {
            return false;
        };
        if player.may_use_item_at(host, marker, marker_face, stack) != Some(true) {
            return false;
        }
        if player.may_use_item_at(host, container, container_hit.face, stack) != Some(true) {
            return false;
        }
    }
    // The container sits inside the world border.
    if let Some(border) = level.world_border(host) {
        let [x, _, z] = block_center(container);
        if !(border[0] <= x && x <= border[2] && border[1] <= z && z <= border[3]) {
            return false;
        }
    }
    // A chest with a redstone conductor above it stays shut.
    if is_chest_at(host, level, container)
        && level.is_redstone_conductor(host, [container[0], container[1] + 1, container[2]])
            == Some(true)
    {
        return false;
    }
    true
}

/// The destination validation for a frame click. The frame itself was
/// range-checked with the interact packet (or the client's own pick), so this
/// covers the container: permission, border and lid, like the block path.
fn frame_destination_clear(
    host: &mut dyn ServerHost,
    level: LevelRef,
    player: PlayerRef,
    container: [i32; 3],
    facing: [i32; 3],
    stack: Option<&mod_api::ItemStack>,
) -> bool {
    if let Some(stack) = stack {
        let Some(face) = face_of(facing) else {
            return false;
        };
        if player.may_use_item_at(host, container, face, stack) != Some(true) {
            return false;
        }
    }
    if let Some(border) = level.world_border(host) {
        let [x, _, z] = block_center(container);
        if !(border[0] <= x && x <= border[2] && border[1] <= z && z <= border[3]) {
            return false;
        }
    }
    if is_chest_at(host, level, container)
        && level.is_redstone_conductor(host, [container[0], container[1] + 1, container[2]])
            == Some(true)
    {
        return false;
    }
    true
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

fn is_chest_at(host: &dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> bool {
    level.block_state(host, pos).is_some_and(|state| {
        matches!(
            state.block().id(host).as_str(),
            "minecraft:chest" | "minecraft:trapped_chest"
        )
    })
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

fn face_of(normal: [i32; 3]) -> Option<BlockFace> {
    match normal {
        [0, -1, 0] => Some(BlockFace::Down),
        [0, 1, 0] => Some(BlockFace::Up),
        [0, 0, -1] => Some(BlockFace::North),
        [0, 0, 1] => Some(BlockFace::South),
        [-1, 0, 0] => Some(BlockFace::West),
        [1, 0, 0] => Some(BlockFace::East),
        _ => None,
    }
}

fn block_center(pos: [i32; 3]) -> [f64; 3] {
    std::array::from_fn(|axis: usize| f64::from(pos[axis]) + 0.5)
}

/// A ray from `eye` toward `at`, long enough to reach any block the game
/// would let the player touch.
fn ray_toward(eye: [f64; 3], at: [f64; 3]) -> [f64; 3] {
    let delta: [f64; 3] = std::array::from_fn(|axis: usize| at[axis] - eye[axis]);
    let length = delta.iter().map(|d| d * d).sum::<f64>().sqrt();
    if length < 1e-6 {
        return eye;
    }
    std::array::from_fn(|axis: usize| eye[axis] + delta[axis] / length * RAY_LENGTH)
}

/// Whether `point` is within the game's own block reach of `eye`.
fn within_reach(eye: [f64; 3], point: [f64; 3]) -> bool {
    const REACH_SQUARED: f64 = 64.0;
    (0..3)
        .map(|axis| eye[axis] - point[axis])
        .map(|d| d * d)
        .sum::<f64>()
        <= REACH_SQUARED
}
