use crate::span_tree::{SpanNode, SpanTree};
use eframe::egui;
use egui::*;
use rr_data::CallsiteId;

type NanoSecond = i64;

const HOVER_COLOR: Rgba = Rgba::from_rgb(0.8, 0.8, 0.8);

// ----------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
struct Filter {
    filter: String,
}

impl Filter {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Filter:");
            ui.text_edit_singleline(&mut self.filter);
            self.filter = self.filter.to_lowercase();
            if ui.button("ｘ").clicked() {
                self.filter.clear();
            }
        });
    }

    /// if true, show everything
    fn is_empty(&self) -> bool {
        self.filter.is_empty()
    }

    fn include(&self, id: &str) -> bool {
        if self.filter.is_empty() {
            true
        } else {
            id.to_lowercase().contains(&self.filter)
        }
    }
}

// ----------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
struct PaintResult {
    rect: Rect,
    color: Option<Rgba>,
}

// ----------------------------------------------------------------------------

/// Context for painting a frame.
struct Info {
    ctx: egui::Context,
    /// Bounding box of canvas in points:
    canvas: egui::Rect,
    /// Interaction with the profiler canvas
    response: egui::Response,
    painter: egui::Painter,

    /// Time of first event
    min_ns: NanoSecond,
    /// Time of last event
    max_ns: NanoSecond,

    text_height: f32,

    font_id: egui::FontId,
}

impl Info {
    fn point_from_ns(&self, options: &FlameGraph, ns: NanoSecond) -> f32 {
        self.canvas.min.x
            + options.sideways_pan_in_points
            + self.canvas.width() * ((ns - self.min_ns) as f32) / options.canvas_width_ns
    }
}

// ----------------------------------------------------------------------------

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct FlameGraph {
    /// Controls zoom. 0 => reset to default
    pub canvas_width_ns: f32,

    /// How much we have panned sideways:
    pub sideways_pan_in_points: f32,

    // --------------------
    // Visuals:
    /// Events shorter than this many points aren't painted
    pub cull_width: f32,
    /// Draw each item with at least this width (only makes sense if [`Self::cull_width`] is 0)
    pub min_width: f32,

    pub rect_height: f32,
    pub spacing: f32,
    pub rounding: f32,

    // --------------------
    #[serde(skip)]
    filter: Filter,

    /// Used to animate zoom+pan.
    ///
    /// First part is `now()`, second is range.
    #[serde(skip)]
    zoom_to_relative_ns_range: Option<(f64, (NanoSecond, NanoSecond))>,
}

impl Default for FlameGraph {
    fn default() -> Self {
        Self {
            canvas_width_ns: 0.0,
            sideways_pan_in_points: 0.0,

            // cull_width: 0.5, // save some CPU?
            cull_width: 0.0, // no culling
            min_width: 1.0,

            rect_height: 16.0,
            spacing: 2.0,
            rounding: 3.0,

            filter: Default::default(),

            zoom_to_relative_ns_range: None,
        }
    }
}

impl FlameGraph {
    pub fn ui(&mut self, ui: &mut egui::Ui, span_tree: &SpanTree) {
        self.filter.ui(ui);
        flamegraph_ui(self, ui, span_tree);
    }
}

// ----------------------------------------------------------------------------

fn flamegraph_ui(options: &mut FlameGraph, ui: &mut egui::Ui, span_tree: &SpanTree) {
    Frame::dark_canvas(ui.style()).show(ui, |ui| {
        let available_height = ui.max_rect().bottom() - ui.min_rect().bottom();
        ScrollArea::vertical().show(ui, |ui| {
            let mut canvas_rect = ui.available_rect_before_wrap();
            canvas_rect.max.y = f32::INFINITY;
            let response = ui.interact(canvas_rect, ui.id(), Sense::click_and_drag());

            let (min_ns, max_ns) = if let Some(ns_range) = span_tree.ns_range() {
                ns_range
            } else {
                return;
            };

            let info = Info {
                ctx: ui.ctx().clone(),
                canvas: canvas_rect,
                response,
                painter: ui.painter_at(canvas_rect),
                min_ns,
                max_ns,
                text_height: 15.0, // TODO
                font_id: TextStyle::Body.resolve(ui.style()),
            };

            interact_with_canvas(options, &info.response, &info);

            let where_to_put_timeline = info.painter.add(Shape::Noop);

            let max_y = ui_canvas(options, &info, span_tree);

            let mut used_rect = canvas_rect;
            used_rect.max.y = max_y;

            // Fill out space that we don't use so that the `ScrollArea` doesn't collapse in height:
            used_rect.max.y = used_rect.max.y.max(used_rect.min.y + available_height);

            let timeline = paint_timeline(&info, used_rect, options, min_ns);
            info.painter
                .set(where_to_put_timeline, Shape::Vec(timeline));

            ui.allocate_rect(used_rect, Sense::click_and_drag());
        });
    });
}

fn interact_with_canvas(view: &mut FlameGraph, response: &Response, info: &Info) {
    if response.drag_delta().x != 0.0 {
        view.sideways_pan_in_points += response.drag_delta().x;
        view.zoom_to_relative_ns_range = None;
    }

    if response.hovered() {
        // Sideways pan with e.g. a touch pad:
        if info.ctx.input().scroll_delta.x != 0.0 {
            view.sideways_pan_in_points += info.ctx.input().scroll_delta.x;
            view.zoom_to_relative_ns_range = None;
        }

        let mut zoom_factor = info.ctx.input().zoom_delta_2d().x;

        if response.dragged_by(PointerButton::Secondary) {
            zoom_factor *= (response.drag_delta().y * 0.01).exp();
        }

        if zoom_factor != 1.0 {
            view.canvas_width_ns /= zoom_factor;

            if let Some(mouse_pos) = response.hover_pos() {
                let zoom_center = mouse_pos.x - info.canvas.min.x;
                view.sideways_pan_in_points =
                    (view.sideways_pan_in_points - zoom_center) * zoom_factor + zoom_center;
            }
            view.zoom_to_relative_ns_range = None;
        }
    }

    if response.double_clicked() {
        // Reset view
        view.zoom_to_relative_ns_range =
            Some((info.ctx.input().time, (0, info.max_ns - info.min_ns)));
    }

    if let Some((start_time, (min_ns, max_ns))) = view.zoom_to_relative_ns_range {
        const ZOOM_DURATION: f32 = 0.75;
        let t = ((info.ctx.input().time - start_time) as f32 / ZOOM_DURATION).min(1.0);

        let canvas_width = response.rect.width();

        let target_canvas_width_ns = (max_ns - min_ns) as f32;
        let target_pan_in_points = -canvas_width * min_ns as f32 / target_canvas_width_ns;

        view.canvas_width_ns = lerp(
            view.canvas_width_ns.recip()..=target_canvas_width_ns.recip(),
            t,
        )
        .recip();
        view.sideways_pan_in_points = lerp(view.sideways_pan_in_points..=target_pan_in_points, t);

        if t >= 1.0 {
            view.zoom_to_relative_ns_range = None;
        }

        info.ctx.request_repaint();
    }
}

// ----------------------------------------------------------------------------

/// Paints the actual flamegraph
fn ui_canvas(options: &mut FlameGraph, info: &Info, span_tree: &SpanTree) -> f32 {
    if options.canvas_width_ns <= 0.0 {
        // Reset view
        options.canvas_width_ns = (info.max_ns - info.min_ns) as f32;
        options.zoom_to_relative_ns_range = None;
    }

    // We paint the threads top-down
    let mut cursor_y = info.canvas.top();
    cursor_y += info.text_height; // Leave room for time labels

    for span_id in &span_tree.roots {
        if let Some(node) = span_tree.nodes.get(span_id) {
            paint_node_and_children(options, info, span_tree, node, &mut cursor_y);
        }

        cursor_y += 16.0; // spacing between roots
    }

    cursor_y
}

fn paint_node_and_children(
    options: &mut FlameGraph,
    info: &Info,
    span_tree: &SpanTree,
    node: &SpanNode,
    cursor_y: &mut f32,
) -> PaintResult {
    let result = paint_span(options, info, span_tree, node, *cursor_y);
    *cursor_y += options.rect_height + options.spacing;

    let parent_bottom_y = *cursor_y;

    // Some children are "spawned" children (in separate async tasks).
    // There can be only one "direct" child.
    // We paint the direct child close (directly under),
    // and the indirect children with arrows down to them.
    let direct_child = span_tree.direct_child_of(node);

    if let Some(direct_child) = &direct_child {
        if let Some(child) = span_tree.nodes.get(direct_child) {
            paint_node_and_children(options, info, span_tree, child, cursor_y);
        }
    }

    for child in &node.children {
        if Some(*child) != direct_child {
            if let Some(child) = span_tree.nodes.get(child) {
                *cursor_y += 8.0; // spacing between spawned children

                let child_top_y = *cursor_y;
                let result = paint_node_and_children(options, info, span_tree, child, cursor_y);

                if let Some(child_color) = result.color {
                    let x = result.rect.left();
                    let path = [pos2(x, parent_bottom_y), pos2(x, child_top_y)];
                    let stroke = Stroke::new(1.0, child_color * 0.5);
                    info.painter
                        .add(Shape::Vec(Shape::dashed_line(&path, stroke, 5.0, 1.0)));
                    // TODO: paint the line UNDER everything else (needs egui improvement).
                }
            }
        }
    }

    result
}

fn paint_span(
    options: &mut FlameGraph,
    info: &Info,
    span_tree: &SpanTree,
    node: &SpanNode,
    top_y: f32,
) -> PaintResult {
    let (min_ns, max_ns) = estimate_lifetime(span_tree, node);

    let min_x = info.point_from_ns(options, min_ns);
    let max_x = info.point_from_ns(options, max_ns);

    let bottom_y = top_y + options.rect_height;
    let rect = Rect::from_min_max(pos2(min_x, top_y), pos2(max_x, bottom_y));

    if info.canvas.max.x < min_x || max_x < info.canvas.min.x || max_x - min_x < options.cull_width
    {
        return PaintResult { rect, color: None };
    }

    let is_hovered = if let Some(mouse_pos) = info.response.hover_pos() {
        rect.contains(mouse_pos)
    } else {
        false
    };

    if is_hovered && info.response.clicked() {
        options.zoom_to_relative_ns_range = Some((
            info.ctx.input().time,
            (min_ns - info.min_ns, max_ns - info.min_ns),
        ));
    }

    let mut rect_color = if is_hovered {
        HOVER_COLOR
    } else {
        color_from_callsite_id(&node.span.callsite_id)
    };

    let mut min_width = options.min_width;

    if !options.filter.is_empty() {
        let span_description = span_tree.span_description(&node.span.id);
        if options.filter.include(&span_description) {
            // keep full opacity
            min_width *= 2.0; // make it more visible even when thin
        } else {
            rect_color = rect_color.multiply(0.075); // fade to highlight others
        }
    }

    paint_rect(options, info, min_width, rect, rect_color * 0.5);

    for interval in &node.intervals {
        if let (Some(min_t), Some(max_t)) = (interval.min, interval.max) {
            let min_x = info.point_from_ns(options, min_t.nanos_since_epoch());
            let max_x = info.point_from_ns(options, max_t.nanos_since_epoch());
            let y_margin = 1.0;
            let rect = Rect::from_min_max(
                pos2(min_x, top_y + y_margin),
                pos2(max_x, bottom_y - y_margin),
            );
            paint_rect(options, info, options.min_width, rect, rect_color);
        }
    }

    // TODO: paint events

    let wide_enough_for_text = max_x - min_x > 32.0;
    if wide_enough_for_text {
        let painter = info.painter.sub_region(rect.intersect(info.canvas));

        let span_description = span_tree.span_description(&node.span.id);
        let text = span_description;
        let pos = pos2(
            min_x + 4.0,
            top_y + 0.5 * (options.rect_height - info.text_height),
        );
        let pos = painter.round_pos_to_pixels(pos);
        const TEXT_COLOR: Color32 = Color32::BLACK;
        painter.text(
            pos,
            Align2::LEFT_TOP,
            text,
            info.font_id.clone(),
            TEXT_COLOR,
        );
    }

    if is_hovered {
        egui::popup::show_tooltip_for(&info.ctx, Id::new("node-tooltip"), &rect, |ui| {
            span_tree.span_summary_ui(ui, node);
        });
    }

    return PaintResult {
        rect,
        color: Some(rect_color),
    };
}

fn paint_rect(options: &FlameGraph, info: &Info, min_width: f32, rect: Rect, rect_color: Rgba) {
    if rect.width() <= min_width {
        // faster to draw it as a thin line
        info.painter.line_segment(
            [rect.center_top(), rect.center_bottom()],
            egui::Stroke::new(min_width, rect_color),
        );
    } else {
        info.painter.rect_filled(rect, options.rounding, rect_color);
    }
}

fn color_from_callsite_id(callsite_id: &CallsiteId) -> Rgba {
    use rand::rngs::SmallRng;
    use rand::{Rng, SeedableRng};

    let mut small_rng = SmallRng::seed_from_u64(callsite_id.0);

    // TODO: OKLab
    let hsva = egui::color::Hsva {
        h: small_rng.gen(),
        s: small_rng.gen_range(0.35..=0.55_f32).sqrt(),
        v: small_rng.gen_range(0.55..=0.80_f32).cbrt(),
        a: 1.0,
    };

    hsva.into()
}

// TODO: return if the value is known or estimated.
fn estimate_lifetime(span_tree: &SpanTree, node: &SpanNode) -> (NanoSecond, NanoSecond) {
    let mut min = NanoSecond::MAX;
    let mut max = NanoSecond::MIN;

    if let Some(t) = node.lifetime.min {
        min = t.nanos_since_epoch();
    }
    if let Some(t) = node.lifetime.max {
        max = t.nanos_since_epoch();
    }

    if min == NanoSecond::MAX {
        if let Some(interval) = node.intervals.first() {
            if let Some(t) = interval.min {
                min = min.min(t.nanos_since_epoch());
            }
        }
        if let Some((t, _)) = node.events.first() {
            min = min.min(t.nanos_since_epoch());
        }
    }

    if max == NanoSecond::MIN {
        if let Some(interval) = node.intervals.last() {
            if let Some(t) = interval.max {
                max = max.max(t.nanos_since_epoch());
            }
        }
        if let Some((t, _)) = node.events.last() {
            max = max.max(t.nanos_since_epoch());
        }
    }

    if min == NanoSecond::MAX || max == NanoSecond::MIN {
        for child in &node.children {
            if let Some(child) = span_tree.nodes.get(child) {
                let (cmin, cmax) = estimate_lifetime(span_tree, child);
                min = min.min(cmin);
                max = max.max(cmax);
            }
        }
    }

    (min, max)
}

// ----------------------------------------------------------------------------

fn paint_timeline(
    info: &Info,
    canvas: Rect,
    options: &FlameGraph,
    min_ns: NanoSecond,
) -> Vec<egui::Shape> {
    let mut shapes = vec![];

    if options.canvas_width_ns <= 0.0 {
        return shapes;
    }

    let alpha_multiplier = if options.filter.is_empty() { 0.3 } else { 0.1 };

    // We show all measurements relative to min_ns

    let max_lines = canvas.width() / 4.0;
    let mut grid_spacing_ns = 1_000;
    while options.canvas_width_ns / (grid_spacing_ns as f32) > max_lines {
        grid_spacing_ns *= 10;
    }

    // We fade in lines as we zoom in:
    let num_tiny_lines = options.canvas_width_ns / (grid_spacing_ns as f32);
    let zoom_factor = remap_clamp(num_tiny_lines, (0.1 * max_lines)..=max_lines, 1.0..=0.0);
    let zoom_factor = zoom_factor * zoom_factor;
    let big_alpha = remap_clamp(zoom_factor, 0.0..=1.0, 0.5..=1.0);
    let medium_alpha = remap_clamp(zoom_factor, 0.0..=1.0, 0.1..=0.5);
    let tiny_alpha = remap_clamp(zoom_factor, 0.0..=1.0, 0.0..=0.1);

    let mut grid_ns = 0;

    loop {
        let line_x = info.point_from_ns(options, min_ns + grid_ns);
        if line_x > canvas.max.x {
            break;
        }

        if canvas.min.x <= line_x {
            let big_line = grid_ns % (grid_spacing_ns * 100) == 0;
            let medium_line = grid_ns % (grid_spacing_ns * 10) == 0;

            let line_alpha = if big_line {
                big_alpha
            } else if medium_line {
                medium_alpha
            } else {
                tiny_alpha
            };

            shapes.push(egui::Shape::line_segment(
                [pos2(line_x, canvas.min.y), pos2(line_x, canvas.max.y)],
                Stroke::new(1.0, Rgba::from_white_alpha(line_alpha * alpha_multiplier)),
            ));

            let text_alpha = if big_line {
                medium_alpha
            } else if medium_line {
                tiny_alpha
            } else {
                0.0
            };

            if text_alpha > 0.0 {
                let text = grid_text(grid_ns);
                let text_x = line_x + 4.0;
                let text_color = Rgba::from_white_alpha((text_alpha * 2.0).min(1.0)).into();

                // Text at top:
                shapes.push(egui::Shape::text(
                    &info.painter.fonts(),
                    pos2(text_x, canvas.min.y),
                    Align2::LEFT_TOP,
                    &text,
                    info.font_id.clone(),
                    text_color,
                ));

                // Text at bottom:
                shapes.push(egui::Shape::text(
                    &info.painter.fonts(),
                    pos2(text_x, canvas.max.y - info.text_height),
                    Align2::LEFT_TOP,
                    &text,
                    info.font_id.clone(),
                    text_color,
                ));
            }
        }

        grid_ns += grid_spacing_ns;
    }

    shapes
}

fn grid_text(grid_ns: NanoSecond) -> String {
    let grid_ms = to_ms(grid_ns);
    if grid_ns % 1_000_000 == 0 {
        format!("{:.0} ms", grid_ms)
    } else if grid_ns % 100_000 == 0 {
        format!("{:.1} ms", grid_ms)
    } else if grid_ns % 10_000 == 0 {
        format!("{:.2} ms", grid_ms)
    } else {
        format!("{:.3} ms", grid_ms)
    }
}

fn to_ms(ns: NanoSecond) -> f64 {
    ns as f64 * 1e-6
}
