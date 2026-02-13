use iced::{
    Alignment, Background, Color, ContentFit, Element, Font,
    Length::Fill,
    Padding, Shadow, Theme, Vector, border, color,
    widget::{
        Column, MouseArea, Svg, button, column, container, horizontal_space, pick_list, row, text,
        text_input, tooltip, vertical_space,
    },
};
use stemgen::constant::DEFAULT_MODEL;

use crate::app::Message;

static OUTPUT_MODE: &'static [&str] = &[
    "Prompt once for all",
    "Prompt for each",
    "Use the same folder than the source file",
];
static CODEC_LIST: &'static [&str] = &[
    "AAC",
    "ALAC",
    "FLAC (not standard)",
    "OGG (not standard)",
    "WAV (not standard)",
];
static EXTENSION_LIST: &'static [&str] = &[".stem.mp4", ".stem.m4a", ".mp4", ".m4a"];

enum Setting<'a, Message> {
    Enum(Vec<&'a str>, &'a str, Box<dyn Fn(&str) -> Message + 'a>),
    String(&'a str, Box<dyn Fn(String) -> Message + 'a>),
    ColorList(
        Box<dyn Fn() -> Message + 'a>,
        Box<dyn Fn(Color) -> Message + 'a>,
        Vec<Color>,
    ),
}

fn style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(color!(0x383838, 1.))),
        border: border::rounded(9.0),
        ..Default::default()
    }
}

fn default_settings<'a>() -> Vec<(&'static str, Setting<'static, Message>)> {
    vec![
        (
            "Demucs model",
            Setting::String(
                DEFAULT_MODEL,
                Box::new(|m| Message::SettingSelectModel(m.to_owned())),
            ),
        ),
        (
            "Output folder",
            Setting::Enum(
                OUTPUT_MODE.into(),
                OUTPUT_MODE[0],
                Box::new(|m| Message::SettingSelectModel(m.to_owned())),
            ),
        ),
        (
            "Stem codec",
            Setting::Enum(
                CODEC_LIST.into(),
                CODEC_LIST[0],
                Box::new(|m| Message::SettingSelectModel(m.to_owned())),
            ),
        ),
        (
            "Extension",
            Setting::Enum(
                EXTENSION_LIST.into(),
                EXTENSION_LIST[0],
                Box::new(|m| Message::SettingSelectModel(m.to_owned())),
            ),
        ),
        (
            "Custom color",
            Setting::ColorList(
                Box::new(|| Message::SettingAddCustomColor),
                Box::new(|color| Message::SettingRemoveCustomColor(color.to_owned())),
                vec![color!(0x123456, 1.0)],
            ),
        ),
    ]
}

pub fn new<'a>() -> Element<'a, Message> {
    let (setting_labels, setting_widgets) = default_settings()
        .into_iter()
        .map(|(label, setting)| {
            let widget: Element<'a, Message> = match setting {
                Setting::Enum(options, selected, on_select) => {
                    pick_list(options, Some(selected), on_select)
                        .font(Font::with_name("Noto Sans"))
                        .text_size(12)
                        .text_line_height(text::LineHeight::Absolute(31.0.into()))
                        .width(Fill)
                        .style(|theme: &Theme, _status: pick_list::Status| {
                            let palette = theme.extended_palette();
                            pick_list::Style {
                                background: Background::Color(color!(0x505050, 1.0)),
                                text_color: color!(0xF1F1F1, 1.),
                                placeholder_color: palette.background.strong.color,
                                handle_color: color!(0x696969, 1.),
                                border: border::rounded(5.0),
                            }
                        })
                        .into()
                }
                Setting::ColorList(on_add, on_delete, value) => container(
                    row!(
                        horizontal_space().width(Fill),
                        container(
                            MouseArea::new(
                                Svg::from_path("res/icons/add.svg")
                                    .width(20)
                                    .height(20)
                                    .content_fit(ContentFit::Contain),
                            )
                            .on_press((on_add)())
                        )
                    )
                    .extend(value.into_iter().enumerate().map(|(i, color)| {
                        tooltip(
                            container(
                                MouseArea::new(vertical_space().width(20).height(20))
                                    .on_press((on_delete)(color.clone())),
                            )
                            .style(move |_| container::Style {
                                border: border::rounded(4.0)
                                    .width(1)
                                    .color(color!(0xFFFFFF, if i == 0 { 1.0 } else { 0.0 })),
                                background: Some(Background::Color(color.clone())),
                                ..Default::default()
                            }),
                            container("Click to remove")
                                .padding(10)
                                .style(container::rounded_box),
                            tooltip::Position::Bottom,
                        )
                        .gap(10)
                        .into()
                    }))
                    .spacing(4.0)
                    .align_y(Alignment::Center),
                )
                .padding(3.0)
                .into(),
                Setting::String(items, on_input) => container(row!(
                    horizontal_space().width(Fill),
                    text_input("Model path or URL", items)
                        .on_input(on_input)
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
                        .line_height(text::LineHeight::Absolute(31.0.into()))
                ))
                .into(),
            };
            (text(label), container(widget))
        })
        .fold(
            (Column::new(), Column::new()),
            |(labels, widgets), (label, widget)| {
                (
                    labels
                        .push(label.height(32.0))
                        .push(vertical_space().height(10)),
                    widgets
                        .push(widget.height(32.0))
                        .push(vertical_space().height(10)),
                )
            },
        );
    iced::widget::container(
        column!(text("Settings"), vertical_space().height(10))
            .push(row!(
                setting_labels,
                horizontal_space().width(16.0),
                setting_widgets
            ))
            .push(vertical_space().height(20))
            .push(row!(
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
                    .width(100) // .on_press(match self.stage {
                                //     Stage::FileSelection => Message::Split,
                                //     Stage::StemEdition => Message::Export,
                                //     Stage::Finished => Message::Quit,
                                //     Stage::Exporting | Stage::Splitting => Message::Cancel,
                                // })
            ))
            .spacing(11)
            .align_x(Alignment::Center),
    )
    .width(450)
    .padding(12)
    .style(style)
    .into()
}
