use crate::{
    sound::{
        SoundManager,
        sounds::{Sound, SoundSource, WeighedSoundEvents},
    },
    util::rng::JavaRng,
};

pub trait SoundInstance {
    // fn identifier(&self) -> Identifier;

    fn resolve(&self, sound_manager: &SoundManager) -> Option<WeighedSoundEvents>;

    fn sound(&self) -> Option<Sound>;

    fn source(&self) -> SoundSource;

    fn is_looping(&self) -> bool;

    fn is_relative(&self) -> bool;

    fn delay(&self) -> i32;

    fn volume(&self) -> f32;

    fn pitch(&self) -> f32;

    fn x(&self) -> f64;

    fn y(&self) -> f64;

    fn z(&self) -> f64;

    fn attenuation(&self) -> Attenuation;

    fn can_start_silent(&self) -> bool {
        false
    }

    fn can_play_sound(&self) -> bool {
        true
    }

    fn create_unseeded_random() -> JavaRng {
        JavaRng::new_from_random_seed()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Attenuation {
    None,
    Linear,
}
