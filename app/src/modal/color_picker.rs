use std::fmt::format;

use iced::{
    border, color, widget::{button, column, container, horizontal_space, row, text, text_input, vertical_space}, Alignment, Background, Color, Element, Font, Length::Fill, Padding, Shadow, Theme, Vector
};

use crate::{app::Message, widget::color_picker};

fn style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(color!(0x383838, 1.))),
        border: border::rounded(9.0),
        ..Default::default()
    }
}

pub fn new<'a>(color: &Color) -> Element<'a, Message> {
    let color_hex= format!("#{:0>2X}{:0>2X}{:0>2X}", (color.r * 255.0) as u8, (color.g * 255.0) as u8, (color.b * 255.0) as u8);
    container(
        column!(
            text("Select a color"),
            vertical_space().height(10),
            color_picker(color),
            vertical_space().height(9),
            text_input("Enter the custom color hex code", &color_hex)
                        // .on_input(label_edited)
                        .font(Font::with_name("Noto Sans"))
                        .size(12)
                        .padding(Padding::new(4.0).left(10))
                        .style(|theme: &Theme, _| {
                            let palette = theme.extended_palette();
                            text_input::Style {
                                background: Background::Color(color!(0x505050, 1.0)),
                                value: color!(0xF1F1F1, 1.),
                                border: border::rounded(5.0),
                                icon: palette.background.strong.color,
                                placeholder: palette.background.strong.color,
                                selection: palette.background.strong.color,
                            }
                        })
                        .width(160)
                        .line_height(text::LineHeight::Absolute(31.0.into())),
            vertical_space().height(5),
            row!(
                button(text("Cancel").center())
                    .style(|_, _| button::Style {
                        background: Some(Background::Color(color!(0x505050, 1.0))),
                        text_color: color!(0xF3F3F3, 1.0),
                        border: border::rounded(9.0),
                        shadow: Shadow {
                            color: Color::BLACK.scale_alpha(0.5),
                            offset: Vector::new(0.0, 0.0),
                            blur_radius: 4.0,
                        },
                    })
                    .height(37)
                    .width(100)
                     .on_press(Message::CloseModal),
                horizontal_space(),
                button(text("Save").center())
                    .style(|_, _| button::Style {
                        background: Some(Background::Color(color!(0x4198D7, 1.0))),
                        text_color: color!(0xF3F3F3, 1.0),
                        border: border::rounded(9.0),
                        shadow: Shadow {
                            color: Color::BLACK.scale_alpha(0.5),
                            offset: Vector::new(0.0, 0.0),
                            blur_radius: 4.0,
                        },
                    })
                    .height(37)
                    .width(100)
                    .on_press(Message::ColorSelected(color.to_owned()))
            )
        )
        .spacing(11)
        .align_x(Alignment::Center),
    )
    .width(450)
    .padding(12)
    .style(style)
    .into()
}
