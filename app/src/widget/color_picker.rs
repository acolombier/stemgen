use iced::{
    advanced::{
        graphics::core::widget,
        layout, mouse, renderer,
        widget::{tree, Tree},
        Clipboard, Layout, Shell, Widget,
    }, border, color, event, gradient, mouse::Button, window::Position, Color, Element, Event, Length, Radians, Rectangle, Renderer, Size
};

use crate::app::{Message, Modal};

pub struct SaturationPicker {
    color: Color,
}

impl SaturationPicker {
    pub fn new(color: Color) -> Self {
        Self { color }
    }
}

pub struct ColorPicker {
    color: Color,
}

impl ColorPicker {
    pub fn new(color: Color) -> Self {
        Self { color }
    }
}

#[derive(Clone, Copy)]
struct SaturationState {
    color: Color,
    value: (f32, f32),
    position: (f32, f32),
    clicked: bool,
}

impl SaturationState {
    pub fn current(&self) -> Color {
        let (r, g, b) = (self.color.r, self.color.g, self.color.b);
        let (rw, gw, bw) = (1.0 - r, 1.0 - g, 1.0 - b);
        Color::new(
            (r + (1.0 - self.position.0) * rw) * (1.0 - self.position.1),
            (g + (1.0 - self.position.0) * gw) * (1.0 - self.position.1),
            (b + (1.0 - self.position.0) * bw) * (1.0 - self.position.1),
            1.0,
        )
    }
    pub fn new(color: &Color) -> Self {
        let (mut r, mut g, mut b) = (color.r, color.g, color.b);
        let min = r.min(g).min(b);
        let max = r.max(g).max(b);

        if r == g && g == b {
            r = 1.0;
            g = 0.0;
            b = 0.0;
        } else {
            r -= min;
            g -= min;
            b -= min;
            let factor = 1.0 / (r + g + b);
            r *= factor;
            g *= factor;
            b *= factor;
        }
        Self {
            color: Color::new(r, g, b, 1.0),
            position: (1.0 - min, 1.0 - max),
            value: (1.0 - min, 1.0 - max),
            clicked: false,
        }
    }
}

#[derive(Clone, Copy)]
struct ColorState {
    value: f32,
    position: f32,
    clicked: bool,
}

impl ColorState {
    fn calculate(value: f32) -> Color {
        // .add_stop(0.0,  FF0000)
        // .add_stop(0.2,  FFFF00)
        // .add_stop(0.3,  00FF00)
        // .add_stop(0.45, 00FFFF)
        // .add_stop(0.6,  0000FF)
        // .add_stop(0.85, FF00FF)
        // .add_stop(1.0,  FF0000,
        let r = if value >= 0.3 && value <= 0.6 {
            0.0
        } else if value > 0.2 && value < 0.3 {
            1.0 - (value - 0.2) / 0.1
        } else if value < 0.85 && value > 0.6 {
            (value - 0.6) / 0.25
        } else {
            1.0
        };
        let g = if value >= 0.6 {
            0.0
        } else if value < 0.2 {
            value / 0.2
        } else if value > 0.45 {
            1.0 - (value - 0.45) / 0.15
        } else {
            1.0
        };
        let b = if value <= 0.3 || value == 1.0  {
            0.0
        } else if value < 0.45 {
            (value - 0.3) / 0.15
        } else if value > 0.85 {
            1.0 - (value - 0.85) / 0.15
        } else {
            1.0
        };
        Color::new(
            r,
            g,
            b,
            1.0,
        )
    }
    pub fn current(&self) -> Color {
        // .add_stop(0.0,  FF0000)
        // .add_stop(0.2,  FFFF00)
        // .add_stop(0.3,  00FF00)
        // .add_stop(0.45, 00FFFF)
        // .add_stop(0.6,  0000FF)
        // .add_stop(0.85, FF00FF)
        // .add_stop(1.0,  FF0000,
        Self::calculate(self.position)
    }
    pub fn saved(&self) -> Color {
        Self::calculate(self.value)
    }
}

impl ColorState {
    pub fn new(color: &Color) -> Self {
        let (mut r, mut g, mut b) = (color.r, color.g, color.b);
        let min = r.min(g).min(b);

        if r == g && g == b {
            r = 1.0;
            g = 0.0;
            b = 0.0;
        } else {
            r -= min;
            g -= min;
            b -= min;
            let factor = 1.0 / (r + g + b);
            r *= factor;
            g *= factor;
            b *= factor;
        }
        let value = if b == 0.0 {
            0.2 - 0.2 * r + 0.15 * g
        } else if r == 0.0 {
            0.45 - 0.15 * g + 0.15 * b
        } else if g == 0.0 {
            0.85 - 0.25 * b + 0.15 * r
        } else {
            panic!("color is null");
        };
        Self {
            value,
            position: value,
            clicked: false,
        }
    }
}

impl<Theme> Widget<Message, Theme, Renderer> for SaturationPicker {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<SaturationState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(SaturationState::new(&self.color))
    }

    fn mouse_interaction(
        &self,
        _state: &Tree,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        mouse::Interaction::Pointer
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fill,
            height: Length::Fixed(140.0),
        }
    }

    fn diff(&self, tree: &mut Tree) {
        let state = tree.state.downcast_mut::<SaturationState>();
        let clicked = state.clicked;
        *state = SaturationState::new(&self.color);
        state.clicked = clicked;
    }

    fn layout(
        &self,
        _tree: &mut widget::Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.max_height(140.0).max())
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> event::Status {
        let state = tree.state.downcast_mut::<SaturationState>();

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(Button::Left)) => {
                state.clicked = true;
                if state.position != state.value {
                    shell.publish(Message::Modal(Modal::ColorPicker(state.current())));
                }
                // shell.request_redraw(RedrawRequest::NextFrame);
            }
            Event::Mouse(mouse::Event::ButtonReleased(Button::Left)) => {
                state.clicked = false;
                // shell.request_redraw(RedrawRequest::NextFrame);
            }

            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if layout.bounds().expand(7.5).contains(position) {
                    let size = layout.bounds().size();
                    let position = position - layout.bounds().position();
                    state.position = ((position.x / size.width).max(0.0).min(1.0), (position.y / size.height).max(0.0).min(1.0));
                    if state.clicked {
                        shell.publish(Message::Modal(Modal::ColorPicker(state.current())));
                    }
                } else {
                    state.position =state.value;
                }
            }
            _ => {}
        }

        event::Status::Ignored
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        use iced::advanced::Renderer as _;
        let state = tree.state.downcast_ref::<SaturationState>();

        renderer.fill_quad(
            renderer::Quad {
                bounds: layout.bounds(),
                ..renderer::Quad::default()
            },
            gradient::Linear::new(Radians::PI / 2.00)
                .add_stop(0.0, color!(0xffffff, 1.0))
                .add_stop(1.0, state.color),
        );
        renderer.fill_quad(
            renderer::Quad {
                bounds: layout.bounds(),
                ..renderer::Quad::default()
            },
            gradient::Linear::new(Radians::PI)
                .add_stop(0.0, color!(0x000000, 0.0))
                .add_stop(1.0, color!(0x000000, 1.0)),
        );
        // renderer.fill_quad(
        //     renderer::Quad {
        //         bounds: Rectangle {
        //             x: bounds.x + bounds.width * state.position.0 - 6.5,
        //             y: bounds.y + bounds.height * state.position.1 - 6.5,
        //             width: 13.0,
        //             height: 13.0,
        //         },
        //         border: border::color(Color::BLACK).width(1).rounded(6.5),
        //         ..renderer::Quad::default()
        //     },
        //     state.current(),
        // );
    }
}

impl<Theme> Widget<Message, Theme, Renderer> for ColorPicker {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<ColorState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(ColorState::new(&self.color))
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fixed(20.0),
            height: Length::Fixed(140.0),
        }
    }

    fn diff(&self, tree: &mut Tree) {
        let state = tree.state.downcast_mut::<ColorState>();
        let clicked = state.clicked;
        let position = state.position;
        *state = ColorState::new(&self.color);
        state.clicked = clicked;
        if position == 1.0 && (state.value * 100.0).round() == 0.0 {
            state.value = 1.0;
        }
    }

    fn layout(
        &self,
        _tree: &mut widget::Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.max_height(140.0).max_width(20.0).max())
    }

    fn mouse_interaction(
        &self,
        _state: &Tree,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        mouse::Interaction::Pointer
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> event::Status {
        let state = tree.state.downcast_mut::<ColorState>();

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(Button::Left)) if state.position != state.value => {
                state.clicked = true;
                shell.publish(Message::Modal(Modal::ColorPicker(state.current())));
                // shell.request_redraw(RedrawRequest::NextFrame);
            }
            Event::Mouse(mouse::Event::ButtonReleased(Button::Left)) => {
                state.clicked = false;
                // shell.request_redraw(RedrawRequest::NextFrame);
            }

            Event::Mouse(mouse::Event::CursorMoved { position }) if layout.bounds().expand(7.5).contains(position) => {
                let size = layout.bounds().size();
                let position = position - layout.bounds().position();
                state.position = (position.y / size.height).max(0.0).min(1.0);
                if state.clicked {
                    shell.publish(Message::Modal(Modal::ColorPicker(state.current())));
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                state.position =state.value;
            }
            _ => {}
        }

        event::Status::Ignored
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        use iced::advanced::Renderer as _;
        let state = tree.state.downcast_ref::<ColorState>();
        let bounds = layout.bounds();
        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    x: bounds.x + bounds.width / 2.0 - 2.5,
                    y: bounds.y,
                    width: 5.0,
                    height: bounds.height,
                },
                border: border::rounded(2),
                ..renderer::Quad::default()
            },
            gradient::Linear::new(Radians::PI)
                .add_stop(0.0, color!(0xFF0000, 1.0))
                .add_stop(0.2, color!(0xFFFF00, 1.0))
                .add_stop(0.3, color!(0x00FF00, 1.0))
                .add_stop(0.45, color!(0x00FFFF, 1.0))
                .add_stop(0.6, color!(0x0000FF, 1.0))
                .add_stop(0.85, color!(0xFF00FF, 1.0))
                .add_stop(1.0, color!(0xFF0000, 1.0)),
        );
        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    x: bounds.x + bounds.width / 2.0 - 7.5,
                    y: bounds.y + bounds.height * state.value - 7.5,
                    width: 15.0,
                    height: 15.0,
                },
                border: border::rounded(7.5),
                ..renderer::Quad::default()
            },
            state.saved(),
        );
    }
}

impl<'a, Theme> From<SaturationPicker> for Element<'a, Message, Theme, Renderer> {
    fn from(picker: SaturationPicker) -> Self {
        Self::new(picker)
    }
}
impl<'a, Theme> From<ColorPicker> for Element<'a, Message, Theme, Renderer> {
    fn from(picker: ColorPicker) -> Self {
        Self::new(picker)
    }
}
