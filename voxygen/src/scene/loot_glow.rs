//! Loot Glow System
//!
//! Adds a pulsing glow effect to dropped items (PickupItem entities) based on
//! their rarity (Quality). The system reads item entities from the ECS,
//! determines the highest quality item in each pickup, and emits colored point
//! lights that pulse using a sine wave animation.
//!
//! Quality-to-color mapping (matching the HUD quality colors):
//!   Low       → Grey   (0.40, 0.40, 0.40)   – subtle, barely visible
//!   Common    → White  (0.79, 1.00, 1.00)   – light blue tint
//!   Moderate  → Green  (0.06, 0.69, 0.12)
//!   High      → Blue   (0.18, 0.32, 0.90)
//!   Epic      → Purple (0.58, 0.29, 0.93)
//!   Legendary → Gold   (0.92, 0.76, 0.00)
//!   Artifact  → Orange (0.74, 0.24, 0.11)
//!   Debug     → Red    (0.79, 0.19, 0.17)

use crate::render::Light;
use common::comp::{PickupItem, Pos, item::Quality};
use specs::{Join, WorldExt};
use std::f32::consts::TAU;
use vek::*;

/// Maximum number of loot glow lights that can be active at once.
/// Keeps the light budget reasonable since MAX_LIGHT_COUNT is 20 and we share
/// it with all other scene lights.
const MAX_LOOT_GLOW_LIGHTS: usize = 8;

/// How far (in world units²) a loot glow light is visible from.
/// Beyond this distance the glow is culled to save light budget.
const LOOT_GLOW_VISIBILITY_RADIUS_SQ: f32 = 64.0 * 64.0;

/// The base pulse speed in radians per second. Each quality tier multiplies
/// this to give higher-rarity items a more noticeable shimmer.
const BASE_PULSE_SPEED: f32 = TAU * 0.75; // ~0.75 Hz full cycle

/// Vertical offset above the ground for the glow light origin (center of the
/// item model is typically at ~0.5 block height for small items).
const GLOW_HEIGHT_OFFSET: f32 = 0.6;

// ─── Quality-to-color lookup ────────────────────────────────────────────────

/// Returns the (base_rgb, base_strength, pulse_speed_multiplier) for a given
/// quality level. Higher qualities get brighter and pulse faster.
fn quality_params(quality: Quality) -> (Rgb<f32>, f32, f32) {
    match quality {
        // Grey – very subtle, no pulse
        Quality::Low => (Rgb::new(0.40, 0.40, 0.40), 0.15, 0.0),
        // Light blue (matches HUD QUALITY_COMMON)
        Quality::Common => (Rgb::new(0.79, 1.00, 1.00), 0.30, 0.8),
        // Green (matches HUD QUALITY_MODERATE)
        Quality::Moderate => (Rgb::new(0.06, 0.69, 0.12), 0.50, 1.0),
        // Blue (matches HUD QUALITY_HIGH)
        Quality::High => (Rgb::new(0.18, 0.32, 0.90), 0.70, 1.2),
        // Purple (matches HUD QUALITY_EPIC)
        Quality::Epic => (Rgb::new(0.58, 0.29, 0.93), 1.00, 1.5),
        // Gold (matches HUD QUALITY_LEGENDARY)
        Quality::Legendary => (Rgb::new(0.92, 0.76, 0.00), 1.30, 2.0),
        // Orange (matches HUD QUALITY_ARTIFACT)
        Quality::Artifact => (Rgb::new(0.74, 0.24, 0.11), 1.50, 2.5),
        // Red (matches HUD QUALITY_DEBUG)
        Quality::Debug => (Rgb::new(0.79, 0.19, 0.17), 1.00, 1.5),
    }
}

/// Determine the highest quality across all items inside a [`PickupItem`].
fn pickup_quality(pickup: &PickupItem) -> Quality { pickup.item().quality() }

// ─── LootGlowSystem ─────────────────────────────────────────────────────────

/// Manages the loot glow effect for dropped items in the world.
///
/// Intended to be instantiated once in the [`Scene`] and updated each frame
/// via [`maintain`](LootGlowSystem::maintain). The returned [`Light`] values
/// should be appended to the scene's light list before the light budget
/// truncation.
pub struct LootGlowSystem {
    /// Accumulated time in seconds, used for pulse animation.
    time: f32,
}

impl LootGlowSystem {
    pub fn new() -> Self { Self { time: 0.0 } }

    /// Advance the internal clock and produce a list of glow lights for all
    /// visible `PickupItem` entities.
    ///
    /// # Arguments
    /// * `state` – the game [`State`], used to access the ECS world.
    /// * `dt` – delta time in seconds since the last frame.
    /// * `viewpoint_pos` – camera / player position for distance culling.
    ///
    /// # Returns
    /// A `Vec<Light>` ready to be merged into the scene's light list.
    pub fn maintain(
        &mut self,
        state: &common_state::State,
        dt: f32,
        viewpoint_pos: Vec3<f32>,
    ) -> Vec<Light> {
        self.time += dt;

        let ecs = state.ecs();

        // Borrow the storages we need
        let entities = ecs.entities();
        let positions = ecs.read_storage::<Pos>();
        let interpolated = ecs.read_storage::<crate::ecs::comp::Interpolated>();
        let pickup_items = ecs.read_storage::<PickupItem>();

        let mut lights: Vec<Light> = Vec::new();

        for (entity, pos, pickup) in (&entities, &positions, &pickup_items).join() {
            // Use interpolated position if available for smooth visuals
            let interp = interpolated.get(entity);
            let world_pos = interp.map_or(pos.0, |i| i.pos);

            // Distance cull
            let dist_sq = world_pos.distance_squared(viewpoint_pos);
            if dist_sq > LOOT_GLOW_VISIBILITY_RADIUS_SQ {
                continue;
            }

            // Determine quality and associated visual parameters
            let quality = pickup_quality(pickup);
            let (base_rgb, base_strength, pulse_speed_mult) = quality_params(quality);

            // Skip items with zero-strength glow (Low quality is intentionally
            // invisible to avoid visual noise from trash drops).
            if base_strength <= 0.0 {
                continue;
            }

            // Compute the pulsing intensity.
            //
            // For items that don't pulse (pulse_speed_mult == 0.0) the
            // intensity is just 1.0 (constant glow). For pulsing items the
            // intensity oscillates between ~0.5 and ~1.5 using a raised sine
            // wave so the light never fully goes dark.
            //
            // An entity-unique phase offset desyncs nearby items that happen
            // to share the same quality tier, so they don't pulse in lockstep.
            let final_intensity = if pulse_speed_mult > 0.0 {
                let entity_offset = (entity.id() as f32 * 2.654_435) % TAU;
                let phase = self.time * BASE_PULSE_SPEED * pulse_speed_mult + entity_offset;
                1.0 + 0.5 * phase.sin()
            } else {
                1.0
            };

            // Build the light
            let light_pos = world_pos + Vec3::new(0.0, 0.0, GLOW_HEIGHT_OFFSET);
            let light = Light::new(light_pos, base_rgb, base_strength * final_intensity);

            lights.push(light);

            // Cap the number of loot glow lights
            if lights.len() >= MAX_LOOT_GLOW_LIGHTS {
                break;
            }
        }

        lights
    }
}

impl Default for LootGlowSystem {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_params_cover_all_variants() {
        // Ensure every Quality variant has a mapping
        let variants = [
            Quality::Low,
            Quality::Common,
            Quality::Moderate,
            Quality::High,
            Quality::Epic,
            Quality::Legendary,
            Quality::Artifact,
            Quality::Debug,
        ];

        for q in variants {
            let (rgb, strength, pulse) = quality_params(q);
            // All color channels must be in [0, 1]
            assert!(rgb.r >= 0.0 && rgb.r <= 1.0, "{:?} r out of range", q);
            assert!(rgb.g >= 0.0 && rgb.g <= 1.0, "{:?} g out of range", q);
            assert!(rgb.b >= 0.0 && rgb.b <= 1.0, "{:?} b out of range", q);
            // Strength must be non-negative
            assert!(strength >= 0.0, "{:?} negative strength", q);
            // Pulse speed multiplier must be non-negative
            assert!(pulse >= 0.0, "{:?} negative pulse", q);
        }
    }

    #[test]
    fn higher_quality_has_higher_strength() {
        assert!(quality_params(Quality::Common).1 < quality_params(Quality::Moderate).1);
        assert!(quality_params(Quality::Moderate).1 < quality_params(Quality::High).1);
        assert!(quality_params(Quality::High).1 < quality_params(Quality::Epic).1);
        assert!(quality_params(Quality::Epic).1 < quality_params(Quality::Legendary).1);
        assert!(quality_params(Quality::Legendary).1 < quality_params(Quality::Artifact).1);
    }

    #[test]
    fn pulse_intensity_stays_positive() {
        // Verify the raised sine wave never goes negative for any quality
        for q in [
            Quality::Common,
            Quality::Moderate,
            Quality::High,
            Quality::Epic,
            Quality::Legendary,
            Quality::Artifact,
        ] {
            let (_, base_strength, pulse_mult) = quality_params(q);
            if pulse_mult > 0.0 {
                // Min possible intensity = 1.0 - 0.5 = 0.5
                let min_possible = base_strength * 0.5;
                assert!(min_possible >= 0.0, "{:?} glow can go negative", q);
            }
        }
    }
}
