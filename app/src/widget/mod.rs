use color_picker::{ColorPicker, SaturationPicker};
use iced::{
    border, color, widget::{container, horizontal_space, row, vertical_space, MouseArea}, Background, Color, Element
};
pub mod color_option;
pub mod color_picker;
pub mod color_selector;
pub use color_selector::ColorSelector;

pub(crate) fn color_option<'a>(selected_color: Color) -> MouseArea<'a, crate::app::Message> {
    MouseArea::new(
        container(vertical_space().width(18).height(18)).style(move |_| container::Style {
            border: border::rounded(4.0).width(1).color(color!(0x505050, 1.0)),
            background: Some(Background::Color(selected_color)),
            ..Default::default()
        }),
    )
}

pub(crate) fn color_picker<'a>(selected_color: &Color) -> Element<'a, crate::app::Message> {
    container(
        row!(
            SaturationPicker::new(selected_color.to_owned()),
            horizontal_space().width(15.0),
            ColorPicker::new(selected_color.to_owned()),
        )
    ).into()
}
