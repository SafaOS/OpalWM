use crate::{
    AppCtx, EventCtx, ShardEvent,
    render::{Alignment, BoundingConstraints, BoundingRect, Color, Padding, PaintBrush, Point},
    shards::{RenderCtx, Shard, ShardLayout, ShardNode},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ItemsDisplay {
    Vertical,
    Horrizontal,
    Grid(u16, u16),
}

/// Describes how a container is displayed.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Display {
    items_disp: ItemsDisplay,
    items_align: Alignment,
    padding: Padding,
    bg: Color,
    max_width: Option<f32>,
    max_height: Option<f32>,
}

impl Display {
    pub const fn new() -> Self {
        Self {
            items_align: Alignment::Default,
            items_disp: ItemsDisplay::Vertical,
            padding: Padding::equal(3.),
            max_width: None,
            max_height: None,
            bg: Color::NONE,
        }
    }

    #[inline(always)]
    pub const fn align(mut self, alignment: Alignment) -> Self {
        self.items_align = alignment;
        self
    }

    #[inline(always)]
    pub const fn with_padding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }

    #[inline(always)]
    pub const fn horizontal(mut self) -> Self {
        self.items_disp = ItemsDisplay::Horrizontal;
        self
    }

    #[inline(always)]
    pub const fn vertical(mut self) -> Self {
        self.items_disp = ItemsDisplay::Vertical;
        self
    }

    #[inline(always)]
    pub const fn grid(mut self, columns: u16, rows: u16) -> Self {
        self.items_disp = ItemsDisplay::Grid(columns, rows);
        self
    }

    #[inline(always)]
    pub const fn with_bg(mut self, color: Color) -> Self {
        self.bg = color;
        self
    }
}

/// A container that dynamically distributes its children.
pub struct Stack<Ctx: AppCtx> {
    display: Display,
    elements: Vec<ShardNode<Ctx>>,
    layout_changed: bool,
    cursor_at: Option<Point>,
}

impl<Ctx: AppCtx> Stack<Ctx> {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            display: Display::new(),
            layout_changed: true,
            cursor_at: None,
        }
    }

    /// Sets the padding of the container.
    #[inline(always)]
    pub fn with_padding(mut self, padding: Padding) -> Self {
        self.display(self.display.with_padding(padding));
        self
    }

    /// Makes the container horizontal.
    #[inline(always)]
    pub fn horizontal(mut self) -> Self {
        self.display(self.display.horizontal());
        self
    }

    /// Makes the container vertical.
    #[inline(always)]
    pub fn vertical(mut self) -> Self {
        self.display(self.display.vertical());
        self
    }

    /// Makes the container a grid.
    #[inline(always)]
    pub fn grid(mut self, columns: u16, rows: u16) -> Self {
        self.display(self.display.grid(columns, rows));
        self
    }

    /// Sets the background color of the container.
    #[inline(always)]
    pub fn with_bg(mut self, color: Color) -> Self {
        self.display(self.display.with_bg(color));
        self
    }

    /// Aligns the container's children to the specified alignment.
    #[inline(always)]
    pub fn set_items_align(&mut self, alignment: Alignment) -> &mut Self {
        self.display(self.display.align(alignment));
        self
    }

    fn display(&mut self, display: Display) {
        if core::mem::replace(&mut self.display, display) != display {
            self.layout_changed = true;
        }
    }

    /// Adds a [`Shard`] to this container.
    #[inline]
    pub fn with<S: Shard<Ctx> + 'static>(mut self, shard: S) -> Self {
        self.elements.push(ShardNode::new(shard));

        self
    }

    #[inline]
    fn plot_elements(
        &mut self,
        our_layout: &ShardLayout,
        mut with_ele: impl FnMut(&mut ShardNode<Ctx>, Point),
    ) {
        let padding = self.display.padding;
        let curr_x = padding.left;
        let mut curr_y = padding.top;

        for ele in self.elements.iter_mut() {
            let layout = ele
                .layout
                .as_ref()
                .expect("Attempt to plot elements before laying them out");

            let align = if layout.alignment == Alignment::Default {
                our_layout.alignment
            } else {
                layout.alignment
            };

            let x;
            let y;

            match (self.display.items_disp, align) {
                (ItemsDisplay::Grid(cols, rows), _) => todo!("Grid: {cols}=>{rows}"),
                (ItemsDisplay::Vertical, Alignment::Left | Alignment::Default) => {
                    let bounds = layout.bounds_with_padding();
                    x = curr_x;
                    y = curr_y;

                    curr_y += bounds.height() + padding.bottom;
                }
                (ItemsDisplay::Vertical, Alignment::Right) => {
                    let bounds = layout.bounds_with_padding();
                    x = our_layout.bounds.width() - bounds.width() - padding.right;
                    y = curr_y;

                    curr_y += bounds.height() + padding.bottom;
                }
                (ItemsDisplay::Vertical, Alignment::Center) => {
                    let bounds = layout.bounds_with_padding();
                    x = (our_layout.bounds.width() - (bounds.width() + padding.padded_width()))
                        / 2.;
                    y = curr_y;

                    curr_y += bounds.height() + padding.bottom;
                }
                (ItemsDisplay::Horrizontal, _) => todo!(),
            }

            with_ele(&mut *ele, Point::new(x, y))
        }
    }
}

impl<Ctx: AppCtx> Shard<Ctx> for Stack<Ctx> {
    fn dirty(&self) -> bool {
        self.layout_changed || self.elements.iter().any(|ele| ele.is_dirty())
    }

    fn layout(&mut self, ctx: &mut super::LayoutCtx) -> super::ShardLayout {
        let constraints = ctx.constraints();
        let default_alignment = self.display.items_align;

        let cont_max_width = constraints
            .max()
            .width()
            .min(self.display.max_width.unwrap_or(f32::MAX));
        let cont_max_height = constraints
            .max()
            .height()
            .min(self.display.max_height.unwrap_or(f32::MAX));

        let padding = self.display.padding;

        let ele_count = self.elements.len() as f32;
        let mut ele_max_width = ((cont_max_width / ele_count) - padding.padded_width()).max(0.);
        let mut ele_max_height = ((cont_max_height / ele_count) - padding.padded_height()).max(0.);

        let mut layout_changed = self.layout_changed;
        for ele in self.elements.iter_mut() {
            let max_size = BoundingRect::new(ele_max_width, ele_max_height);
            ctx.with_constraints(BoundingConstraints::from_max(max_size), |ele_ctx| {
                let (layout, is_new_layout) = ele.layout(ele_ctx, |new| {
                    if new.alignment == Alignment::Default {
                        new.alignment = default_alignment;
                    }
                });

                if layout.bounds_with_padding().width() < ele_max_width {
                    ele_max_width += ele_max_width - layout.bounds.width();
                }

                if layout.bounds_with_padding().height() < ele_max_height {
                    ele_max_height += ele_max_height - layout.bounds.height();
                }

                layout_changed |= is_new_layout;
            });
        }

        self.layout_changed = layout_changed;
        let our_layout = ShardLayout {
            bounds: BoundingRect::new(cont_max_width, cont_max_height),
            padding: Padding::none(),
            alignment: Alignment::Default,
        };

        if layout_changed {
            self.plot_elements(&our_layout, |ele, origin| ele.plot_at(origin));
        }

        our_layout
    }

    fn on_event(&mut self, event_ctx: &mut EventCtx, event: &ShardEvent, app_ctx: &mut Ctx) {
        for ele in &mut self.elements {
            if event.is_mouse_event() {
                self.cursor_at = event_ctx.event_origin();
            }

            ele.route_event(
                event_ctx.shard_origin(),
                event_ctx.event_origin(),
                event,
                app_ctx,
            );
        }
    }

    fn on_ctx_update(&mut self, context: &Ctx) {
        for shard in &mut self.elements {
            shard.on_ctx_update(context);
        }
    }

    fn on_message(
        &mut self,
        _layout: &ShardLayout,
        pos: Point,
        state: &mut Ctx,
        message: &Ctx::Message,
    ) {
        for ele in &mut self.elements {
            ele.route_message(pos, state, message);
        }
    }

    fn render(&mut self, ctx: &mut RenderCtx) -> Option<(Point, BoundingRect)> {
        let layout = *ctx.layout();
        let origin = ctx.origin();

        let paint = PaintBrush::Color(self.display.bg);
        let mut results: Option<(Point, BoundingRect)>;

        if core::mem::take(&mut self.layout_changed) {
            ctx.clear(&paint, layout.bounds);
            for ele in &mut self.elements {
                ele.render(ctx, true, self.cursor_at);
            }

            results = None;
        } else {
            results = None;
            ctx.clear(&paint, layout.bounds);

            for ele in &mut self.elements {
                if ele.is_dirty() {
                    // Render and calculate damage
                    let ele_abs_pos = origin + ele.position();

                    let layout = *ele
                        .layout_ref()
                        .expect("Attempt to render before laying out elements");

                    ele.render(ctx, false, self.cursor_at);
                    if let Some((d_pos, d_rect)) = results.as_mut() {
                        let d_last_x = d_pos.x() + d_rect.width();
                        let d_last_y = d_pos.y() + d_rect.height();

                        let x = d_pos.x().min(ele_abs_pos.x());
                        let y = d_pos.y().min(ele_abs_pos.y());
                        let w = d_last_x.max(ele_abs_pos.x() + layout.bounds.width()) - x;
                        let h = d_last_y.max(ele_abs_pos.y() + layout.bounds.height()) - y;

                        *d_pos = Point::new(x, y);
                        *d_rect = BoundingRect::new(w, h);
                    } else {
                        results = Some((ele_abs_pos, layout.bounds));
                    }
                } else {
                    // Render only
                    ele.render(ctx, false, self.cursor_at);
                }
            }
        }

        results
    }
}
