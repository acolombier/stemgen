use iced::{
    border, color,
    widget::{button, column, container, horizontal_space, row, text, vertical_space},
    Alignment, Background, Color, Element, Shadow, Theme, Vector,
};

use crate::app::Message;

fn style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(color!(0x383838, 1.))),
        border: border::rounded(9.0),
        ..Default::default()
    }
}

pub fn new<'a>() -> Element<'a, Message> {
    container(
        column!(
            text("Warning"),
            vertical_space().height(30),
            text(format!("A file named “Tribal King - Façon Sex.stem.mp4” already exist in the output directory.")),
            vertical_space().height(40),
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
                    .width(100), // .on_press(match self.stage {
                //     Stage::FileSelection => Message::Split,
                //     Stage::StemEdition => Message::Export,
                //     Stage::Finished => Message::Quit,
                //     Stage::Exporting | Stage::Splitting => Message::Cancel,
                // })
                horizontal_space(),
                button(text("Overwrite").center())
                    .style(|_, _| button::Style {
                        background: Some(Background::Color(color!(0xF40162, 1.0))),
                        text_color: color!(0xF3F3F3, 1.0),
                        border: border::rounded(9.0),
                        shadow: Shadow {
                            color: Color::BLACK.scale_alpha(0.5),
                            offset: Vector::new(0.0, 0.0),
                            blur_radius: 4.0,
                        },
                    })
                    .height(37)
                    .width(100), // .on_press(match self.stage {
                                //     Stage::FileSelection => Message::Split,
                                //     Stage::StemEdition => Message::Export,
                                //     Stage::Finished => Message::Quit,
                                //     Stage::Exporting | Stage::Splitting => Message::Cancel,
                                // }),
                                button(text("Save as...").center())
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
                                    .width(100) // .on_press(match self.stage {
                                                //     Stage::FileSelection => Message::Split,
                                                //     Stage::StemEdition => Message::Export,
                                                //     Stage::Finished => Message::Quit,
                                                //     Stage::Exporting | Stage::Splitting => Message::Cancel,
                                                // })
            )
            .spacing(15)
        )
        .spacing(11)
        .align_x(Alignment::Center),
    )
    .width(450)
    .padding(12)
    .style(style)
    .into()
}
