//! HUD widget showing the player's premium currency (Cristais) balance.
//!
//! Renders a small pill in the top-right corner of the HUD showing
//! the current Cristais balance. The balance is read each frame from
//! the player's `PremiumCurrency` component on the client ECS, which
//! is kept up to date by the `ServerGeneral::CurrencyChange` handler
//! in the client.
//!
//! This widget is purely presentational — no interaction.

use conrod_core::{
    Color, Colorable, Positionable, Widget, WidgetCommon,
    widget::{self, Rectangle, Text},
    widget_ids,
};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Label text shown next to the balance. Localised at the call site
/// (HUD has access to the i18n handle). We keep a non-localised
/// fallback here for clarity.
pub const CURRENCY_LABEL: &str = "\u{272A}"; // ✪ — placeholder glyph
/// Base alpha of the background pill.
pub const PILL_BG_COLOR: Color = Color::Rgba(0.0, 0.0, 0.0, 0.6);
/// Text colour for the balance number.
pub const PILL_TEXT_COLOR: Color = Color::Rgba(1.0, 0.84, 0.36, 1.0); // gold-ish
/// Pill width (logical pixels).
pub const PILL_WIDTH: f64 = 90.0;
/// Pill height (logical pixels).
pub const PILL_HEIGHT: f64 = 22.0;
/// Margin from the top-right corner of the window.
pub const TOP_MARGIN: f64 = 30.0;
/// Margin from the right edge of the window.
pub const RIGHT_MARGIN: f64 = 5.0;

// ---------------------------------------------------------------------------
// conrod widget
// ---------------------------------------------------------------------------

widget_ids! {
    struct Ids {
        bg,
        label,
    }
}

/// The conrod widget that renders the Cristais balance pill.
#[derive(WidgetCommon)]
pub struct CurrencyDisplay<'a> {
    /// Current balance to display. 0 is rendered as "0".
    balance: u32,
    /// Font used to render the balance number.
    fonts: &'a conrod_core::text::font::Id,
    /// Font size to render the balance number at.
    font_size: u32,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

impl<'a> CurrencyDisplay<'a> {
    pub fn new(balance: u32, fonts: &'a conrod_core::text::font::Id, font_size: u32) -> Self {
        Self {
            balance,
            fonts,
            font_size,
            common: widget::CommonBuilder::default(),
        }
    }
}

pub struct State {
    ids: Ids,
}

impl Widget for CurrencyDisplay<'_> {
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
        let widget::UpdateArgs { id, state, ui, .. } = args;

        // Background pill
        Rectangle::fill_with([PILL_WIDTH, PILL_HEIGHT], 1.0, PILL_BG_COLOR)
            .x_y(0.0, 0.0)
            .parent(id)
            .depth(1.0)
            .set(state.ids.bg, ui);

        // Balance text (right-aligned to leave room for the ✪ label)
        let text = format!("{} {}", CURRENCY_LABEL, self.balance);
        Text::new(&text)
            .font_id(self.fonts)
            .font_size(self.font_size)
            .color(PILL_TEXT_COLOR)
            .x_y(0.0, 0.0)
            .parent(id)
            .depth(2.0)
            .set(state.ids.label, ui);
    }
}