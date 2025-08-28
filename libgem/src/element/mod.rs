use std::any::Any;

use libopal::window::Pixel;

use crate::{Gem, canvas::DrawingCanvas};

pub mod button;
pub mod container;
pub mod image;
pub mod text_box;

pub(crate) const fn is_inside_rect(
    x: u32,
    y: u32,
    rect_x: u32,
    rect_y: u32,
    rect_width: u32,
    rect_height: u32,
) -> bool {
    x >= rect_x && x < rect_x + rect_width && y >= rect_y && y < rect_y + rect_height
}

pub trait Element<RootCanvas: DrawingCanvas, G: Gem>: Any {
    /// The amount of pixels this element takes up from the x axis, not including padding.
    fn draw_width(&self) -> u32;
    /// The amount of pixels this element takes up from the y axis, not including padding.
    fn draw_height(&self) -> u32;
    /// The amount of pixels this element takes up from the x axis, including padding.
    fn container_width(&self) -> u32;
    /// The amount of pixels this element takes up from the y axis, including padding.
    fn container_height(&self) -> u32;

    /// Draws the element onto the canvas, given a relative position of the element from the canvas.
    /// Returns either None or the end position of the element as if it was a rectangle, and that is (x, y) where x is the rightmost x coordinate and y is the lowest y coordinate of the element.
    ///
    /// The `bg_color` parameter specifies the background color that the element is supposed to draw on, before drawing the element you likely want to draw the background color without alpha-blending first.
    fn draw(
        &mut self,
        canvas: &mut RootCanvas,
        x: u32,
        y: u32,
        bg_color: Pixel,
    ) -> Option<(u32, u32)>;
    /// Returns true if the element needs to be redrawn.
    fn needs_redraw(&self) -> bool;
    /// Handles an event for the element, given a relative position of the element from the canvas.
    fn handle_event(&mut self, gem: &mut G, event: libopal::Event, ele_x: u32, ele_y: u32) {
        _ = gem;
        _ = event;
        _ = ele_x;
        _ = ele_y;
    }
}
