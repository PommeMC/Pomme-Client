pub mod interaction;
pub mod inventory;
pub mod menu_click;
pub mod tab_list;

use glam::{dvec2, dvec3};
use inventory::Inventory;

use crate::entity::components::{LookDirection, Position, Velocity};
use crate::world::block::{FluidKind, fluid};

pub const MAX_AIR_SUPPLY: i32 = 300;
pub const STANDING_HEIGHT: f64 = 1.8;
pub const CROUCH_HEIGHT: f64 = 1.5;
pub const STANDING_EYE_HEIGHT: f64 = 1.62;
pub const CROUCH_EYE_HEIGHT: f64 = 1.27;
const DROWN_DAMAGE_THRESHOLD: i32 = -20;
const DROWN_DAMAGE: f32 = 2.0;
const AIR_RECOVERY_RATE: i32 = 4;

// TODO: migrate the remaining raw `game_mode == N` checks to shared constants
// or an enum.
/// Matches vanilla GameType.isSurvival(): Survival (0) or Adventure (2).
pub fn is_survival(game_mode: u8) -> bool {
    game_mode == 0 || game_mode == 2
}

/// Matches vanilla GameType.isCreative(): Creative (1).
pub fn is_creative(game_mode: u8) -> bool {
    game_mode == 1
}

/// Spectator (3).
pub fn is_spectator(game_mode: u8) -> bool {
    game_mode == 3
}

fn is_water_block(state: azalea_block::BlockState) -> bool {
    fluid(state).kind == FluidKind::Water
}

pub struct LocalPlayer {
    pub position: Position,
    pub prev_position: Position,
    pub velocity: Velocity,
    pub look_dir: LookDirection,
    pub prev_look_dir: LookDirection,
    pub on_ground: bool,
    pub health: f32,
    pub death_time: u32,
    pub absorption: f32,
    pub max_health: f32,
    pub food: u32,
    pub armor: u32,
    pub saturation: f32,
    pub inventory: Inventory,
    pub sprinting: bool,
    pub crouching: bool,
    // TODO: remaining Abilities fields - invulnerable, instabuild, may_build
    pub flying: bool,
    pub may_fly: bool,
    pub fly_speed: f32,
    pub walk_speed: f32,
    pub jump_trigger_time: u32,
    pub no_jump_delay: u32,
    pub was_jump_pressed: bool,
    /// Vanilla `jumpRidingTicks`: ride-jump charge ticks; negative counts up
    /// through the post-jump refractory window.
    pub jump_riding_ticks: i32,
    /// Vanilla `jumpRidingScale`: jump-bar fill, 0..=1.
    pub jump_riding_scale: f32,
    /// Vanilla `onUpdateAbilities`: a locally toggled `flying` still has to be
    /// reported to the server via `ServerboundPlayerAbilities`.
    pub abilities_dirty: bool,
    pub eye_height: f64,
    pub prev_eye_height: f64,
    pub walk_dist: f32,
    pub prev_walk_dist: f32,
    pub bob: f32,
    pub prev_bob: f32,
    pub horizontal_collision: bool,
    pub sprint_toggle_timer: u32,
    pub was_forward_pressed: bool,
    pub in_water: bool,
    /// Vanilla `getFluidHeight(WATER)`: water surface height above the feet.
    pub fluid_height: f64,
    pub eyes_in_water: bool,
    pub swimming: bool,
    pub air_supply: i32,
    /// Vanilla LocalPlayer.portalEffectIntensity: drives the full-screen
    /// portal overlay while standing in a nether portal.
    pub portal_effect_intensity: f32,
    pub prev_portal_effect_intensity: f32,
    /// Vanilla LivingEntity SLEEPING_POS metadata: Some while in a bed.
    pub sleeping_pos: Option<azalea_core::position::BlockPos>,
    /// Vanilla Player.sleepCounter: drives the sleep overlay fade.
    pub sleep_counter: u32,
    pub game_mode: u8,
    pub score: i32,
    pub entity_id: i32,
    pub experience_level: i32,
    pub experience_progress: f32,
    pub effects: crate::mob_effect::ActiveMobEffects,
}

impl LocalPlayer {
    pub fn new() -> Self {
        Self {
            position: Position::default(),
            prev_position: Position::default(),
            velocity: Velocity::default(),
            look_dir: LookDirection::default(),
            prev_look_dir: LookDirection::default(),
            on_ground: false,
            health: 20.0,
            death_time: 0,
            absorption: 0.0,
            max_health: 20.0,
            food: 20,
            armor: 0,
            saturation: 5.0,
            inventory: Inventory::new(),
            sprinting: false,
            crouching: false,
            flying: false,
            may_fly: false,
            fly_speed: 0.05,
            walk_speed: 0.1,
            jump_trigger_time: 0,
            no_jump_delay: 0,
            was_jump_pressed: false,
            jump_riding_ticks: 0,
            jump_riding_scale: 0.0,
            abilities_dirty: false,
            eye_height: STANDING_EYE_HEIGHT,
            prev_eye_height: STANDING_EYE_HEIGHT,
            walk_dist: 0.0,
            prev_walk_dist: 0.0,
            bob: 0.0,
            prev_bob: 0.0,
            horizontal_collision: false,
            sprint_toggle_timer: 0,
            was_forward_pressed: false,
            in_water: false,
            fluid_height: 0.0,
            eyes_in_water: false,
            swimming: false,
            air_supply: MAX_AIR_SUPPLY,
            portal_effect_intensity: 0.0,
            prev_portal_effect_intensity: 0.0,
            sleeping_pos: None,
            sleep_counter: 0,
            game_mode: 0,
            score: 0,
            entity_id: -1,
            experience_level: 0,
            experience_progress: 0.0,
            effects: crate::mob_effect::ActiveMobEffects::default(),
        }
    }

    pub fn reset_death_time(&mut self) {
        self.death_time = 0;
    }

    pub fn snapshot_render_state(&mut self) {
        self.prev_position = self.position;
        self.prev_look_dir = self.look_dir;
        self.prev_eye_height = self.eye_height;
    }

    pub fn tick_death(&mut self) {
        self.death_time = (self.death_time + 1).min(20);
    }

    pub fn death_animation_finished(&self) -> bool {
        self.death_time >= 20
    }

    pub fn height(&self) -> f64 {
        if self.crouching {
            CROUCH_HEIGHT
        } else {
            STANDING_HEIGHT
        }
    }

    pub fn target_eye_height(&self) -> f64 {
        if self.crouching {
            CROUCH_EYE_HEIGHT
        } else {
            STANDING_EYE_HEIGHT
        }
    }

    pub fn tick_eye_height(&mut self) {
        self.prev_eye_height = self.eye_height;
        self.eye_height += (self.target_eye_height() - self.eye_height) * 0.5;
    }

    /// Accumulates walk distance and a smoothed bob amplitude for view bobbing,
    /// mirroring vanilla `AbstractClientPlayer.updateBob`.
    pub fn tick_bob(&mut self, dx: f64, dz: f64, dead: bool) {
        self.prev_walk_dist = self.walk_dist;
        // Vanilla LocalPlayer.move: addWalkedDistance(len * 0.6).
        self.walk_dist += dvec2(dx, dz).length() as f32 * 0.6;
        // updateBob's target is horizontal speed, not the walk delta.
        let target = if !dead && self.on_ground && !self.swimming {
            (dvec2(self.velocity.x, self.velocity.z).length() as f32).min(0.1)
        } else {
            0.0
        };
        self.prev_bob = self.bob;
        self.bob += (target - self.bob) * 0.4;
    }

    pub fn prev_eye_pos(&self) -> Position {
        self.prev_position + dvec3(0.0, self.prev_eye_height, 0.0)
    }

    pub fn eye_pos(&self) -> Position {
        self.position + dvec3(0.0, self.eye_height, 0.0)
    }

    // TODO: OXYGEN_BONUS attribute - chance to skip air loss per tick
    pub fn tick_air_supply(&mut self) {
        if self.eyes_in_water {
            self.air_supply -= 1;
            if self.air_supply <= DROWN_DAMAGE_THRESHOLD {
                self.air_supply = 0;
                self.health = (self.health - DROWN_DAMAGE).max(0.0);
            }
        } else if self.air_supply < MAX_AIR_SUPPLY {
            self.air_supply = (self.air_supply + AIR_RECOVERY_RATE).min(MAX_AIR_SUPPLY);
        }
    }

    pub fn update_water_state(&mut self, chunks: &crate::world::chunk::ChunkStore) {
        let half_w = 0.3;
        let height = self.height();
        let eye_height = self.target_eye_height();

        // Vanilla `EntityFluidInteraction.update`: scan the bounding box
        // deflated by 0.001; a block's fluid column is `amount / 9` of a
        // block, or a full block when more water sits directly above.
        const MARGIN: f64 = 0.001;
        let feet_y = self.position.y;
        let x0 = (self.position.x - half_w + MARGIN).floor() as i32;
        let x1 = (self.position.x + half_w - MARGIN).ceil() as i32 - 1;
        let y0 = (feet_y + MARGIN).floor() as i32;
        let y1 = (feet_y + height - MARGIN).ceil() as i32 - 1;
        let z0 = (self.position.z - half_w + MARGIN).floor() as i32;
        let z1 = (self.position.z + half_w - MARGIN).ceil() as i32 - 1;

        let mut fluid_height = 0.0f64;
        for bx in x0..=x1 {
            for by in y0..=y1 {
                for bz in z0..=z1 {
                    let f = fluid(chunks.get_block_state(bx, by, bz));
                    if f.kind != FluidKind::Water {
                        continue;
                    }
                    let block_height = if is_water_block(chunks.get_block_state(bx, by + 1, bz)) {
                        1.0
                    } else {
                        // f32 to match vanilla's float math.
                        f64::from(f.amount as f32 / 9.0)
                    };
                    let fluid_top = f64::from(by) + block_height;
                    if fluid_top >= feet_y + MARGIN {
                        fluid_height = fluid_height.max(fluid_top - feet_y);
                    }
                }
            }
        }

        let eye_y = (self.position.y + eye_height).floor() as i32;
        let eye_x = self.position.x.floor() as i32;
        let eye_z = self.position.z.floor() as i32;

        self.fluid_height = fluid_height;
        // Vanilla `wasTouchingWater` is exactly "fluid height > 0".
        self.in_water = fluid_height > 0.0;
        self.eyes_in_water = is_water_block(chunks.get_block_state(eye_x, eye_y, eye_z));
        self.swimming = self.sprinting && self.in_water && self.eyes_in_water;
    }

    /// Vanilla Entity.checkInsideBlocks: a nether portal counts as entered when
    /// the bounding box deflated by 1.0E-5 overlaps its (full) block cell.
    pub fn is_inside_nether_portal(&self, chunks: &crate::world::chunk::ChunkStore) -> bool {
        const MARGIN: f64 = 1.0e-5;
        let half_w = 0.3;
        let x0 = (self.position.x - half_w + MARGIN).floor() as i32;
        let x1 = (self.position.x + half_w - MARGIN).ceil() as i32 - 1;
        let y0 = (self.position.y + MARGIN).floor() as i32;
        let y1 = (self.position.y + self.height() - MARGIN).ceil() as i32 - 1;
        let z0 = (self.position.z - half_w + MARGIN).floor() as i32;
        let z1 = (self.position.z + half_w - MARGIN).ceil() as i32 - 1;

        for bx in x0..=x1 {
            for by in y0..=y1 {
                for bz in z0..=z1 {
                    if crate::world::block::block_id(chunks.get_block_state(bx, by, bz))
                        == "nether_portal"
                    {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Vanilla LocalPlayer.handlePortalTransitionEffect: the overlay ramps up
    /// over 80 ticks inside a portal and decays 4x as fast outside. Returns
    /// true when the rise starts from zero (vanilla plays PORTAL_TRIGGER).
    /// TODO: vanilla also force-closes portal-disallowed screens while rising.
    pub fn tick_portal_effect(&mut self, inside_portal: bool) -> bool {
        self.prev_portal_effect_intensity = self.portal_effect_intensity;
        let mut step = 0.0;
        let mut triggered = false;
        if inside_portal {
            triggered = self.portal_effect_intensity == 0.0;
            step = 0.0125;
        } else if self.portal_effect_intensity > 0.0 {
            step = -0.05;
        }
        self.portal_effect_intensity = (self.portal_effect_intensity + step).clamp(0.0, 1.0);
        triggered
    }

    /// Vanilla LivingEntity.isSleeping: getSleepingPos().isPresent().
    pub fn is_sleeping(&self) -> bool {
        self.sleeping_pos.is_some()
    }

    /// Vanilla Player.tick sleep-counter branch: ramps to 100 while sleeping,
    /// then runs 100..110 after waking and resets to 0.
    pub fn tick_sleep(&mut self) {
        if self.is_sleeping() {
            self.sleep_counter = (self.sleep_counter + 1).min(100);
        } else if self.sleep_counter > 0 {
            self.sleep_counter += 1;
            if self.sleep_counter >= 110 {
                self.sleep_counter = 0;
            }
        }
    }

    /// Vanilla client stopSleepInBed(false, _): the counter jumps to 100 so
    /// the fade-out runs from full even on an early wake-up.
    pub fn wake_up(&mut self) {
        self.sleeping_pos = None;
        self.sleep_counter = 100;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_player_is_removed_at_vanilla_death_tick() {
        let mut player = LocalPlayer::new();
        player.death_time = 18;

        player.tick_death();
        assert_eq!(player.death_time, 19);
        assert!(
            !player.death_animation_finished(),
            "vanilla keeps the local player renderable through death tick 19"
        );

        player.tick_death();
        assert_eq!(player.death_time, 20);
        assert!(
            player.death_animation_finished(),
            "vanilla removes LocalPlayer exactly when deathTime reaches 20"
        );

        player.tick_death();
        assert_eq!(
            player.death_time, 20,
            "removed local players no longer advance their death clock"
        );
    }

    #[test]
    fn dead_bob_advances_previous_state_and_decays() {
        let mut player = LocalPlayer::new();
        player.walk_dist = 3.25;
        player.prev_walk_dist = 2.75;
        player.bob = 0.1;
        player.prev_bob = 0.04;
        player.velocity = crate::entity::components::Velocity::new(0.2, 0.0, 0.0);
        player.on_ground = true;

        player.tick_bob(0.0, 0.0, true);

        assert_eq!(
            player.prev_walk_dist, 3.25,
            "dead ticks must advance the walk interpolation endpoint"
        );
        assert_eq!(
            player.walk_dist, 3.25,
            "dead ticks must not add walked distance"
        );
        assert_eq!(
            player.prev_bob, 0.1,
            "dead ticks must advance the bob interpolation endpoint"
        );
        assert!(
            (player.bob - 0.06).abs() < 1e-6,
            "vanilla dead-player bob decays 40% toward zero per tick"
        );
    }

    #[test]
    fn snapshot_render_state_clears_stale_interpolation_endpoints() {
        let mut player = LocalPlayer::new();
        player.position = dvec3(10.0, 64.0, -3.0).into();
        player.prev_position = dvec3(9.5, 63.8, -3.0).into();
        player.look_dir = LookDirection::new(35.0, -12.0);
        player.prev_look_dir = LookDirection::new(5.0, 8.0);
        player.eye_height = 1.4;
        player.prev_eye_height = 1.62;

        player.snapshot_render_state();

        assert_eq!(
            player.prev_position, player.position,
            "position interpolation must not replay a prior tick"
        );
        assert_eq!(
            player.prev_look_dir, player.look_dir,
            "look interpolation must start from the current tick state"
        );
        assert_eq!(
            player.prev_eye_height, player.eye_height,
            "eye interpolation must not reuse stale state"
        );
    }
}
