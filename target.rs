use super::{Game, Hit};

pub(super) fn target(game: &Game<'_>, entity: bool) -> Option<Hit> {
    let (eye, direction) = game.aim()?;
    let (marker, face, point) = if entity {
        let (distance, id) = game.pick_entity_hit_at()?;
        let id = u32::try_from(id?).ok()?;
        let frame = game.item_frames.iter().find(|frame| frame.id == id)?;
        (
            frame.pos,
            frame.step(),
            std::array::from_fn(|axis| eye[axis] + direction[axis] * distance),
        )
    } else {
        let hit = game.pick()?;
        let state = game.world.block(hit.pos[0], hit.pos[1], hit.pos[2]);
        let name = game.block_name(hit.pos);
        if !(name.ends_with("_wall_sign")
            || name.ends_with("_wall_hanging_sign")
            || name.ends_with("_wall_banner"))
        {
            return None;
        }
        let state = game.pack.assets.registry.state(state)?;
        let face = match state.properties.get("facing")?.as_str() {
            "north" => [0, 0, -1],
            "south" => [0, 0, 1],
            "west" => [-1, 0, 0],
            "east" => [1, 0, 0],
            _ => return None,
        };
        (hit.pos, face, hit.point)
    };

    let pos = std::array::from_fn(|axis| marker[axis] - face[axis]);
    let name = game.block_name(pos);
    if !is_container(&name) {
        return None;
    }

    // Put the packet cursor on the supporting block's face, inside its bounds.
    let point = std::array::from_fn(|axis| {
        let min = f64::from(pos[axis]);
        match face[axis] {
            -1 => min,
            1 => min + 1.0,
            _ => point[axis].clamp(min, min + 1.0),
        }
    });
    let distance_squared: f64 = (0..3).map(|axis| (eye[axis] - point[axis]).powi(2)).sum();
    (distance_squared <= game.block_reach.powi(2)).then_some(Hit { pos, face, point })
}

fn is_container(name: &str) -> bool {
    horizon_client::containers::is_chest_block(name)
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
