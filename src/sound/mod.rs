use azalea_core::sound::CustomSound;
use azalea_protocol::packets::game::c_sound::SoundSource;
use azalea_registry::builtin::SoundEvent;
use tracing::debug;

pub mod sound_instance;
pub mod sounds;

use crate::assets::AssetIndex;

#[derive(Debug)]
pub struct PlayableSound {
    pub name: azalea_registry::Holder<SoundEvent, CustomSound>,
    pub source: SoundSource,

    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub volume: f32,
    pub pitch: f32,
    pub seed: u64,
}

pub struct SoundManager {
    assets: AssetIndex,
}

impl SoundManager {
    pub fn new(index: AssetIndex) -> Self {
        SoundManager { assets: index }
    }

    pub fn play(&self, sound: PlayableSound) {
        let name: String = match sound.name {
            azalea_registry::Holder::Reference(sound_ref) => sound_ref.to_str().to_string(),
            azalea_registry::Holder::Direct(sound_direct) => {
                sound_direct.sound_id.path().to_string()
            }
        };

        debug!("Playing sound: {}", name);
    }
}
