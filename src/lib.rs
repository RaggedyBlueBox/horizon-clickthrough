//! ClickThrough as a sandboxed WebAssembly mod.
//!
//! Right-click a wall sign, wall hanging sign, or wall banner attached to a
//! container to open the container, on either logical side: the server hook
//! cancels the marker's own use and opens the container's menu for the
//! clicking player instead. Hold your Sneak key to use the marker normally.
//! Only Survival and Creative players are redirected.
//!
//! Before opening, the server replays a direct use's checks against the
//! destination container itself: an outline ray must reach the marker, then
//! a second ray from just past the marker must hit the container (a marker
//! hit never counts as the destination hit), the player must be within
//! vanilla reach and may use both blocks, the container must sit inside the
//! world border, and a chest with a redstone conductor above it stays shut.
//! Anything the mod cannot verify falls through to vanilla.
//!
//! Item frames are deliberately not redirected: the entity hook carries no
//! player handle and the mod API exposes no way to resolve the clicking
//! player's eye position on that path, so the destination ray the container
//! requires cannot be built. Frame clicks keep vanilla behavior.
use mod_api::{
    self, BlockFace, ClipBlock, ClipFluid, Flow, GameMode, Hand, HostSide, Hooks, InteractionResult,
    LevelRef, PlayerRef, RightClickBlockEvent, ServerHost, TriState,
};

/// Ray length for the validation rays: past any legitimate interaction, so
/// only the hit conditions can refuse.
const RAY_LENGTH: f64 = 8.0;

/// Reach mirroring the engine's own use validation: 5.0 in Creative, 4.5
/// otherwise, plus the engine's own 1.0 tolerance.
fn reach_for(game_mode: Option<GameMode>) -> f64 {
    if game_mode == Some(GameMode::Creative) {
        5.0 + 1.0
    } else {
        4.5 + 1.0
    }
}

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
/// through a clear outline ray, then a second ray starting just past the
/// marker must hit the container itself (a marker hit never counts), the
/// player is within vanilla reach of the destination, may use both blocks
/// (held stack or empty hand alike), the container sits inside the world
/// border, and a chest with a redstone conductor above it stays shut.
/// Anything the mod cannot verify falls through to vanilla.
#[allow(clippy::too_many_arguments)]
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
    let reach = reach_for(player.game_mode(host));
    // The marker's outline is hit with nothing in between.
    let Some(marker_hit) = level.clip(
        host,
        eye,
        ray_toward(eye, hit_point),
        ClipBlock::Outline,
        ClipFluid::None,
    ) else {
        return false;
    };
    if marker_hit.miss
        || marker_hit.world_border
        || marker_hit.block != marker
        || !within_reach(eye, marker_hit.location, reach)
    {
        return false;
    }
    // The container's outline is hit by a ray that starts just past the
    // marker: only the selected marker is skipped.
    let step = unit_toward(block_center(marker), block_center(container));
    let past_marker: [f64; 3] = [
        block_center(marker)[0] + step[0] * 0.51,
        block_center(marker)[1] + step[1] * 0.51,
        block_center(marker)[2] + step[2] * 0.51,
    ];
    let Some(container_hit) = level.clip(
        host,
        past_marker,
        ray_toward(past_marker, block_center(container)),
        ClipBlock::Outline,
        ClipFluid::None,
    ) else {
        return false;
    };
    if container_hit.miss
        || container_hit.world_border
        || container_hit.block != container
        || !within_reach(eye, container_hit.location, reach)
    {
        return false;
    }
    // The normal block interaction checks, on both blocks, held stack or
    // empty hand alike.
    let Some(marker_face) = face_of(hit_face) else {
        return false;
    };
    if !may_use(host, player, marker, marker_face, stack) {
        return false;
    }
    if !may_use(host, player, container, container_hit.face, stack) {
        return false;
    }
    // The container sits inside the world border.
    if let Some(border) = level.world_border(host) {
        let [x, _, z] = block_center(container);
        if !(border[0] <= x && x <= border[2] && border[1] <= z && z <= border[3]) {
            return false;
        }
    }
    // A chest with a redstone conductor above it stays shut. (A cat sitting
    // on the chest cannot be seen through the mod API: no entity enumeration
    // or sitting query exists, so that case falls through to vanilla, which
    // also refuses it.)
    if is_chest_at(host, level, container)
        && level.is_redstone_conductor(host, [container[0], container[1] + 1, container[2]])
            == Some(true)
    {
        return false;
    }
    true
}

/// Whether the player may use the block at `pos` clicked on `face`: the
/// normal interaction check, with a held stack or an empty hand (which
/// falls back to the player's build permission, matching what an empty
/// use checks).
fn may_use(
    host: &dyn ServerHost,
    player: PlayerRef,
    pos: [i32; 3],
    face: BlockFace,
    stack: Option<&mod_api::ItemStack>,
) -> bool {
    match stack {
        Some(stack) => player.may_use_item_at(host, pos, face, stack) == Some(true),
        None => player.may_build(host) == Some(true),
    }
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

/// A unit step from `from` toward `at`, or zero when they coincide.
fn unit_toward(from: [f64; 3], at: [f64; 3]) -> [f64; 3] {
    let delta: [f64; 3] = std::array::from_fn(|axis: usize| at[axis] - from[axis]);
    let length = delta.iter().map(|d| d * d).sum::<f64>().sqrt();
    if length < 1e-6 {
        return [0.0; 3];
    }
    std::array::from_fn(|axis: usize| delta[axis] / length)
}

/// A ray from `eye` toward `at`, long enough to reach any block the game
/// would let the player touch.
fn ray_toward(eye: [f64; 3], at: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|axis: usize| eye[axis] + unit_toward(eye, at)[axis] * RAY_LENGTH)
}

/// Whether `point` is within `reach` of `eye`.
fn within_reach(eye: [f64; 3], point: [f64; 3], reach: f64) -> bool {
    (0..3)
        .map(|axis| eye[axis] - point[axis])
        .map(|d| d * d)
        .sum::<f64>()
        <= reach * reach
}
