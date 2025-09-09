use iced::{
    advanced::{graphics::core::widget, layout, mouse, renderer, Layout, Widget},
    border, color,
    Color, Element, Length, Rectangle, Size,
};

pub struct ColorOption {
    color: Color,
}

impl ColorOption {
    pub fn new(color: Color) -> Self {
        Self { color }
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for ColorOption
where
    Renderer: renderer::Renderer,
{
    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Shrink,
            height: Length::Shrink,
        }
    }

    fn layout(
        &self,
        _tree: &mut widget::Tree,
        _renderer: &Renderer,
        _limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(Size::new(18.0, 18.0))
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
        renderer.fill_quad(
            renderer::Quad {
                bounds: layout.bounds(),
                border: border::rounded(4.0).width(1).color(color!(0x505050, 1.0)),
                ..renderer::Quad::default()
            },
            self.color,
        );
    }
}

impl<'a, Message, Theme, Renderer> From<ColorOption> for Element<'a, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
    fn from(circle: ColorOption) -> Self {
        Self::new(circle)
    }
}
