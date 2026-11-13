fn clickthrough_remote(game: &mut crate::game::Game<'_>) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};

    if !game.native_on(super::clickthrough::ID)
        || game
            .remote
            .as_ref()
            .is_some_and(|remote| remote.is_destroying)
        || game.hands_busy()
        || game.spectator()
        || game.remote_shift_down()
        || game.keys.held(super::Mapping::Sneak)
    {
        return Continue(());
    }
    let Some(hit) = super::clickthrough::target(game, game.remote_use_targets_entity()) else {
        return Continue(());
    };

    game.use_delay = 4;
    if let Some(swing) = game.native_interaction(1, Hand::Main) {
        if swing {
            game.swing_hand(Hand::Main, SwingAnimation::DEFAULT);
        }
        return Break(());
    }
    if game.mod_use_start() {
        return Break(());
    }

    let previous = game.forced_hit.replace(hit);
    horizon_client::interaction_input::use_on_target(&mut RemoteUseWorld(game), false);
    game.forced_hit = previous;
    Break(())
}
