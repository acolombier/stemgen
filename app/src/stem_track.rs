use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{layout, mouse, overlay, renderer, widget, Layout, Widget};
use iced::advanced::{Clipboard, Shell};
use iced::alignment::Vertical;
use iced::widget::{
    column, container, horizontal_space, pick_list, row, text, text_input, vertical_space,
    MouseArea, Svg,
};
use iced::{
    border, color, event, Alignment, Background, Color, ContentFit, Element, Event, Fill, Font,
    Length, Padding, Rectangle, Renderer, Size, Theme, Vector,
};

use crate::model::{File, Stem, TrackLabel};
use crate::waveform::Waveform;
use crate::widget::{color_option, ColorSelector};
use crate::Message;

pub struct StemTrack<'a, Message> {
    content: Element<'a, Message>,
    stem: &'a Stem,
    color_select: Option<Box<dyn Fn(Option<Color>) -> Message + 'a>>,
}

const EMPTY_WAVEFORM: Vec<(f32, f32)> = vec![];

impl<'a> StemTrack<'a, Message> {
    fn new(
        file: &'a File,
        stem_idx: usize,
        playback: Option<f32>,
    ) -> Self {
        let label_updated = move |label: TrackLabel| -> Message {
            Message::StemTrackLabelUpdated(file.id(), stem_idx, label, "".to_owned())
        };
        let label_edited = move |text: String| -> Message {
            Message::StemTrackLabelUpdated(file.id(), stem_idx, TrackLabel::Custom, text)
        };

        fn container_style(_theme: &Theme) -> container::Style {
            container::Style {
                background: Some(Background::Color(color!(0x505050, 1.))),
                ..Default::default()
            }
        }
        let mute_style = move |_theme: &Theme| text::Style {
            color: Some(color!(if file.stems[stem_idx].muted { 0xFFFFFF } else { 0xA5A5A5 }, 1.)),
        };

        let label: container::Container<'a, _> = if file.stems[stem_idx].label != TrackLabel::Custom {
            fn picklist_style(theme: &Theme, _status: pick_list::Status) -> pick_list::Style {
                let palette = theme.extended_palette();
                pick_list::Style {
                    background: Background::Color(color!(0x505050, 1.0)),
                    text_color: color!(0xF1F1F1, 1.),
                    placeholder_color: palette.background.strong.color,
                    handle_color: color!(0x696969, 1.),
                    border: border::rounded(5.0),
                }
            }
            container(
                pick_list(
                    &TrackLabel::ALL[..],
                    Some(file.stems[stem_idx].label.clone()),
                    label_updated,
                )
                .font(Font::with_name("Noto Sans"))
                .text_size(12)
                .text_line_height(text::LineHeight::Absolute(31.0.into()))
                .width(Fill)
                .style(picklist_style),
            )
        } else {
            container(
                row!(
                    text_input("Enter the custom label", &file.stems[stem_idx].label_text)
                        .on_input(label_edited)
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
                        .width(Fill)
                        .line_height(text::LineHeight::Absolute(31.0.into())),
                    MouseArea::new(
                        Svg::from_path("res/icons/remove.svg")
                            .width(15)
                            .height(15)
                            .content_fit(ContentFit::Contain),
                    )
                    .on_press(Message::StemTrackLabelUpdated(
                        file.id(),
                        stem_idx,
                        TrackLabel::Drums,
                        "".to_owned()
                    ))
                )
                .align_y(Vertical::Center)
                .spacing(5),
            )
        };

        Self {
            color_select: None,
            content: container(
                column!(
                    container(horizontal_space().width(Fill).height(2)).style(container_style),
                    row!(
                        horizontal_space().width(3),
                        label.height(31).width(125.0),
                        color_option(file.stems[stem_idx].color).on_press(Message::StemTrackRequestColorChange(
                            file.id(),
                            stem_idx
                        )),
                        Waveform::new(
                            file.stems[stem_idx].color,
                            file.stems[stem_idx].muted,
                            // file.stems[stem_idx].waveform.as_ref().unwrap_or(&EMPTY_WAVEFORM),
                            file.stems[stem_idx].waveform.as_ref().unwrap(),
                            playback.unwrap_or(1.0),
                            |progress| Message::PreviewSeek(file.id(), progress)
                        ),
                        horizontal_space().width(2),
                        MouseArea::new(
                            text("M")
                                .style(mute_style)
                                .size(12)
                                .font(Font::with_name("Noto Sans"))
                        )
                        .on_press(Message::StemTrackToggleMute(file.id(), stem_idx)),
                        horizontal_space().width(2),
                    )
                    .align_y(Vertical::Center)
                    .spacing(5)
                )
                .spacing(4),
            )
            .into(),
            stem: &file.stems[stem_idx],
        }
    }
    pub fn on_color_select(mut self, callback: impl Fn(Option<Color>) -> Message + 'a) -> Self {
        self.color_select = Some(Box::new(callback));
        self
    }
}

// #[derive(Debug, Clone, Copy, PartialEq, Default)]
// enum State {
//     #[default]
//     DefinedLabel(widget::tree::State),
//     CustomLabel(widget::tree::State)
// }

impl<'a> Widget<Message, Theme, Renderer> for StemTrack<'a, Message> {
    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Shrink,
            height: Length::Shrink,
        }
    }

    fn layout(
        &self,
        tree: &mut widget::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget().layout(tree, renderer, limits)
    }

    fn state(&self) -> widget::tree::State {
        self.content.as_widget().state()
    }

    fn tag(&self) -> widget::tree::Tag {
        self.content.as_widget().tag()
    }

    fn diff(&self, tree: &mut Tree) {
        self.content.as_widget().diff(tree)
    }

    /// Applies an [`Operation`] to the [`Widget`].
    fn operate(
        &self,
        state: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget()
            .operate(state, layout, renderer, operation)
    }

    fn draw(
        &self,
        state: &widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content
            .as_widget()
            .draw(state, renderer, theme, style, layout, cursor, viewport)
    }

    /// Processes a runtime [`Event`].
    ///
    /// By default, it does nothing.
    fn on_event(
        &mut self,
        state: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) -> event::Status {
        // let state = tree.state.downcast_mut::<State>();
        // if {
        //     shell.invalidate_layout();
        //     return event::Status::Captured;
        // }
        self.content.as_widget_mut().on_event(
            state, event, layout, cursor, renderer, clipboard, shell, viewport,
        )
    }

    fn mouse_interaction(
        &self,
        state: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content
            .as_widget()
            .mouse_interaction(state, layout, cursor, viewport, renderer)
    }

    fn children(&self) -> Vec<Tree> {
        self.content.as_widget().children()
    }

    /// Returns the overlay of the [`Widget`], if there is any.
    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        if self.stem.selecting_color && self.color_select.is_some() {
            fn picker_style(_theme: &Theme) -> container::Style {
                container::Style {
                    background: Some(Background::Color(color!(0x505050, 1.))),
                    border: border::rounded(4.),
                    ..Default::default()
                }
            }
            let mut color_list: Vec<Color> = [
                color!(0x4198D7, 1.0),
                color!(0xFF9D0A, 1.0),
                color!(0x31B15D, 1.0),
                color!(0xF40162, 1.0),
                color!(0xAB4AFF, 1.0),
            ]
            .into_iter()
            .filter_map(|color| {
                if self.stem.color.eq(&color) {
                    None
                } else {
                    Some(color.to_owned())
                }
            })
            .collect();
            color_list.insert(0, self.stem.color);
            let picker: Element<'_, Message> = container(
                row![
                    container(
                            MouseArea::new(
                                Svg::from_path("res/icons/add.svg")
                                    .width(20)
                                    .height(20)
                                    .content_fit(ContentFit::Contain),
                            )
                            .on_press(Message::SettingAddCustomColor)
                        )]
                .extend(color_list.into_iter().enumerate().map(|(i, color)| {
                    container(
                        MouseArea::new(vertical_space().width(20).height(20)).on_press((self
                            .color_select
                            .as_ref()
                            .unwrap())(
                            Some(color.to_owned()),
                        )),
                    )
                    .style(move |_| container::Style {
                        border: border::rounded(4.0)
                            .width(1)
                            .color(color!(0xFFFFFF, if i == 0 { 1.0 } else { 0.0 })),
                        background: Some(Background::Color(color.clone())),
                        ..Default::default()
                    })
                    .into()
                }))
                .spacing(3.0)
                .width(Fill)
                .height(Fill)
                .align_y(Alignment::Center),
            )
            .padding(3.0)
            .style(picker_style)
            .width(Fill)
            .height(Fill)
            .into();
            let selector = ColorSelector {
                color_select: self.color_select.as_ref().unwrap(),
                state: Tree::new(picker.as_widget()),
                picker,
                viewport: layout.bounds(),
            };
            Some(overlay::Element::new(Box::new(selector)))
        } else {
            self.content
                .as_widget_mut()
                .overlay(tree, layout, renderer, translation)
        }
    }
}

impl<'a> From<StemTrack<'a, Message>> for Element<'a, Message> {
    fn from(circle: StemTrack<'a, Message>) -> Self {
        Self::new(circle)
    }
}

pub fn stem_track<'a>(
    file: &'a File,
    stem_idx: usize,
    playback: Option<f32>,
) -> StemTrack<'a, Message> {
    StemTrack::new(file, stem_idx, playback)
}
