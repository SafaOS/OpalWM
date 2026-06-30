use crate::{
    Data, EventCtx, ShardEvent,
    render::{BoundingConstraints, BoundingRect, Padding, Point},
    shards::{AxisAlign, RenderCtx, Shard, ShardLayout, ShardNode, lifecycle::LifeCycle},
};

/// Describes the stack direction, horizontal or vertical.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Horizontal,
    Vertical,
}

impl Direction {
    #[inline]
    pub const fn skip(&self, at: f32) -> Point {
        match self {
            Self::Horizontal => Point::new(at, 0.),
            Self::Vertical => Point::new(0., at),
        }
    }

    #[inline]
    pub fn bounds_skip(&self, bounds: BoundingRect, padding: Padding) -> Point {
        match self {
            Self::Horizontal => self.skip(bounds.width()) + self.skip(padding.right),
            Self::Vertical => self.skip(bounds.height()) + self.skip(padding.bottom),
        }
    }

    #[inline]
    pub fn rev(&self) -> Self {
        match self {
            Self::Horizontal => Self::Vertical,
            Self::Vertical => Self::Horizontal,
        }
    }
}

/// Describes how space should be put.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Justify {
    #[default]
    /// Pack elements together with spaces at end.
    Start,
    /// Pack elements together with spaces at start.
    End,
    /// Pack together with space at both sides.
    Center,
    /// Puts equal space around each element.
    SpaceAround,
    /// Puts equal space between elements.
    SpaceBetween,
}

enum Element<S, M> {
    Normal {
        node: ShardNode<S, M>,
        flex_weight: f32,
    },
    Spacer {
        bounds: BoundingRect,
        flex_weight: f32,
    },
}

impl<S, M> Element<S, M> {
    pub fn node_mut(&mut self) -> Option<&mut ShardNode<S, M>> {
        match self {
            Self::Normal { node, .. } => Some(node),
            Self::Spacer { .. } => None,
        }
    }

    pub fn bounds(&self) -> BoundingRect {
        match self {
            Self::Normal { node, .. } => node
                .layout_ref()
                .expect("Attempt to access bounds of a layoutless node")
                .full_bounds(),
            Self::Spacer { bounds, .. } => *bounds,
        }
    }

    pub fn flex(&self) -> f32 {
        match self {
            Self::Normal { flex_weight, .. } | Self::Spacer { flex_weight, .. } => *flex_weight,
        }
    }
}

/// A container that dynamically distributes its children.
pub struct Stack<S = (), M = ()> {
    direction: Direction,
    default_align: AxisAlign,
    padding: Padding,
    justify_content: Justify,

    elements: Vec<Element<S, M>>,
    dirty: bool,
    layout_changed: bool,
    cursor_at: Option<Point>,
    last_constraints: BoundingConstraints,
    size: BoundingRect,
}

impl<T, M> Stack<T, M> {
    pub fn new(direction: Direction) -> Self {
        Self {
            direction,
            elements: Vec::new(),
            default_align: AxisAlign::default(),
            padding: Padding::equal(3.),
            justify_content: Justify::default(),
            layout_changed: true,
            dirty: true,
            cursor_at: None,
            size: BoundingRect::default(),
            last_constraints: BoundingConstraints::default(),
        }
    }

    pub fn row() -> Self {
        Self::new(Direction::Vertical)
    }

    pub fn column() -> Self {
        Self::new(Direction::Horizontal)
    }

    /// Sets the padding of the container.
    #[inline(always)]
    pub fn with_padding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self.layout_changed = true;
        self
    }

    /// Makes the container horizontal.
    #[inline(always)]
    pub fn horizontal(mut self) -> Self {
        self.direction = Direction::Horizontal;
        self.layout_changed = true;
        self
    }

    /// Makes the container vertical.
    #[inline(always)]
    pub fn vertical(mut self) -> Self {
        self.direction = Direction::Vertical;
        self.layout_changed = true;
        self
    }

    /// Aligns the container's children to the specified alignment.
    #[inline(always)]
    pub fn set_align(&mut self, alignment: AxisAlign) -> &mut Self {
        self.layout_changed = core::mem::replace(&mut self.default_align, alignment) != alignment;
        self
    }
    #[inline(always)]
    pub fn align(mut self, alignment: AxisAlign) -> Self {
        self.set_align(alignment);
        self
    }

    #[inline(always)]
    pub fn justify(mut self, justify: Justify) -> Self {
        self.justify_content = justify;
        self
    }

    /// Adds a [`Shard`] to this container.
    ///
    /// Currently behaves the same as [`Self::with_flex`] but with weight as 0
    #[inline]
    pub fn with<S: Shard<T, M> + 'static>(self, shard: S) -> Self {
        self.with_flex(shard, 0.)
    }

    /// Adds a [`Shard`] to this container.
    ///
    /// With its size being determined by a given flex weight.
    #[inline]
    pub fn with_flex<S: Shard<T, M> + 'static>(mut self, shard: S, weight: f32) -> Self {
        self.elements.push(Element::Normal {
            node: ShardNode::new(shard),
            flex_weight: weight,
        });
        self
    }

    /// Adds a flexible spacer with the given `weight` to Self.
    #[inline]
    pub fn with_spacer(mut self, weight: f32) -> Self {
        self.elements.push(Element::Spacer {
            bounds: BoundingRect::new(0., 0.),
            flex_weight: weight,
        });
        self
    }
}

impl<S, M> Shard<S, M> for Stack<S, M> {
    fn dirty(&self) -> bool {
        self.layout_changed || self.dirty
    }
    fn lifecycle(
        &mut self,
        _: &mut super::lifecycle::LifeCycleCtx,
        event: &LifeCycle,
        data: &Data<S, M>,
    ) {
        match event {
            LifeCycle::Init { .. } | LifeCycle::WindowMetaChanged { .. } => {
                for ele in &mut self.elements {
                    if let Some(node) = ele.node_mut() {
                        node.route_lifecycle(event, data);
                    }
                }
            }
            _ => {}
        }
    }

    fn layout(&mut self, ctx: &mut super::LayoutCtx) -> super::ShardLayout {
        let constraints = ctx.constraints();
        if core::mem::replace(&mut self.last_constraints, constraints) == constraints
            && !self.dirty()
        {
            return super::ShardLayout {
                bounds: self.size,
                ..Default::default()
            };
        }
        let stack_min = constraints.min();
        let stack_max = constraints.max();

        let stack_min_w = stack_min.width();
        let stack_min_h = stack_min.height();
        let stack_max_w = stack_max.width();
        let stack_max_h = stack_max.height();
        let padded_width = self.padding.padded_width();
        let padded_height = self.padding.padded_height();

        let mut layout_changed = false;

        let mut width_used = 0.;
        let mut height_used = 0.;
        let mut minor_width = 0.;
        let mut minor_height = 0.;
        let mut total_flex = 0.;

        for child in &mut self.elements {
            let flex = child.flex();
            if flex <= 0. {
                let constraints = BoundingConstraints::from_max(match self.direction {
                    Direction::Vertical => {
                        BoundingRect::new(stack_max_w - padded_width, f32::INFINITY)
                    }
                    Direction::Horizontal => {
                        BoundingRect::new(f32::INFINITY, stack_max_h - padded_height)
                    }
                });

                if let Some(node) = child.node_mut() {
                    let (child_layout, is_new) =
                        ctx.with_constraints(constraints, |ctx| node.layout(ctx));
                    layout_changed |= is_new;

                    if child_layout.align == AxisAlign::Default {
                        child_layout.align = self.default_align;
                    }

                    let c_w = child_layout.full_bounds().width();
                    let c_h = child_layout.full_bounds().height();

                    minor_height = c_h.max(minor_height);
                    minor_width = c_w.max(minor_width);
                    width_used += c_w + padded_width;
                    height_used += c_h + padded_height;
                }
            } else {
                total_flex += flex;
            }
        }

        let leftover = match self.direction {
            Direction::Vertical => stack_max_h - height_used,
            Direction::Horizontal => stack_max_w - width_used,
        }
        .max(0.);

        for child in &mut self.elements {
            let flex = child.flex();
            if flex > 0. {
                let share = leftover * (flex / total_flex);
                let child_max;
                let child_min;

                match self.direction {
                    Direction::Vertical => {
                        child_min = BoundingRect::new(minor_width, share - padded_height);
                        child_max = BoundingRect::new(f32::INFINITY, share - padded_height);
                    }

                    Direction::Horizontal => {
                        child_min = BoundingRect::new(share - padded_width, minor_height);
                        child_max = BoundingRect::new(share - padded_width, f32::INFINITY);
                    }
                }

                let c_w;
                let c_h;

                match child {
                    Element::Normal { node, .. } => {
                        let bc = BoundingConstraints::new(child_min, child_max);
                        let (child_layout, is_new) =
                            ctx.with_constraints(bc, |ctx| node.layout(ctx));
                        layout_changed |= is_new;

                        if child_layout.align == AxisAlign::Default {
                            child_layout.align = self.default_align;
                        }

                        c_w = child_layout.full_bounds().width();
                        c_h = child_layout.full_bounds().height();
                    }
                    Element::Spacer { bounds, .. } => {
                        *bounds = child_min;
                        c_w = bounds.width();
                        c_h = bounds.height();
                    }
                }

                minor_height = c_h.max(minor_height);
                minor_width = c_w.max(minor_width);
                width_used += c_w + padded_width;
                height_used += c_h + padded_height;
            }
        }

        let our_height = match self.direction {
            Direction::Horizontal => minor_height + padded_height,
            Direction::Vertical => height_used,
        }
        .max(stack_min_h)
        .min(stack_max_h);

        let our_width = match self.direction {
            Direction::Vertical => minor_width + padded_width,
            Direction::Horizontal => width_used,
        }
        .max(stack_min_w)
        .min(stack_max_w);

        let leftover_space = match self.direction {
            Direction::Vertical => our_height - height_used,
            Direction::Horizontal => our_width - width_used,
        };

        let our_bounds = BoundingRect::new(our_width, our_height);
        self.layout_changed = layout_changed;
        self.size = our_bounds;

        let stack_layout = ShardLayout {
            bounds: our_bounds,
            ..Default::default()
        };

        // Plot all elements.
        if self.layout_changed && !self.elements.is_empty() {
            let mut curr_pos = Point::default();
            let mut gap = 0.;
            match self.justify_content {
                Justify::Center => {
                    gap = leftover_space / 2.;
                }
                Justify::End => {
                    curr_pos = curr_pos + self.direction.skip(leftover_space);
                }
                Justify::Start => {}
                Justify::SpaceAround => {
                    gap = leftover_space / (self.elements.len() + 1) as f32;
                    curr_pos += self.direction.skip(gap);
                }
                Justify::SpaceBetween => {
                    gap = leftover_space / (self.elements.len() - 1) as f32;
                }
            }

            for ele in &mut self.elements {
                if ele.flex() < 0. {
                    continue;
                }

                let ele_bounds = ele.bounds();

                let rev_our_skip = self
                    .direction
                    .rev()
                    .bounds_skip(our_bounds, Padding::none());

                let rev_ele_skip = self
                    .direction
                    .rev()
                    .bounds_skip(ele_bounds, Padding::none());
                let ele_skip = self.direction.bounds_skip(ele_bounds, self.padding);

                if let Some(node) = ele.node_mut() {
                    let layout = node.layout.expect("No layout for node to place");
                    let align = layout.align;
                    let padding = layout.padding;
                    let added = Point::new(padding.left, padding.top);

                    match align {
                        AxisAlign::Default | AxisAlign::Start => {
                            node.plot_at(
                                curr_pos + Point::new(self.padding.left, self.padding.top) + added,
                            );
                        }
                        AxisAlign::End => {
                            node.plot_at(
                                ((curr_pos + Point::new(0., self.padding.top) + rev_our_skip)
                                    - rev_ele_skip)
                                    + added,
                            );
                        }
                        AxisAlign::Center => {
                            node.plot_at(
                                (curr_pos
                                    + Point::new(0., self.padding.top)
                                    + ((rev_our_skip - rev_ele_skip) / 2.))
                                    + added,
                            );
                        }
                    }
                }

                curr_pos += ele_skip + self.direction.skip(gap);
            }
        }
        stack_layout
    }

    fn on_event(&mut self, event_ctx: &mut EventCtx, event: &ShardEvent, data: &mut Data<S, M>) {
        for node in self.elements.iter_mut().filter_map(|e| e.node_mut()) {
            if event.is_mouse_event() {
                self.cursor_at = event_ctx.event_origin();
            }

            node.route_event(
                event_ctx.shard_origin(),
                event_ctx.event_origin(),
                event,
                data,
            );

            self.dirty |= node.is_dirty();
        }
    }

    fn on_ctx_update(&mut self, context: &Data<S, M>) {
        for node in self.elements.iter_mut().filter_map(|e| e.node_mut()) {
            node.on_ctx_update(context);
            self.dirty |= node.is_dirty();
        }
    }

    fn on_message(
        &mut self,
        _layout: &ShardLayout,
        pos: Point,
        state: &mut Data<S, M>,
        message: &M,
    ) {
        for node in self.elements.iter_mut().filter_map(|e| e.node_mut()) {
            node.route_message(pos, state, message);
            self.dirty |= node.is_dirty();
        }
    }

    fn render(&mut self, ctx: &mut RenderCtx, data: &Data<S, M>) -> Option<(Point, BoundingRect)> {
        let origin = ctx.origin();

        let mut results: Option<(Point, BoundingRect)>;
        let mut new_dirty = false;

        if core::mem::take(&mut self.layout_changed) {
            for node in self.elements.iter_mut().filter_map(|e| e.node_mut()) {
                node.render(ctx, true, self.cursor_at, data);
                new_dirty |= node.is_dirty();
            }

            results = None;
        } else {
            results = None;

            for node in self.elements.iter_mut().filter_map(|e| e.node_mut()) {
                if node.is_dirty() {
                    // Render and calculate damage
                    let ele_abs_pos = origin + node.position();

                    let layout = *node
                        .layout_ref()
                        .expect("Attempt to render before laying out elements");

                    node.render(ctx, false, self.cursor_at, data);
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
                    node.render(ctx, false, self.cursor_at, data);
                }

                new_dirty |= node.is_dirty();
            }
        }

        self.dirty = new_dirty;
        results
    }
}
