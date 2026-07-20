//! Hit direction indicator for the HUD.
//!
//! When the player takes damage, displays a red directional indicator
//! (arc/wedge) on the edge of the screen pointing toward the damage source.
//! Indicators fade out over 500 ms and multiple hits stack independently.

use conrod_core::{
    Color, Colorable, Positionable, Sizeable, Widget, WidgetCommon,
    widget::{self, Rectangle},
    widget_ids,
};
use std::{collections::VecDeque, f32::consts::PI, time::Instant};
use vek::Vec2;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// How long (in seconds) a hit indicator stays visible before fully fading.
const FADE_DURATION_SECS: f32 = 0.5;

/// Base opacity of the indicator when first created.
const BASE_OPACITY: f32 = 0.7;

/// Width of each indicator bar in logical pixels.
const INDICATOR_WIDTH: f64 = 200.0;

/// Thickness of each indicator bar in logical pixels.
const INDICATOR_THICKNESS: f64 = 8.0;

/// How many rectangular segments compose one directional indicator arc.
/// More segments = smoother arc but more widget IDs consumed.
const ARC_SEGMENTS: usize = 7;

/// The angular span of one indicator arc in radians (~40 degrees).
const ARC_SPAN: f32 = 0.7;

/// Minimum distance from screen centre where we start placing indicators.
/// This keeps indicators at the very edge of the viewport.
const EDGE_OFFSET: f64 = 0.0;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// A single hit indicator that will be rendered on screen.
#[derive(Clone, Debug)]
pub struct HitIndicator {
    /// Angle in radians relative to the player's forward direction.
    /// 0 = straight ahead, positive = right, negative = left.
    /// Range: -PI .. PI
    pub angle: f32,
    /// Wall-clock instant when this hit was registered.
    pub created_at: Instant,
    /// Normalised severity (0.0 .. 1.0). Maps linearly to opacity.
    pub severity: f32,
}

impl HitIndicator {
    pub fn new(angle: f32, severity: f32) -> Self {
        Self {
            angle,
            created_at: Instant::now(),
            severity: severity.clamp(0.0, 1.0),
        }
    }

    /// Age in seconds since creation.
    fn age_secs(&self) -> f32 { self.created_at.elapsed().as_secs_f32() }

    /// Returns true when the indicator should no longer be displayed.
    fn is_expired(&self) -> bool { self.age_secs() >= FADE_DURATION_SECS }

    /// Current opacity, linearly interpolated from `BASE_OPACITY` to 0 over
    /// `FADE_DURATION_SECS`, further scaled by severity.
    fn opacity(&self) -> f32 {
        let fade = 1.0 - (self.age_secs() / FADE_DURATION_SECS).clamp(0.0, 1.0);
        BASE_OPACITY * fade * self.severity
    }
}

// ---------------------------------------------------------------------------
// conrod widget
// ---------------------------------------------------------------------------

widget_ids! {
    struct Ids {
        // Dynamically sized: ARC_SEGMENTS widget IDs per active indicator.
        segments[],
    }
}

/// The conrod widget that renders all active hit-direction indicators.
#[derive(WidgetCommon)]
pub struct HitDirectionIndicator<'a> {
    /// Active indicators. The widget drains expired entries on each update.
    indicators: &'a mut VecDeque<HitIndicator>,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

impl<'a> HitDirectionIndicator<'a> {
    pub fn new(indicators: &'a mut VecDeque<HitIndicator>) -> Self {
        Self {
            indicators,
            common: widget::CommonBuilder::default(),
        }
    }
}

/// Persistent state kept across frames.
pub struct State {
    ids: Ids,
}

impl Widget for HitDirectionIndicator<'_> {
    type Event = ();
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;

        // Prune expired indicators.
        self.indicators.retain(|ind| !ind.is_expired());

        let active = self.indicators.len();
        let needed_ids = active * ARC_SEGMENTS;

        // Resize the dynamic ID list if necessary.
        if state.ids.segments.len() != needed_ids {
            state.update(|s| {
                s.ids
                    .segments
                    .resize(needed_ids, &mut ui.widget_id_generator());
            });
        }

        let win_w = ui.win_w;
        let win_h = ui.win_h;
        // Half-extents of the window (conrod window origin = centre).
        let half_w = win_w * 0.5;
        let half_h = win_h * 0.5;

        // Draw each active indicator.
        for (i, indicator) in self.indicators.iter().enumerate() {
            let opacity = indicator.opacity();
            if opacity < 0.01 {
                continue;
            }

            let color = Color::Rgba(0.85, 0.08, 0.08, opacity);

            // The indicator angle tells us which screen edge to paint on.
            // We break the full circle into four quadrants and place the arc
            // on the corresponding edge.
            let angle = indicator.angle; // -PI..PI

            for seg in 0..ARC_SEGMENTS {
                let id_idx = i * ARC_SEGMENTS + seg;
                let seg_id = state.ids.segments[id_idx];

                // Fractional position of this segment within the arc [0, 1].
                let t = if ARC_SEGMENTS <= 1 {
                    0.5
                } else {
                    seg as f32 / (ARC_SEGMENTS - 1) as f32
                };

                // Sub-angle for this segment relative to the arc centre.
                let sub_angle = angle + (t - 0.5) * ARC_SPAN;

                // Normalise to -PI..PI.
                let norm_angle = ((sub_angle + PI) % (2.0 * PI)) - PI;

                // Compute a position on the screen edge.
                // Map the angle to a point on the screen perimeter.
                let (x, y, w, h) = edge_placement(norm_angle, half_w, half_h);

                Rectangle::fill([w, h])
                    .x_y(x, y)
                    .color(color)
                    .parent(ui.window)
                    .set(seg_id, ui);
            }
        }
    }
}

/// Given an angle relative to the player's forward direction, compute the
/// position and dimensions of a small rectangle sitting on the screen edge.
///
/// Returns `(x, y, width, height)` in conrod coordinates (origin at screen
/// centre, +x right, +y up).
fn edge_placement(angle: f32, half_w: f64, half_h: f64) -> (f64, f64, f64, f64) {
    let (sin_a, cos_a) = angle.sin_cos();

    // We project the direction onto the screen edges.  The largest absolute
    // component determines which edge the indicator clings to.
    let abs_cos = cos_a.abs() as f64;
    let abs_sin = sin_a.abs() as f64;

    if abs_cos * half_w > abs_sin * half_h {
        // Hits top or bottom edge (forward / backward relative to player).
        if cos_a > 0.0 {
            // Top edge (ahead of player).
            let edge_y = half_h - EDGE_OFFSET;
            let edge_x = (sin_a as f64) * half_w;
            (
                edge_x,
                edge_y - INDICATOR_THICKNESS * 0.5,
                INDICATOR_WIDTH,
                INDICATOR_THICKNESS,
            )
        } else {
            // Bottom edge (behind player).
            let edge_y = -half_h + EDGE_OFFSET;
            let edge_x = -(sin_a as f64) * half_w;
            (
                edge_x,
                edge_y + INDICATOR_THICKNESS * 0.5,
                INDICATOR_WIDTH,
                INDICATOR_THICKNESS,
            )
        }
    } else {
        // Hits left or right edge.
        if sin_a > 0.0 {
            // Right edge.
            let edge_x = half_w - EDGE_OFFSET;
            let edge_y = (cos_a as f64) * half_h;
            (
                edge_x - INDICATOR_THICKNESS * 0.5,
                edge_y,
                INDICATOR_THICKNESS,
                INDICATOR_WIDTH,
            )
        } else {
            // Left edge.
            let edge_x = -half_w + EDGE_OFFSET;
            let edge_y = -(cos_a as f64) * half_h;
            (
                edge_x + INDICATOR_THICKNESS * 0.5,
                edge_y,
                INDICATOR_THICKNESS,
                INDICATOR_WIDTH,
            )
        }
    }
}

// ---------------------------------------------------------------------------
// Public helpers (used by mod.rs integration code)
// ---------------------------------------------------------------------------

/// Register a new incoming-damage event.
///
/// `damage_source_pos` – world position of the entity / projectile that dealt
///   the damage.
/// `player_pos`       – world position of the player.
/// `player_forward`   – the player's horizontal look direction (2D, already
///   normalised in the XY plane).
/// `severity`         – 0.0 .. 1.0 normalised damage magnitude.
pub fn register_hit(
    indicators: &mut VecDeque<HitIndicator>,
    damage_source_pos: Vec2<f32>,
    player_pos: Vec2<f32>,
    player_forward: Vec2<f32>,
    severity: f32,
) {
    let delta = damage_source_pos - player_pos;
    let dist_sq = delta.magnitude_squared();
    if dist_sq < 0.001 {
        // Source is right on top of the player – pick a random-ish direction.
        indicators.push_back(HitIndicator::new(0.0, severity));
        return;
    }

    let forward = player_forward.try_normalized().unwrap_or(Vec2::unit_y());
    let right = Vec2::new(forward.y, -forward.x);

    let dir = delta / dist_sq.sqrt();
    let fwd_dot = dir.dot(forward);
    let rgt_dot = dir.dot(right);

    // Angle from forward axis: positive = right, negative = left.
    let angle = rgt_dot.atan2(fwd_dot);

    indicators.push_back(HitIndicator::new(angle, severity));
}

/// Convenience: register a hit from a 3D world-space delta (e.g.
/// `attacker_pos - player_pos`) using the camera's horizontal forward vector.
pub fn register_hit_3d(
    indicators: &mut VecDeque<HitIndicator>,
    delta_3d: Vec2<f32>,
    player_forward_xy: Vec2<f32>,
    severity: f32,
) {
    let dist_sq = delta_3d.magnitude_squared();
    if dist_sq < 0.001 {
        indicators.push_back(HitIndicator::new(0.0, severity));
        return;
    }

    let forward = player_forward_xy.try_normalized().unwrap_or(Vec2::unit_y());
    let right = Vec2::new(forward.y, -forward.x);

    let dir = delta_3d / dist_sq.sqrt();
    let fwd_dot = dir.dot(forward);
    let rgt_dot = dir.dot(right);

    let angle = rgt_dot.atan2(fwd_dot);
    indicators.push_back(HitIndicator::new(angle, severity));
}
