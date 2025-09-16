use std::u32;

use libopal::window::Pixel;

use crate::{Gem, canvas::DrawingCanvas, element::Element};

/// A grid layout for arranging elements in a grid of rows.
#[derive(Debug, Clone, Copy)]
pub struct GridLayout {
    elements_per_row: u32,
}

impl GridLayout {
    /// Constructs a new grid layout with default values.
    pub const fn new() -> Self {
        GridLayout {
            elements_per_row: u32::MAX,
        }
    }

    /// Constructs a new grid layout with a specified number of elements per row.
    pub const fn with_elements_per_row(mut self, elements_per_row: u32) -> Self {
        self.elements_per_row = elements_per_row;
        self
    }
}

/// A vertical layout for arranging elements in a vertical view.
#[derive(Debug, Clone, Copy)]
pub struct VerticalLayout {
    align_center: bool,
}

impl VerticalLayout {
    /// Constructs a new vertical layout with default values.
    pub const fn new() -> Self {
        VerticalLayout {
            align_center: false,
        }
    }

    /// Constructs a new vertical layout with a specified alignment.
    pub const fn with_align_center(mut self, align_center: bool) -> Self {
        self.align_center = align_center;
        self
    }
}

#[derive(Debug, Clone, Copy)]
/// Describes how a container should be laid out.
pub enum ContainerLayout {
    Grid(GridLayout),
    Vertical(VerticalLayout),
}

impl ContainerLayout {
    /// Constructs a new container layout with default values.
    pub const fn default() -> Self {
        ContainerLayout::Vertical(VerticalLayout::new())
    }
}

#[derive(Debug, Clone, Copy)]
/// Describes the styles of a container.
pub struct ContainerStyles {
    element_padding: u32,
    layout: ContainerLayout,
}

impl ContainerStyles {
    const DEFAULT_ELEMENT_PADDING: u32 = 3;

    /// Constructs a new container styles with default values.
    pub const fn new() -> Self {
        ContainerStyles {
            element_padding: Self::DEFAULT_ELEMENT_PADDING,
            layout: ContainerLayout::default(),
        }
    }

    /// Constructs a new container styles with a specified number of pixels of padding between elements.
    pub const fn with_element_padding(mut self, element_padding: u32) -> Self {
        self.element_padding = element_padding;
        self
    }

    /// Constructs a new container styles with a specified layout.
    pub const fn with_layout(mut self, layout: ContainerLayout) -> Self {
        self.layout = layout;
        self
    }
}

/// A customizable container of elements, that handles their layout and such.
pub struct Container<Canvas: DrawingCanvas + 'static, G: Gem> {
    elements: Vec<Box<dyn Element<Canvas, G>>>,
    /* FIXME: Save element height and width information and set this true if these were changed */
    elements_changed: bool,
    max_width: u32,
    max_height: u32,
    styles: ContainerStyles,
}

impl<Canvas: DrawingCanvas + 'static, G: Gem> Container<Canvas, G> {
    pub const fn new(styles: ContainerStyles, max_width: u32, max_height: u32) -> Self {
        Container {
            elements: Vec::new(),
            elements_changed: false,
            max_width,
            max_height,
            styles,
        }
    }

    pub const fn styles(&self) -> ContainerStyles {
        self.styles
    }

    pub const fn set_styles(&mut self, styles: ContainerStyles) {
        self.styles = styles;
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

    fn layout_elements<F: FnMut(&mut dyn Element<Canvas, G>, u32, u32)>(
        &mut self,
        start_x: u32,
        start_y: u32,
        mut on_element: F,
    ) {
        let mut curr_x = start_x;
        let mut curr_y = start_y;

        match self.styles.layout {
            ContainerLayout::Grid(g) => {
                let biggest_ele_height = self
                    .elements
                    .iter()
                    .map(|ele| ele.container_height())
                    .max()
                    .unwrap_or(0);

                let max_columns = g.elements_per_row;

                let mut curr_col = 0;
                for element in self.elements.iter_mut() {
                    let ele_width = element.container_width();

                    if curr_col + 1 >= max_columns
                        || (curr_x - start_x) + ele_width > self.max_width
                    {
                        curr_col = 0;
                        curr_x = start_x;
                        curr_y += biggest_ele_height + self.styles.element_padding;
                    } else {
                        curr_col += 1;
                    }

                    on_element(&mut **element, curr_x, curr_y);
                    curr_x += ele_width + self.styles.element_padding;
                }
            }
            ContainerLayout::Vertical(v) => {
                let is_centered = v.align_center;

                for element in self.elements.iter_mut() {
                    let element_height = element.container_height();

                    let (element_x, element_y) = if is_centered {
                        let element_width = element.container_width();

                        let element_x = curr_x + (self.max_width.saturating_sub(element_width)) / 2;
                        let element_y = curr_y;

                        (element_x, element_y)
                    } else {
                        (curr_x, curr_y)
                    };

                    on_element(&mut **element, element_x, element_y);
                    curr_y += element_height + self.styles.element_padding;
                }
            }
        }
    }
}

impl<Canvas: DrawingCanvas + 'static, G: Gem> Element<Canvas, G> for Container<Canvas, G> {
    fn draw(
        &mut self,
        canvas: &mut Canvas,
        start_x: u32,
        start_y: u32,
        bg_color: Pixel,
    ) -> (Option<(u32, u32)>, Option<(u32, u32)>) {
        let mut draw_ended_at = None;
        let mut draw_started_at = None;

        let elements_changed = self.elements_changed;
        self.layout_elements(start_x, start_y, |ele, draw_x, draw_y| {
            if elements_changed || ele.needs_redraw() {
                let (draw_start, draw_end) = ele.draw(canvas, draw_x, draw_y, bg_color);

                match (draw_end, draw_ended_at) {
                    (None, None) => (),
                    (Some((x, y)), Some((x2, y2))) => {
                        draw_ended_at = Some((x.max(x2), y.max(y2)));
                    }
                    (Some((x, y)), None) => {
                        draw_ended_at = Some((x, y));
                    }
                    (None, Some(_)) => {}
                }

                match (draw_start, draw_started_at) {
                    (None, None) => (),
                    (Some((x, y)), Some((x2, y2))) => {
                        draw_started_at = Some((x.min(x2), y.min(y2)));
                    }
                    (s @ Some(_), None) => draw_started_at = s,
                    (None, Some(_)) => {}
                }
            }
        });

        self.elements_changed = false;
        (draw_started_at, draw_ended_at)
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
        self.layout_elements(start_x, start_y, |ele, ele_x, ele_y| {
            ele.handle_event(gem, event, ele_x, ele_y);
        });
    }
}
