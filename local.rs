fn clickthrough_local(
    game: &mut crate::game::Game<'_>,
) -> std::ops::ControlFlow<Result<Option<Hand>>> {
    use std::ops::ControlFlow::{Break, Continue};

    if game.guest_use.is_some()
        || game.spectator()
        || game.keys.held(Mapping::Sneak)
        || (game.riding.is_some()
            && game.held().is_some_and(|stack| {
                matches!(
                    &*stack.item,
                    "minecraft:carrot_on_a_stick" | "minecraft:warped_fungus_on_a_stick"
                )
            }))
    {
        return Continue(());
    }
    let Some(hit) = super::clickthrough::target(game, game.entity_targeted()) else {
        return Continue(());
    };
    for hand in Hand::BOTH {
        match game.with_hand(hand, |game| game.use_hand(Some(&hit))) {
            Ok(true) => return Break(Ok(Some(hand))),
            Ok(false) => {}
            Err(error) => return Break(Err(error)),
        }
    }
    Break(Ok(None))
}
