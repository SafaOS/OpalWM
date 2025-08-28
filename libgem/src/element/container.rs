use libopal::window::Pixel;

use crate::{Gem, canvas::DrawingCanvas, element::Element};

#[derive(Debug, Clone, Copy)]
/// Describes how a container should be laid out.
pub enum ContainerLayout {
    Horizontal,
    Vertical {
        /// Aligns the elements in the center of the container.
        align_center: bool,
    },
}

/// A customizable container of elements, that handles their layout and such.
pub struct Container<Canvas: DrawingCanvas + 'static, G: Gem> {
    layout: ContainerLayout,
    elements: Vec<Box<dyn Element<Canvas, G>>>,
    /* FIXME: Save element height and width information and set this true if these were changed */
    elements_changed: bool,
    max_width: u32,
    max_height: u32,
}

impl<Canvas: DrawingCanvas + 'static, G: Gem> Container<Canvas, G> {
    pub const fn new(layout: ContainerLayout, max_width: u32, max_height: u32) -> Self {
        Container {
            layout,
            elements: Vec::new(),
            elements_changed: false,
            max_width,
            max_height,
        }
    }

    pub const fn set_layout(&mut self, layout: ContainerLayout) {
        self.layout = layout;
        self.elements_changed = true;
    }

    #[must_use]
    /// Adds an element to the container and returns its index.
    pub fn add_element(&mut self, element: Box<dyn Element<Canvas, G>>) -> usize {
        self.elements.push(element);
        self.elements_changed = true;
        self.elements.len() - 1
    }

    /// Attempts to get at index as the specified type.
    pub fn get_element_as<T: Element<Canvas, G>>(&self, index: usize) -> Option<&T> {
        let any: &dyn std::any::Any = &self.elements[index];
        any.downcast_ref()
    }

    /// Attempts to get at index as the specified type muttably.
    pub fn get_element_as_mut<T: Element<Canvas, G>>(&mut self, index: usize) -> Option<&mut T> {
        let any: &mut dyn std::any::Any = &mut *self.elements[index];
        any.downcast_mut()
    }
}

impl<Canvas: DrawingCanvas + 'static, G: Gem> Element<Canvas, G> for Container<Canvas, G> {
    fn draw(
        &mut self,
        canvas: &mut Canvas,
        start_x: u32,
        start_y: u32,
        bg_color: Pixel,
    ) -> Option<(u32, u32)> {
        let mut ele_x = start_x;
        let mut ele_y = start_y;

        let is_centered = matches!(
            self.layout,
            ContainerLayout::Vertical { align_center: true }
        );
        let mut draw_ended_at = None;

        let biggest_ele_height = self
            .elements
            .iter()
            .map(|ele| ele.container_height())
            .max()
            .unwrap_or(0);

        for element in self.elements.iter_mut() {
            let draw_x = if is_centered {
                let element_width = element.draw_width();
                let container_width = self.max_width;
                (container_width.saturating_sub(element_width)) / 2
            } else {
                if (ele_x - start_x) + element.draw_width() > self.max_width {
                    ele_x = start_x;
                    ele_y += biggest_ele_height;
                }

                ele_x
            };
            let draw_y = ele_y;

            if self.elements_changed || element.needs_redraw() {
                let results = element.draw(canvas, draw_x, draw_y, bg_color);

                match (results, draw_ended_at) {
                    (None, None) => (),
                    (Some((x, y)), Some((x2, y2))) => {
                        draw_ended_at = Some((x.max(x2), y.max(y2)));
                    }
                    (Some((x, y)), None) => {
                        draw_ended_at = Some((x, y));
                    }
                    (None, Some(_)) => {}
                }
            }

            match self.layout {
                ContainerLayout::Horizontal => {
                    ele_x += element.container_width();
                }
                ContainerLayout::Vertical { .. } => {
                    ele_y += element.container_height();
                }
            }
        }

        self.elements_changed = false;
        draw_ended_at
    }

    fn needs_redraw(&self) -> bool {
        self.elements_changed || self.elements.iter().any(|ele| ele.needs_redraw())
    }

    #[inline]
    fn draw_height(&self) -> u32 {
        self.elements
            .iter()
            .map(|element| element.draw_height())
            .sum()
    }

    #[inline]
    fn draw_width(&self) -> u32 {
        self.elements
            .iter()
            .map(|element| element.draw_width())
            .max()
            .unwrap_or(0)
    }

    #[inline]
    fn container_height(&self) -> u32 {
        self.draw_height()
    }

    #[inline]
    fn container_width(&self) -> u32 {
        self.draw_width()
    }

    fn handle_event(&mut self, gem: &mut G, event: libopal::Event, start_x: u32, start_y: u32) {
        let mut ele_x = start_x;
        let mut ele_y = start_y;

        let is_centered = matches!(
            self.layout,
            ContainerLayout::Vertical { align_center: true }
        );

        let biggest_ele_height = self
            .elements
            .iter()
            .map(|ele| ele.container_height())
            .max()
            .unwrap_or(0);

        for element in self.elements.iter_mut() {
            let draw_x = if is_centered {
                let element_width = element.draw_width();
                let container_width = self.max_width;
                (container_width - element_width) / 2
            } else {
                if (ele_x - start_x) + element.draw_width() > self.max_width {
                    ele_x = start_x;
                    ele_y += biggest_ele_height;
                }

                ele_x
            };

            element.handle_event(gem, event, draw_x, ele_y);
            match self.layout {
                ContainerLayout::Horizontal => {
                    ele_x += element.container_width();
                }
                ContainerLayout::Vertical { .. } => {
                    ele_y += element.container_height();
                }
            }
        }
    }
}
