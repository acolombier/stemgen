use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{self, Tree};
use iced::advanced::{overlay, renderer, Clipboard, Shell};
use iced::mouse::Button;
use iced::{event, mouse};
use iced::{Color, Event, Point, Renderer, Theme, Vector};
use iced::{Element, Rectangle, Size};

pub struct ColorSelector<'a, Message> {
    pub(crate) color_select: &'a Box<dyn Fn(Option<Color>) -> Message + 'a>,
    pub(crate) viewport: Rectangle,
    pub(crate) picker: Element<'a, Message>,
    pub(crate) state: Tree,
}

impl<'a, Message> overlay::Overlay<Message, Theme, Renderer> for ColorSelector<'a, Message> {
    fn layout(&mut self, renderer: &Renderer, _: Size) -> layout::Node {
        let limits = layout::Limits::new(Size::new(141.0, 27.0), Size::new(141.0, 27.0));
        self.picker
            .as_widget()
            .layout(&mut self.state, renderer, &limits)
            .translate(Vector::new(self.viewport.x + 133.0, self.viewport.y + 13.0))
    }

    fn on_event(
        &mut self,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) -> event::Status {
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(Button::Left)) => {
                if cursor.position_over(layout.bounds()).is_some() {
                    // shell.publish((self.color_select)(Some(color!(0x124578, 1.0))));
                    let viewport = layout.bounds();
                    self.picker.as_widget_mut().on_event(
                        &mut self.state,
                        event,
                        layout,
                        cursor,
                        renderer,
                        clipboard,
                        shell,
                        &viewport,
                    );
                } else {
                    shell.publish((self.color_select)(None));
                }
                event::Status::Captured
            }
            _ => event::Status::Ignored,
        }
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        let viewport = layout.bounds();
        self.picker.as_widget().draw(
            &self.state,
            renderer,
            theme,
            style,
            layout,
            cursor,
            &viewport,
        );
    }

    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation<()>,
    ) {
        operation.container(None, layout.bounds(), &mut |operation| {
            self.picker
                .as_widget()
                .operate(&mut self.state, layout, renderer, operation);
        });
    }

    fn mouse_interaction(
        &self,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        mouse::Interaction::Pointer
    }

    fn is_over(&self, layout: Layout<'_>, _renderer: &Renderer, cursor_position: Point) -> bool {
        layout.bounds().contains(cursor_position)
    }
}
