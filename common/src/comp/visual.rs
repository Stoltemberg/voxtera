use serde::{Deserialize, Serialize};
use specs::{Component, DenseVecStorage, DerefFlaggedStorage};
use vek::*;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LightEmitter {
    pub col: Rgb<f32>,
    pub strength: f32,
    pub flicker: f32,
    pub animated: bool,
    // (direction, +cos(beam_angle))
    pub dir: Option<(Vec3<f32>, f32)>,
}

impl Component for LightEmitter {
    type Storage = DerefFlaggedStorage<Self, DenseVecStorage<Self>>;
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LightAnimation {
    pub offset: Vec3<f32>,
    pub col: Rgb<f32>,
    pub strength: f32,
    // (direction, +cos(beam_angle))
    pub dir: Option<(Vec3<f32>, f32)>,
}

impl Component for LightAnimation {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum FrontendMarker {
    IgniteArrow,
    FreezeArrow,
    DrenchArrow,
    JoltArrow,
    Torus(f32, TorusMode),
}
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TorusMode {
    RedBlueFire,
}

impl Component for FrontendMarker {
    type Storage = DerefFlaggedStorage<Self, specs::HashMapStorage<Self>>;
}

/// Component that triggers a visual flash when an entity is hit.
/// Added to an entity when it takes damage, and removed after the duration expires.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HitFlash {
    /// Time remaining for the flash effect (in seconds)
    pub timer: f32,
    /// Color of the flash (white for normal hit, red for critical)
    pub col: Rgb<f32>,
    /// Intensity of the flash (0.0 to 1.0)
    pub intensity: f32,
    /// Whether this was a critical hit
    pub is_critical: bool,
}

impl Default for HitFlash {
    fn default() -> Self {
        Self {
            timer: 0.1,
            col: Rgb::new(1.0, 1.0, 1.0),
            intensity: 1.0,
            is_critical: false,
        }
    }
}

impl HitFlash {
    /// Create a new HitFlash for a normal hit (white flash)
    pub fn normal() -> Self {
        Self {
            timer: 0.08,
            col: Rgb::new(1.0, 1.0, 1.0),
            intensity: 0.8,
            is_critical: false,
        }
    }

    /// Create a new HitFlash for a critical hit (red flash, longer duration)
    pub fn critical() -> Self {
        Self {
            timer: 0.15,
            col: Rgb::new(1.0, 0.2, 0.2),
            intensity: 1.0,
            is_critical: true,
        }
    }

    /// Create a HitFlash with custom color (for elemental damage)
    pub fn elemental(col: Rgb<f32>) -> Self {
        Self {
            timer: 0.1,
            col,
            intensity: 0.9,
            is_critical: false,
        }
    }
}

impl Component for HitFlash {
    type Storage = DenseVecStorage<Self>;
}
