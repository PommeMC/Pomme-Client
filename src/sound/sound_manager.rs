use crate::sound::sounds::{Sound, SoundType};
use azalea_registry::identifier::Identifier;
use std::sync::LazyLock;

pub static EMPTY_SOUND: LazyLock<Sound> = LazyLock::new(|| {
    Sound::new(
        Identifier::new("empty"),
        1.0,
        1.0,
        1,
        SoundType::File,
        false,
        false,
        16,
    )
});
