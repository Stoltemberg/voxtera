use super::{FILL_FRAC_TWO, Imgs};
use crate::ui::{
    fonts::IcedFonts as Fonts,
    ice::{
        Element,
        component::neat_button,
        style,
        widget::{BackgroundContainer, Image, Padding},
    },
};

use i18n::Localization;
use iced::{Align, Column, Container, Length, Row, Space, Text, TextInput, button, text_input};

const INPUT_WIDTH: u16 = 230;
const INPUT_TEXT_SIZE: u16 = 20;

/// Registration screen for the main menu
/// Follows the exact same 3-column layout as the login screen
#[derive(Default)]
pub struct Screen {
    pub email: text_input::State,
    pub username: text_input::State,
    pub password: text_input::State,
    pub confirm_password: text_input::State,

    register_button: button::State,
    back_button: button::State,

    pub registration_info: RegistrationInfo,
}

#[derive(Default, Clone)]
pub struct RegistrationInfo {
    pub email: String,
    pub username: String,
    pub password: String,
    pub confirm_password: String,
}

impl Screen {
    pub(super) fn view(
        &mut self,
        fonts: &Fonts,
        imgs: &Imgs,
        error: Option<&str>,
        i18n: &Localization,
        button_style: style::button::Style,
    ) -> Element<'_, super::Message> {
        let input_text_size = fonts.cyri.scale(INPUT_TEXT_SIZE);

        // === LEFT COLUMN: info text (same as login) ===
        let info_text = i18n.get_msg("main-login_process");
        let info_window = Container::new(
            Text::new(info_text.to_string()).size(fonts.cyri.scale(18)),
        )
        .max_width(360)
        .padding(iced::Padding { top: 10, right: 20, bottom: 60, left: 20 });

        let left_column = Container::new(info_window)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(27);

        // === CENTER COLUMN: registration form ===
        // Build input fields — same structure as LoginBanner
        let inputs = Column::with_children(vec![
            // Email field
            BackgroundContainer::new(
                Image::new(imgs.input_bg)
                    .width(Length::Units(INPUT_WIDTH))
                    .fix_aspect_ratio(),
                TextInput::new(
                    &mut self.email,
                    "Email",
                    &self.registration_info.email,
                    super::Message::RegisterEmail,
                )
                .size(input_text_size)
                .on_submit(super::Message::FocusRegisterUsername),
            )
            .padding(Padding::new().horizontal(7).top(5))
            .into(),
            // Username field
            BackgroundContainer::new(
                Image::new(imgs.input_bg)
                    .width(Length::Units(INPUT_WIDTH))
                    .fix_aspect_ratio(),
                TextInput::new(
                    &mut self.username,
                    &i18n.get_msg("main-username").to_string(),
                    &self.registration_info.username,
                    super::Message::RegisterUsername,
                )
                .size(input_text_size)
                .on_submit(super::Message::FocusRegisterPassword),
            )
            .padding(Padding::new().horizontal(7).top(5))
            .into(),
            // Password field
            BackgroundContainer::new(
                Image::new(imgs.input_bg)
                    .width(Length::Units(INPUT_WIDTH))
                    .fix_aspect_ratio(),
                TextInput::new(
                    &mut self.password,
                    &i18n.get_msg("main-password").to_string(),
                    &self.registration_info.password,
                    super::Message::RegisterPassword,
                )
                .size(input_text_size)
                .password()
                .on_submit(super::Message::FocusRegisterConfirmPassword),
            )
            .padding(Padding::new().horizontal(7).top(5))
            .into(),
            // Confirm password field
            BackgroundContainer::new(
                Image::new(imgs.input_bg)
                    .width(Length::Units(INPUT_WIDTH))
                    .fix_aspect_ratio(),
                TextInput::new(
                    &mut self.confirm_password,
                    "Confirmar Senha",
                    &self.registration_info.confirm_password,
                    super::Message::RegisterConfirmPassword,
                )
                .size(input_text_size)
                .password()
                .on_submit(super::Message::RegisterSubmit),
            )
            .padding(Padding::new().horizontal(7).top(5))
            .into(),
        ])
        .spacing(5);

        // Error message (if any)
        let error_display: Element<'_, super::Message> = if let Some(err) = error {
            Text::new(err.to_string())
                .size(fonts.cyri.scale(16))
                .color(iced::Color::from_rgb(1.0, 0.3, 0.3))
                .horizontal_alignment(iced::HorizontalAlignment::Center)
                .into()
        } else {
            Space::new(Length::Units(0), Length::Units(0)).into()
        };

        // Buttons — same style as LoginBanner
        let buttons = Column::with_children(vec![
            neat_button(
                &mut self.register_button,
                "Criar Conta",
                FILL_FRAC_TWO,
                button_style,
                Some(super::Message::RegisterSubmit),
            ),
            neat_button(
                &mut self.back_button,
                &i18n.get_msg("common-back").to_string(),
                FILL_FRAC_TWO,
                button_style,
                Some(super::Message::Back),
            ),
        ])
        .max_width(170)
        .height(Length::Units(120))
        .spacing(8);

        // Central content: inputs + error + buttons, centered
        let central_content = Column::with_children(vec![
            inputs.into(),
            Space::new(Length::Units(0), Length::Units(8)).into(),
            error_display,
            Space::new(Length::Units(0), Length::Units(8)).into(),
            buttons.into(),
        ])
        .width(Length::Fill)
        .align_items(Align::Center);

        let central_column = Container::new(central_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y();

        // === RIGHT COLUMN: version text (same as login) ===
        let version_text = format!("Voxtera {}", *common::util::DISPLAY_VERSION);
        let version_stage =
            Text::new(common::util::VELOREN_VERSION_STAGE).size(fonts.cyri.scale(22));

        let right_column = Container::new(
            Column::with_children(vec![
                Text::new(version_text)
                    .size(fonts.cyri.scale(35))
                    .color(iced::Color::from_rgb(0.2, 0.8, 0.4))
                    .horizontal_alignment(iced::HorizontalAlignment::Center)
                    .into(),
                version_stage.into(),
            ])
            .align_items(Align::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Align::End);

        // === 3-column layout matching login screen ===
        Row::with_children(vec![
            left_column.into(),
            central_column.into(),
            right_column.into(),
        ])
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(10)
        .into()
    }
}
