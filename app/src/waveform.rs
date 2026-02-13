use iced::advanced::graphics::mesh::SolidVertex2D;
use iced::advanced::graphics::{self, Mesh};
use iced::advanced::widget::Tree;
use iced::advanced::{layout, mouse, renderer, widget, Layout, Widget};
use iced::advanced::{Clipboard, Renderer as _, Shell};
use iced::mouse::Button;
use iced::widget::container;
use iced::{
    border, color, event, Color, Element, Event, Length, Rectangle, Renderer, Size, Theme,
    Transformation,
};

pub struct Waveform<'a, Message>
where
    Renderer: iced::advanced::Renderer,
    Theme: container::Catalog,
{
    color: Color,
    opaque: bool,
    progress: f32,
    seek: Box<dyn Fn(f32) -> Message + 'a>,
    waveform: &'a Vec<(f32, f32)>,
}

pub const SAMPLE_COUNT: usize = 2048;

impl<'a, Message> Waveform<'a, Message> {
    pub fn new(
        color: Color,
        opaque: bool,
        waveform: &'a Vec<(f32, f32)>,
        progress: f32,
        seek: impl Fn(f32) -> Message + 'a,
    ) -> Self {
        Self {
            color,
            opaque,
            progress,
            seek: Box::new(seek),
            waveform,
        }
    }
}

impl<'a, Message> Widget<Message, Theme, Renderer> for Waveform<'a, Message> {
    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fill,
            height: Length::Fixed(40.0),
        }
    }

    fn layout(
        &self,
        _tree: &mut widget::Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.max_height(41.0).max())
    }

    fn on_event(
        &mut self,
        _state: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> event::Status {
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(Button::Left))
                if cursor.position_over(layout.bounds()).is_some() =>
            {
                let bound = cursor.position_in(layout.bounds()).unwrap();
                shell.publish((self.seek)(bound.x / layout.bounds().size().width));
                // println!("{}, {}", bound.x / layout.bounds().size().width);
                event::Status::Captured
            }
            _ => event::Status::Ignored,
        }
    }

    fn draw(
        &self,
        _state: &widget::Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let opacity = if self.opaque { 0.5 } else { 1.0 };
        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: border::rounded(8.0),
                ..renderer::Quad::default()
            },
            if self.progress == 1.0 {
                self.color.scale_alpha(opacity)
            } else {
                color!(0x505050, opacity)
            },
        );
        if self.progress != 1.0 {
            let bounds = Rectangle::new(
                bounds.position(),
                Size::new(bounds.width * self.progress, bounds.height),
            );
            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    border: border::rounded(if self.progress == 1.0 {
                        border::Radius::new(8.0)
                    } else {
                        border::Radius::new(8.0).top_right(0.0).bottom_right(0.0)
                    }),
                    ..renderer::Quad::default()
                },
                self.color.scale_alpha(opacity),
            );
        }
        let waveform_color = color!(0x202020, opacity);

        let mut offset = 0.0;
        let mut vertices: Vec<SolidVertex2D> = vec![SolidVertex2D {
            position: [offset, bounds.height / 2.0],
            color: graphics::color::pack(waveform_color),
        }];
        let mut indices: Vec<u32> = Vec::new();

        let factor = bounds.width / (SAMPLE_COUNT as f32);
        let mut origin: [u32; 2] = [0, 0];
        offset += factor;

        for i in 0..=SAMPLE_COUNT {
            let (mut left, mut right): (f32, f32) = if i >= self.waveform.len() {
                (0.501, 0.501)
            } else {
                // println!("{}", self.waveform[i]);
                self.waveform[i]
            };

            left *= bounds.height * 0.3;
            right *= bounds.height * 0.3;

            let current_indices_count = vertices.len() as u32;

            let mut samples_vertices: Vec<SolidVertex2D>;
            (samples_vertices, origin) = if left > 0.0 || right > 0.0 {
                indices.append(&mut if origin[0] != origin[1] {
                    vec![
                        // Core sample
                        origin[0],
                        current_indices_count,
                        origin[1],
                        // Left sample
                        origin[0],
                        current_indices_count + 1,
                        current_indices_count,
                        // Right sample
                        current_indices_count,
                        current_indices_count + 2,
                        origin[1],
                    ]
                } else {
                    vec![
                        // Left sample
                        origin[0],
                        current_indices_count + 1,
                        current_indices_count,
                        // Right sample
                        current_indices_count,
                        current_indices_count + 2,
                        origin[1],
                    ]
                });
                (
                    vec![
                        SolidVertex2D {
                            position: [offset, bounds.height / 2.0],
                            color: graphics::color::pack(waveform_color),
                        },
                        SolidVertex2D {
                            position: [offset, bounds.height / 2.0 - left],
                            color: graphics::color::pack(waveform_color),
                        },
                        SolidVertex2D {
                            position: [offset, bounds.height / 2.0 + right],
                            color: graphics::color::pack(waveform_color),
                        },
                    ],
                    [current_indices_count + 1, current_indices_count + 2],
                )
            } else {
                if origin[0] != origin[1] {
                    indices.append(&mut vec![
                        // Core sample
                        origin[0],
                        current_indices_count,
                        origin[1],
                    ]);
                }
                (
                    vec![SolidVertex2D {
                        position: [offset, bounds.height / 2.0],
                        color: graphics::color::pack(waveform_color),
                    }],
                    [current_indices_count, current_indices_count],
                )
            };

            vertices.append(&mut samples_vertices);
            offset += factor
        }

        // println!("{:?}", &vertices[100..105]);
        // println!("{:?}", &self.waveform[100..105]);
        // println!("{:?}", indices);

        let mesh = Mesh::Solid {
            buffers: mesh::Indexed { vertices, indices },
            transformation: Transformation::translate(bounds.x, bounds.y),
            clip_bounds: Rectangle::with_size(bounds.size()),
        };

        use iced::advanced::graphics::mesh::{self, Renderer as _};
        renderer.draw_mesh(mesh);
        // renderer.with_layer(bounds, |renderer| {
        //     Container::draw(
        //         &self.container,
        //         state,
        //         renderer,
        //         theme,
        //         style,
        //         layout,
        //         cursor,
        //         viewport,
        //     )
        // });
        // renderer
    }

    // fn mouse_interaction(
    //     &self,
    //     state: &iced::advanced::widget::Tree,
    //     layout: iced::advanced::Layout<'_>,
    //     cursor: iced::advanced::mouse::Cursor,
    //     viewport: &iced::Rectangle,
    //     renderer: &Renderer,
    // ) -> iced::advanced::mouse::Interaction {
    //     TODO seek?
    // }
}

impl<'a, Message: 'a> From<Waveform<'a, Message>> for Element<'a, Message, Theme, Renderer> {
    fn from(track: Waveform<'a, Message>) -> Self {
        Self::new(track)
    }
}
