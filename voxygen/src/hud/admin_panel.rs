//! Admin panel widget — only visible to players with `comp::Admin` component.
//! Opened with Ctrl+Alt+F12 (default binding: F12).

use super::{Imgs, Fonts, TEXT_COLOR};
use client::Client;
use conrod_core::{
    Colorable, Positionable, Sizeable, Widget, WidgetCommon,
    widget::{self, Canvas, Text},
    widget_ids,
};
use i18n::Localization;
use specs::{Join, WorldExt};

widget_ids! {
    struct Ids {
        canvas,
        title,
        body,
    }
}

#[derive(WidgetCommon)]
pub struct AdminPanel<'a> {
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    imgs: &'a Imgs,
    fonts: &'a Fonts,
    i18n: &'a Localization,
    client: &'a Client,
}

impl<'a> AdminPanel<'a> {
    pub fn new(
        imgs: &'a Imgs,
        fonts: &'a Fonts,
        i18n: &'a Localization,
        client: &'a Client,
    ) -> Self {
        Self {
            common: widget::CommonBuilder::default(),
            imgs,
            fonts,
            i18n,
            client,
        }
    }

    fn online_player_count(&self) -> usize {
        self.client
            .state()
            .ecs()
            .read_storage::<common::comp::Player>()
            .join()
            .count()
    }
}

pub struct State {
    ids: Ids,
}

impl Widget for AdminPanel<'_> {
    type Event = ();
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State { ids: Ids::new(id_gen) }
    }

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;

        let online = self.online_player_count();
        let body = format!(
            "{}\n\n{}\n\n{}",
            self.i18n.get_msg("hud-admin-title"),
            format!("{}: {}", self.i18n.get_msg("hud-admin-players"), online),
            [
                self.i18n.get_msg("hud-admin-tp-to"),
                self.i18n.get_msg("hud-admin-tp-here"),
                self.i18n.get_msg("hud-admin-kick"),
                self.i18n.get_msg("hud-admin-mute"),
                self.i18n.get_msg("hud-admin-give-title"),
                self.i18n.get_msg("hud-admin-godmode"),
                self.i18n.get_msg("hud-admin-pvp"),
                self.i18n.get_msg("hud-admin-announce-title"),
            ].join("  |  ")
        );

        Canvas::new()
            .w(620.0)
            .h(520.0)
            .middle_of(ui.window)
            .color(conrod_core::Color::Rgba(0.05, 0.05, 0.1, 0.95))
            .set(state.ids.canvas, ui);

        Text::new(&self.i18n.get_msg("hud-admin-title"))
            .top_left_with_margins_on(state.ids.canvas, 20.0, 20.0)
            .font_size(self.fonts.cyri.scale(24))
            .color(TEXT_COLOR)
            .font_id(self.fonts.cyri.conrod_id)
            .set(state.ids.title, ui);

        Text::new(&body)
            .top_left_with_margins_on(state.ids.canvas, 60.0, 20.0)
            .font_size(self.fonts.cyri.scale(16))
            .color(TEXT_COLOR)
            .font_id(self.fonts.cyri.conrod_id)
            .set(state.ids.body, ui);
    }
}
