mod app;
mod modal;
pub mod model;
mod player;
mod server;
mod stem_track;
mod waveform;
mod widget;

use app::{Player, Stage};
use iced::alignment::Vertical;
use iced::widget::{center, image, mouse_area, opaque, stack, text_input, Row, Svg};
use iced::Length::{FillPortion, Shrink};
use iced::{
    alignment, border,
    widget::{button, column, container, horizontal_space, row},
    Color, Element, Length, Renderer, Settings, Theme, Vector,
};
use iced::{
    futures::channel::mpsc,
    widget::{text, vertical_space, Column, MouseArea},
    Background, Event,
    Length::Fill,
    Padding, Shadow, Subscription, Task,
};
use player::player_run;
use server::server_loop;
use stemgen::constant::{Metadata, MetadataValue};
use uuid::Uuid;
use std::collections::HashMap;
use iced::{color, gradient, Alignment, ContentFit, Font, Radians};


use crate::app::{Message, Modal, Server};
use crate::model::File;
use crate::stem_track::stem_track;

#[derive(Default, Debug)]
struct StemgenApp {
    input_files: HashMap<Uuid, File>,
    playing: Option<(Uuid, f32)>,
    link: Option<mpsc::Sender<Server>>,
    player: Option<mpsc::Sender<Player>>,
    stage: Stage,
    modal: Option<Modal>,
}

pub fn custom_style(_theme: &Theme) -> iced::widget::container::Style {
    let active = iced::widget::container::Style {
        text_color: Some(Color::from_rgb(0.8, 0.0, 0.8)),
        background: Some(Background::Color(Color::from_rgb(0.2, 0.2, 0.2))),
        shadow: Shadow {
            color: Color::from_rgb(0.2, 0.2, 0.2),
            offset: Vector::new(0.0, 0.0),
            blur_radius: 15.0,
        },
        ..Default::default()
    };
    active
}

impl StemgenApp {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                input_files: HashMap::from([File::test()]),
                stage: Stage::StemEdition,
                ..Default::default()
            },
            Task::none(),
        )
    }

    fn subscription(&self) -> Subscription<Message> {
        // event::listen()
        //     .map(Message::EventOccurred)
        //     .with(Subscription::run(server_loop));
        Subscription::batch([
            Subscription::run(server_loop),
            Subscription::run(player_run),
        ])
    }

    fn view(&'_ self) -> Element<'_, Message> {
        fn window_style(_theme: &Theme) -> container::Style {
            container::Style {
                background: Some(Background::Color(color!(0x383838, 1.))),
                ..Default::default()
            }
        }
        let mut col = column!(container(row!(
            horizontal_space(),
            MouseArea::new(text("...")).on_press(Message::Modal(Modal::Settings))
        ))
        .padding(10));
        if self.input_files.is_empty() {
            col = col.push(
                MouseArea::new(
                    container(
                        column!(
                            Svg::from_path("res/icons/add.svg")
                                .width(110)
                                .height(110)
                                .content_fit(ContentFit::Contain),
                            text("Click to add file to process\nor drag and drop there",)
                                .size(16)
                                .style(|_| text::Style {
                                    color: Some(color!(0xA5A5A5, 1.0))
                                })
                                .center()
                                .width(Fill)
                        )
                        .spacing(20)
                        .align_x(Alignment::Center),
                    )
                    .width(Fill)
                    .height(Fill)
                    .align_y(Alignment::Center),
                )
                .on_press(Message::AddNewFile),
            );
        } else {
            col = col
                .push(items_list_view(self.input_files.values().into_iter(), &self.playing, self.stage))
                .push(
                    row!(
                        horizontal_space(),
                        button(
                            text(match self.stage {
                                Stage::FileSelection => "Split",
                                Stage::StemEdition => "Save",
                                Stage::Finished => "Quit",
                                Stage::Exporting | Stage::Splitting => "Cancel",
                            })
                            .center()
                        )
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
                        .on_press(match self.stage {
                            Stage::FileSelection => Message::Split,
                            Stage::StemEdition => Message::Export,
                            Stage::Finished => Message::Quit,
                            Stage::Exporting | Stage::Splitting => Message::Cancel,
                        })
                    )
                    .padding(Padding::new(0.0).top(16).bottom(16)),
                );
        }
        let mut main: Element<'_, Message> = container(col)
            .padding(Padding {
                top: 0.0,
                bottom: 12.0,
                right: 16.0,
                left: 16.0,
            })
            .into();
        if self.modal.is_some() {
            main = modal(
                main,
                self.modal.as_ref().unwrap().build(),
                Message::CloseModal,
            );
        }
        container(main)
            .style(window_style)
            .width(Length::Fill)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Top)
            .into()
    }
    fn theme(&self) -> iced::Theme {
        iced::Theme::Dark
    }

    fn get_file_by_id(&mut self, id: Uuid) -> Option<&mut File> {
        self.input_files.get_mut(&id)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::EventOccurred(Event::Window(e)) => {
                println!("Window file: {:?}", e);
            }
            Message::EventOccurred(_) => {}
            // Message::FileUpdated(file_id) => {
            //     println!("updating: {:?}", file_id);
            //     match self.get_file_by_id(&file.i) {
            //         Some(f) => {
            //             f.stems = file.stems;
            //             f.metadata = file.metadata;
            //         }
            //         None => {
            //             println!("Can't find file: {:?}", file_id);
            //         }
            //     };
            // }
            Message::FileSplitProgress(file_id, progress) => match self.get_file_by_id(file_id) {
                Some(f) => {
                    f.progress = Some(progress);
                }
                None => {
                    println!("Can't find file: {:?}", file_id);
                }
            },
            Message::FileSplitCompleted(file_id, rendered, waveforms,) => {
                match self.get_file_by_id(file_id) {
                    Some(f) => {
                        f.set_rendered(rendered);
                        f.set_waveforms(waveforms);
                    }
                    None => {
                        println!("Can't find file: {:?}", file_id);
                    }
                }

                // update to next stage if all file have completed
                if self
                    .input_files
                    .values()
                    .fold(true, |acc, file| acc && file.is_ready())
                {
                    for file in self.input_files.values_mut() {
                        file.progress = None;
                    }
                    self.stage = Stage::StemEdition;
                }
            }
            Message::StemTrackLabelUpdated(file_id, stem_idx, label, text) => {
                println!("updateding: {:?}", file_id);
                match self.get_file_by_id(file_id) {
                    Some(f) => {
                        f.stems[stem_idx].label = label;
                        f.stems[stem_idx].label_text = text.to_owned();
                    }
                    None => {
                        println!("Can't find file: {:?}", file_id);
                    }
                }
            }
            Message::Player(Player::Ready(sender)) => {
                self.player = Some(sender);
                println!("Player ready");
            }
            Message::Player(Player::Progress(progress)) => {
                if let Some((_, current_progress)) = &mut self.playing {
                    *current_progress = progress;
                }
            }
            Message::Server(Server::Ready(app_ch)) => {
                self.link = Some(app_ch);
                println!("Link ready");
            }
            Message::AddNewFile => {
                if let Some(files) = rfd::FileDialog::new()
                    .set_title("Select files to load in stemgen")
                    .add_filter("Audio files", &["mp3", "wav", "flac", "ogg"])
                    .pick_files()
                {
                    // let factory = self.link.as_ref().unwrap();
                    for file in &files {
                        match File::new(&file) {
                            Ok(f) => assert!(self.input_files.insert(f.id(), f).is_none()),
                            Err(e) => {
                                println!("Unable to add file {:?}: {}", file, e);
                            }
                        };
                    }
                }
            }
            Message::PreviewSeek(file_id, progress) => {
                if let Some((playing, _)) = &self.playing {
                    if *playing == file_id {}

                    if let Err(e) = self
                        .player
                        .as_mut()
                        .unwrap()
                        .try_send(Player::Seek(progress))
                    {
                        println!("Unable to interact with player! {:?}", e);
                    }
                }
            }
            Message::Edit(file_id) => match self.get_file_by_id(file_id) {
                Some(f) if f.is_ready() => {
                    f.editing = !f.editing;
                }
                Some(_) => {
                    println!(
                        "Cannot edit {:?} because not yet total_samples().is_ok()",
                        file_id
                    );
                }
                None => {
                    println!("Can't find file: {:?}", file_id);
                }
            },
            Message::Preview(file_id, state) => {
                let mut player = self.player.clone().unwrap();
                self.playing = match (self.playing.clone(), self.get_file_by_id(file_id)) {
                    (_, Some(f)) if f.is_ready() => {
                        let mask = f.preview_mask();
                        let command = if state {
                            Player::PlayFile(f.id(), mask)
                        } else {
                            Player::Stop
                        };
                        if let Err(e) = player.try_send(command) {
                            println!("Unable to interact with player! {:?}", e);
                            None
                        } else {
                            if state {
                                Some((f.id(), 0.0))
                            } else {
                                None
                            }
                        }
                    }
                    (p, _) => {
                        println!(
                            "Cannot preview {:?} because it doesn't not have any data",
                            file_id
                        );
                        p
                    }
                };
            }
            Message::Remove(file_id) => {
                self.input_files.remove(&file_id);
                if let Some((playing, _)) = &self.playing {
                    if *playing == file_id {
                        if let Err(e) = self.player.as_mut().unwrap().try_send(Player::Stop) {
                            println!("Unable to interact with player! {:?}", e);
                        }
                        self.playing = None;
                    }
                }
                if self.input_files.is_empty() {
                    self.stage = Stage::FileSelection;
                }
            }
            Message::Toggle(file_id) => match self.get_file_by_id(file_id) {
                Some(f) => {
                    f.selected = !f.selected;
                }
                None => {
                    println!("Can't find file: {:?}", file_id);
                }
            },
            Message::StemTrackRequestColorChange(file_id, stem_idx) => {
                match self.get_file_by_id(file_id) {
                    Some(f) if f.is_ready() => {
                        f.stems[stem_idx].selecting_color = true;
                    }
                    Some(_) => {
                        println!(
                            "Cannot edit {:?} because not yet total_samples().is_ok()",
                            file_id
                        );
                    }
                    None => {
                        println!("Can't find file: {:?}", file_id);
                    }
                }
            }
            Message::StemTrackToggleMute(file_id, stem_idx) => {
                let mut player = self.player.clone().unwrap();
                self.playing = match (self.playing.clone(), self.get_file_by_id(file_id)) {
                    (Some((playing, progress)), Some(file))
                        if file.is_ready() && playing == file_id =>
                    {
                        file.stems[stem_idx].muted = !file.stems[stem_idx].muted;
                        if let Err(e) =
                            player.try_send(Player::PlayFile(file.id(), file.preview_mask()))
                        {
                            println!("Unable to interact with player! {:?}", e);
                            None
                        } else {
                            Some((playing, progress))
                        }
                    }
                    (playing, Some(file)) => {
                        file.stems[stem_idx].muted = !file.stems[stem_idx].muted;
                        playing
                    }
                    (playing, _) => {
                        println!("File {:?} is found!", file_id);
                        playing
                    }
                }
            }
            Message::StemTrackColorChanged(file_id, stem_idx, color) => {
                match self.get_file_by_id(file_id) {
                    Some(f) => {
                        f.stems[stem_idx].selecting_color = false;
                        if let Some(new_color) = color {
                            f.stems[stem_idx].color = new_color;
                        }
                    }
                    None => {
                        println!("Can't find file: {:?}", file_id);
                    }
                }
            }
            Message::MetadataEdit(file_id, metata, value) => {
                match self.get_file_by_id(file_id) {
                    Some(f) => {
                        f.metadata.insert(metata, value);
                    }
                    None => {
                        println!("Can't find file: {:?}", file_id);
                    }
                }
            }
            Message::Split => {
                self.stage = Stage::Splitting;
                self.input_files.iter_mut().for_each(|(file_id, file)| {
                    if let Err(e) = self
                        .link
                        .as_mut()
                        .unwrap()
                        .try_send(Server::SplitFile(file.id(), file.path.clone()))
                    {
                        // cell.progress = Some(progress);
                        println!("Unable to send {:?}: {:?}", file_id, e)
                    } else {
                        file.progress = Some(0.0);
                    }
                });
            }
            Message::CloseModal => {
                self.modal = None;
            }
            Message::Modal(modal) => {
                self.modal = Some(modal);
            }
            Message::Cancel => {}
            Message::Export => {

                self.stage = Stage::Exporting;
            }
            Message::SettingAddCustomColor => {
                self.modal = Some(Modal::ColorPicker(Color::new(1.0, 0.0, 0.0, 1.0)));
            }
            _ => {
                println!("UI unhandled message: {:?}", message);
            }
        };
        Task::none()
    }
}

fn modal<'a, Message>(
    base: impl Into<Element<'a, Message>>,
    content: impl Into<Element<'a, Message>>,
    on_blur: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    stack![
        base.into(),
        opaque(
            mouse_area(center(opaque(content)).style(|_theme| {
                container::Style {
                    background: Some(
                        Color {
                            a: 0.8,
                            ..Color::BLACK
                        }
                        .into(),
                    ),
                    ..container::Style::default()
                }
            }))
            .on_press(on_blur)
        )
    ]
    .into()
}

fn items_list_view<'a, I: IntoIterator<Item = &'a File>>(
    items: I,
    playing: &Option<(Uuid, f32)>,
    stage: Stage
) -> Element<'a, Message> {
    let column: Element<'a, Message, Theme, Renderer> = {
        let mut list = Column::new()
            .spacing(10)
            .width(Length::Fill)
            .height(Length::Fill);
        for file in items.into_iter() {
            list = list.push(file_item(
                file,
                match &playing {
                    Some((playing, progress)) if *playing == file.id() => {
                        Some(progress.clone())
                    }
                    _ => None,
                },
                stage
            ));
        }
        list.push(vertical_space().height(Fill))
            .align_x(iced::Alignment::Start)
            .padding(9)
            .into()
    };

    fn custom_style(_theme: &Theme) -> container::Style {
        container::Style {
            border: border::rounded(9.0),
            background: Some(Background::Color(color!(0x2E2E2E, 1.))),
            ..Default::default()
        }
    }

    // scrollable(
    container(column)
        .height(Length::Fill)
        .style(custom_style)
        .width(Length::Fill)
        // )
        // .height(Length::Fill)
        // .width(Length::Fill)
        .into()
}
fn file_item<'a>(file: &'a File, playback: Option<f32>, stage: Stage) -> Element<'a, Message> {
    let expanded = file.selected && file.is_ready();
    let custom_style = move |_theme: &Theme| container::Style {
        background: if let Some(progress) = file.progress {
            Some(Background::Gradient(gradient::Gradient::Linear(
                gradient::Linear::new(Radians::PI / 2.0)
                    .add_stop(0.0, color!(0x4198D7, 1.))
                    .add_stop(progress, color!(0x4198D7, 1.))
                    .add_stop(progress + 0.0001, color!(0x696969, 1.))
                    .add_stop(1.0, color!(0x696969, 1.)),
            )))
        } else {
            Some(Background::Color(color!(0x696969, 1.)))
        },
        border: border::rounded(6.0)
            .color(color!(0xFFFFFF, 1.0))
            .width(if file.selected { 1.0 } else { 0.0 }),
        ..Default::default()
    };
    fn title_style(_them: &Theme) -> text::Style {
        text::Style {
            color: Some(color!(0xF1F1F1, 1.)),
        }
    }
    let cover: Element<'a, _> = match &file.cover {
        Some(cover) => image(cover)
            .width(47)
            .height(47)
            .content_fit(ContentFit::Cover)
            .into(),
        None => container(text("?").center().width(Fill))
            .align_y(Alignment::Center)
            .width(47)
            .height(47)
            .style(|_| container::Style {
                background: Some(Background::Color(color!(0x494949, 1.0))),
                border: border::rounded(4)
                    .width(1)
                    .color(Color::BLACK.scale_alpha(0.8)),
                ..Default::default()
            })
            .into()
    };
    let mut row = row!(
        cover,
        horizontal_space().width(19.0),
        text(file.label())
            .style(title_style)
            .font(Font::with_name("Noto Sans")),
        horizontal_space()
    );
    if file.is_ready() && stage == Stage::StemEdition {
        row = row.extend([
            MouseArea::new(
                Svg::from_path(if playback.is_some() {
                    "res/icons/stop.svg"
                } else {
                    "res/icons/play.svg"
                })
                .width(15)
                .height(15)
                .content_fit(ContentFit::Contain),
            )
            .on_press(Message::Preview(file.id(), !playback.is_some()))
            .into(),
            horizontal_space().width(9.0).into(),
            MouseArea::new(
                Svg::from_path("res/icons/edit.svg")
                    .opacity(if file.editing { 1.0 } else { 0.6 })
                    .width(15)
                    .height(15)
                    .content_fit(ContentFit::Contain),
            )
            .on_press(Message::Edit(file.id()))
            .into(),
        ]);
    }
    if file.progress.is_none() {
        row = row.push(horizontal_space().width(9.0)).push(
            MouseArea::new(
                Svg::from_path("res/icons/remove.svg")
                    .width(15)
                    .height(15)
                    .content_fit(ContentFit::Contain),
            )
            .on_press(Message::Remove(file.id())),
        );
    } else if let Some(progress) = file.progress {
        row = row.push(
            text(format!("{:.0} %", progress * 100.0))
                .size(15)
                .style(|_| text::Style {
                    color: Some(color!(0xF1F1F1, 1.0)),
                }),
        );
    }
    row = row.push(horizontal_space().width(12.0));
    let row = MouseArea::new(
        container(
            row.width(Fill)
                .height(Length::Shrink)
                .align_y(Vertical::Center),
        )
        .padding(3),
    )
    .on_press(Message::Toggle(file.id()));
    let mut element = Column::new()
        .spacing(5)
        .align_x(iced::Alignment::Start)
        .width(Length::Fill)
        .height(Length::Shrink)
        .push(row);

    if expanded {
        if file.stems.is_empty() {
            element = element.push(text("Loading file details...")).padding(5)
        } else if file.editing {
            let left_column = [Metadata::Title, Metadata::Artist, Metadata::Release];
            let right_column = [Metadata::Label, Metadata::Genre, Metadata::TrackNo];

            let build_input_label = |m: Metadata| {
                text(m.to_string())
                    .height(31.0)
                    .align_y(Alignment::Center)
            };
            let build_input_field = |m| {
                text_input("", file.metadata.get(&m).map_or("".to_owned(), |v| v.to_string()).as_ref())
                    .on_input(move|value|
                        match file.metadata.get(&m) {
                            Some(MetadataValue::String(_)) | None => Message::MetadataEdit(file.id(), m, MetadataValue::String(value)),
                            Some(MetadataValue::Number(old)) => match value.parse::<u32>(){
                                Ok(value) => Message::MetadataEdit(file.id(), m, MetadataValue::Number(value)),
                                Err(_) => Message::MetadataEdit(file.id(), m, MetadataValue::Number(*old)),
                            },
                        }
                    )
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
            };
            fn container_style(_theme: &Theme) -> container::Style {
                container::Style {
                    background: Some(Background::Color(color!(0x505050, 1.))),
                    ..Default::default()
                }
            }
            let left_column_labels = Column::with_children(left_column.into_iter().map(|m|build_input_label(m).into()))
            .align_x(Alignment::End)
            .spacing(15.0);
            let left_column_fields = Column::with_children(left_column.into_iter().map(|m|build_input_field(m).into()))
            .spacing(7.0);
            let right_column_labels = Column::with_children(right_column.into_iter().map(|m|build_input_label(m).into()))
            .align_x(Alignment::End)
            .spacing(15.0);
            let right_column_fields = Column::with_children(right_column.into_iter().map(|m|build_input_field(m).into()))
            .spacing(7.0);

            element = element.push(
                container(column![
                    container(horizontal_space().width(Fill).height(2)).style(container_style),
                    horizontal_space().width(Fill).height(6),
                    Row::with_children([
                        left_column_labels.width(FillPortion(1)).into(),
                        left_column_fields.width(FillPortion(3)).into(),
                        container(vertical_space().width(2).height(Shrink)).style(container_style).into(),
                        right_column_labels.width(FillPortion(1)).into(),
                        right_column_fields.width(FillPortion(3)).into(),
                    ])
                    .spacing(5)
                .padding(5)
                ]),
            )
        } else {
            let stems: Vec<Element<'a, Message, Theme, Renderer>> = (0..4)
                .map(|i| {
                    stem_track(&file, i, playback)
                        .on_color_select(move |color| {
                            Message::StemTrackColorChanged(file.id(), i, color)
                        })
                        .into()
                })
                .collect();
            element = element.push(Column::with_children(stems).padding(5).spacing(5))
        }
    }

    container(element)
        .width(Length::Fill)
        .height(Length::Shrink)
        .style(custom_style)
        .align_x(alignment::Horizontal::Left)
        .align_y(alignment::Vertical::Top)
        .into()
}

fn main() -> iced::Result {
    let settings = Settings {
        id: Some("test".into()),
        antialiasing: true,
        default_font: Font::with_name("Noto Sans"),
        ..Settings::default()
    };
    // pyo3::prepare_freethreaded_python();
    iced::application("Stemgen App", StemgenApp::update, StemgenApp::view)
        .subscription(StemgenApp::subscription)
        .theme(StemgenApp::theme)
        .settings(settings)
        .run_with(StemgenApp::new)
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use stemgen::{demucs::{Demucs, DemusOpts, Model}, track::Track};
    use uuid::Uuid;

    use crate::{model::RenderedFile, waveform};

    #[test]
    fn test_splitting() -> Result<(), Box<dyn std::error::Error>> {
        let path = "../testdata/Oddchap - Sound 104.mp3".into();
        let mut demucs = Demucs::new_from_file(&Model::default(), DemusOpts::default())?;

        let mut input = Track::new(&path)?;
        let mut read = 0;
        let mut output = RenderedFile::new(Uuid::new_v4()).unwrap();
        // FIXME what is total returning? Document!
        let total_size = input.total() as usize;
        let sample_per_slice = total_size / waveform::SAMPLE_COUNT;
        let mut current_slice = 0;

        let mut waveforms = vec![vec![(0.0, 0.0); waveform::SAMPLE_COUNT]; 4];
        let mut waveform_overrun = vec![vec![0f32; sample_per_slice]; 4];

        loop {
            let mut buf: Vec<f32> = vec![0f32; 343980 * 2];
            let mut original_packets = Vec::with_capacity(512);

            let (data, eof) = loop {
                let size = input.read(Some(&mut original_packets), &mut buf)?;
                read += size;
                if let Some(mut data) = demucs.send(&buf[..size])? {
                    data.insert(0, buf[..size].to_vec());
                    break (data, false);
                }
                if size != buf.len() {
                    let mut data = demucs.flush()?;
                    data.insert(0, buf[..size].to_vec());
                    break (data, true);
                }
            };

            for (i, waveform) in waveforms.iter_mut().enumerate() {
                let mut slice = current_slice;
                // TODO handle current waveform overrun
                for samples in data[1 + i].chunks(sample_per_slice) {
                    if samples.len() != sample_per_slice {
                        waveform_overrun[i].resize(samples.len(), 0.0);
                        waveform_overrun[i].copy_from_slice(&samples);
                    }
                    // Using Max - TODO RMS
                    waveform[slice] = samples.into_iter()
                        .tuples::<(_, _)>()
                        .fold((0.0 as f32, 0.0 as f32), |(lacc, racc), (l, r)| {
                            (
                                if lacc > l.abs() { lacc } else { l.abs() },
                                if racc > r.abs() { racc } else { r.abs() },
                            )
                        });
                    slice+=1;
                }
            }
            current_slice += data[0].len() / sample_per_slice;
            output.write(original_packets, data).unwrap();

            println!("progress: {}", (read as f32) / (total_size as f32));

            if eof {
                output.complete()?;
                break;
            }
        }

        assert_eq!(waveforms[0].len(), waveform::SAMPLE_COUNT);

        println!("cached output size: {sample_per_slice} {:?}", waveforms[0]);
        // let mut f = std::fs::File::create("./out.pcm").unwrap();
        let mut total = 0;
        loop {
            let mut buffer = vec![0f32; 10240];
            let read = output.read(None, Some(buffer.as_mut_slice()), None, None, None).unwrap();
            total += read;
            if read != 10240{
                break
            }
        }

        let existing = RenderedFile::existing(output.id()).unwrap();
        assert_eq!(output.total_samples().unwrap() as usize, total);
        assert_eq!(existing.total_samples().unwrap() as usize, total);

        println!("total size: {total}");
        Ok(())
    }
}
