fn clickthrough_list(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    mods.push(crate::game::native_hooks::NativeMod {
        id: super::clickthrough::ID.into(),
        title: "ClickThrough 0.1.1".into(),
        description: "Open containers through attached item frames, signs, and banners.".into(),
        keys: Vec::new(),
    });
}
