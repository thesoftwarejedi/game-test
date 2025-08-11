use eframe::egui::{self, vec2, Color32, Rect, Rounding, Stroke, Vec2};
use level_format::{Level, Meta, Rect as LRect, Start};

#[derive(Debug, Default)]
struct Camera {
    // World-space offset of screen origin (0,0) in world coords
    offset: Vec2,
    // Pixels per world unit
    zoom: f32,
}

fn draw_axes(painter: &egui::Painter, rect: Rect, cam: &Camera) {
    // World axes through origin
    let stroke_x = Stroke { width: 2.0, color: Color32::from_rgb(200, 80, 80) };
    let stroke_y = Stroke { width: 2.0, color: Color32::from_rgb(80, 200, 80) };
    // Choose a span based on current view
    let cam_min = panel_to_cam(vec2(0.0, 0.0), rect);
    let cam_max = panel_to_cam(rect.size(), rect);
    let wmin = cam.screen_to_world(cam_min);
    let wmax = cam.screen_to_world(cam_max);
    // Horizontal (X axis, y=0)
    {
        let a_cam = cam.world_to_screen(vec2(wmin.x, 0.0));
        let b_cam = cam.world_to_screen(vec2(wmax.x, 0.0));
        let a = rect.min + cam_to_panel(a_cam, rect);
        let b = rect.min + cam_to_panel(b_cam, rect);
        painter.line_segment([a, b], stroke_x);
    }
    // Vertical (Y axis, x=0)
    {
        let a_cam = cam.world_to_screen(vec2(0.0, wmin.y));
        let b_cam = cam.world_to_screen(vec2(0.0, wmax.y));
        let a = rect.min + cam_to_panel(a_cam, rect);
        let b = rect.min + cam_to_panel(b_cam, rect);
        painter.line_segment([a, b], stroke_y);
    }
}

// Convert between our camera screen (origin bottom-left of viewport with Y up)
// and egui panel coordinates (origin top-left with Y down).
fn cam_to_panel(screen_cam: Vec2, viewport: Rect) -> Vec2 {
    vec2(screen_cam.x, viewport.height() - screen_cam.y)
}
fn panel_to_cam(screen_panel: Vec2, viewport: Rect) -> Vec2 {
    vec2(screen_panel.x, viewport.height() - screen_panel.y)
}

fn draw_rect_center_stroked(painter: &egui::Painter, rect: Rect, cam: &Camera, r: LRect, color: Color32, width: f32) {
    let half = vec2(r.w * 0.5, r.h * 0.5);
    let min = vec2(r.x, r.y) - half;
    let max = vec2(r.x, r.y) + half;
    let a_cam = cam.world_to_screen(min);
    let b_cam = cam.world_to_screen(max);
    let a = cam_to_panel(a_cam, rect);
    let b = cam_to_panel(b_cam, rect);
    // Use from_two_pos to handle inverted Y after panel conversion
    let rr = Rect::from_two_pos(rect.min + a, rect.min + b);
    painter.rect(rr, Rounding::ZERO, Color32::TRANSPARENT, Stroke { width, color });
}

#[derive(Debug, Clone, Copy)]
enum Handle { NW, NE, SW, SE }

#[derive(Debug, Clone)]
struct ResizeState {
    kind: ItemKind,
    idx: usize,
    anchor_world: Vec2, // fixed opposite corner
}

impl ResizeState {
    fn start(level: &Level, kind: ItemKind, idx: usize, handle: Handle) -> Self {
        let (cx, cy, w, h) = match kind {
            ItemKind::Platform => {
                let r = &level.platforms[idx]; (r.x, r.y, r.w, r.h)
            }
            ItemKind::Exit => {
                let e = &level.exits[idx]; (e.x, e.y, e.w, e.h)
            }
        };
        let half = vec2(w * 0.5, h * 0.5);
        let min = vec2(cx, cy) - half;
        let max = vec2(cx, cy) + half;
        // Opposite corner is anchor
        let anchor_world = match handle {
            Handle::NW => vec2(max.x, max.y),
            Handle::NE => vec2(min.x, max.y),
            Handle::SW => vec2(max.x, min.y),
            Handle::SE => vec2(min.x, min.y),
        };
        Self { kind, idx, anchor_world }
    }

    fn apply(&mut self, level: &mut Level, drag_world: Vec2) {
        // New rect from anchor to drag point
        let min = vec2(self.anchor_world.x.min(drag_world.x), self.anchor_world.y.min(drag_world.y));
        let max = vec2(self.anchor_world.x.max(drag_world.x), self.anchor_world.y.max(drag_world.y));
        let center = (min + max) * 0.5;
        let w = (max.x - min.x).max(1.0);
        let h = (max.y - min.y).max(1.0);
        match self.kind {
            ItemKind::Platform => {
                if let Some(r) = level.platforms.get_mut(self.idx) { r.x = center.x; r.y = center.y; r.w = w; r.h = h; }
            }
            ItemKind::Exit => {
                if let Some(e) = level.exits.get_mut(self.idx) { e.x = center.x; e.y = center.y; e.w = w; e.h = h; }
            }
        }
    }
}

fn draw_handles(painter: &egui::Painter, rect: Rect, cam: &Camera, r: &LRect) {
    let half = vec2(r.w * 0.5, r.h * 0.5);
    let min = vec2(r.x, r.y) - half;
    let max = vec2(r.x, r.y) + half;
    let corners = [min, vec2(max.x, min.y), vec2(min.x, max.y), max];
    for c in corners {
        let s_cam = cam.world_to_screen(c);
        let p = rect.min + cam_to_panel(s_cam, rect);
        let hs = 4.0; // half-size px
        let rr = Rect::from_min_max(p - vec2(hs, hs), p + vec2(hs, hs));
        painter.rect_filled(rr, Rounding::ZERO, Color32::from_rgb(220, 220, 220));
        painter.rect_stroke(rr, Rounding::ZERO, Stroke { width: 1.0, color: Color32::BLACK });
    }
}

fn hit_test_handles(level: &Level, cam: &Camera, viewport: Rect, world: Vec2) -> Option<(ItemKind, usize, Handle)> {
    // check exits then platforms for selection consistency
    // Use screen-space hit test for small squares
    let handle_hit = |center_world: Vec2, viewport: Rect, cam: &Camera, world: Vec2| -> bool {
        let s_cam = cam.world_to_screen(center_world);
        let p = viewport.min + cam_to_panel(s_cam, viewport);
        let hs = 6.0; // Hit box radius px
        let rect = Rect::from_min_max(p - vec2(hs, hs), p + vec2(hs, hs));
        // Convert world point to screen
        let mouse_cam = cam.world_to_screen(world);
        let mouse_s = viewport.min + cam_to_panel(mouse_cam, viewport);
        rect.contains(mouse_s)
    };

    // Helper to iterate corners
    let check_rect = |x: f32, y: f32, w: f32, h: f32| -> Option<Handle> {
        let half = vec2(w * 0.5, h * 0.5);
        let min = vec2(x, y) - half;
        let max = vec2(x, y) + half;
        let nw = min;
        let ne = vec2(max.x, min.y);
        let sw = vec2(min.x, max.y);
        let se = max;
        if handle_hit(nw, viewport, cam, world) { return Some(Handle::NW); }
        if handle_hit(ne, viewport, cam, world) { return Some(Handle::NE); }
        if handle_hit(sw, viewport, cam, world) { return Some(Handle::SW); }
        if handle_hit(se, viewport, cam, world) { return Some(Handle::SE); }
        None
    };

    for (i, e) in level.exits.iter().enumerate().rev() {
        if let Some(h) = check_rect(e.x, e.y, e.w, e.h) { return Some((ItemKind::Exit, i, h)); }
    }
    for (i, r) in level.platforms.iter().enumerate().rev() {
        if let Some(h) = check_rect(r.x, r.y, r.w, r.h) { return Some((ItemKind::Platform, i, h)); }
    }
    None
}

impl Camera {
    fn world_to_screen(&self, world: Vec2) -> Vec2 {
        // Camera space with +Y up
        (world - self.offset) * self.zoom
    }
    fn screen_to_world(&self, screen: Vec2) -> Vec2 {
        // From camera space (+Y up) back to world
        screen / self.zoom + self.offset
    }
}

struct EditorApp {
    level: Option<Level>,
    camera: Camera,
    snap_enabled: bool,
    snap_size: f32,
    status: String,
    // file path for Save
    current_path: Option<std::path::PathBuf>,
    // tools and selection
    tool: Tool,
    selection: Selection,
    // temp state for drawing new rects
    drag_start_world: Option<Vec2>,
    // resizing state
    resizing: Option<ResizeState>,
    // need to frame content after load/new when viewport known
    needs_frame: bool,
}

impl Default for EditorApp {
    fn default() -> Self {
        Self {
            level: None,
            camera: Camera { offset: vec2(0.0, 0.0), zoom: 1.0 },
            snap_enabled: true,
            snap_size: 10.0,
            status: String::new(),
            current_path: None,
            tool: Tool::Select,
            selection: Selection::None,
            drag_start_world: None,
            resizing: None,
            needs_frame: false,
        }
    }
}

impl eframe::App for EditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top menu bar
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("New").clicked() {
                    self.level = Some(Level {
                        meta: Meta { name: "untitled".to_string() },
                        start: Start { x: 0.0, y: 0.0 },
                        platforms: vec![],
                        exits: vec![],
                    });
                    self.status = "Created new level".into();
                    self.current_path = None;
                    self.selection = Selection::None;
                    self.needs_frame = true;
                }
                if ui.button("Open").clicked() {
                    if let Some(path) = rfd::FileDialog::new().add_filter("TOML", &["toml"]).pick_file() {
                        match std::fs::read_to_string(&path) {
                            Ok(contents) => match Level::from_toml_str(&contents) {
                                Ok(level) => {
                                    self.level = Some(level);
                                    self.status = format!("Opened {}", path.display());
                                    self.current_path = Some(path.clone());
                                    self.selection = Selection::None;
                                    self.needs_frame = true;
                                }
                                Err(e) => self.status = format!("Failed to parse: {e}"),
                            },
                            Err(e) => self.status = format!("Failed to read: {e}"),
                        }
                    }
                }
                if ui.button("Frame").on_hover_text("Fit content to view").clicked() {
                    self.needs_frame = true;
                }
                if ui.button("Save").clicked() {
                    if let Some(level) = &self.level {
                        // Enforce start exists (always present in model). Additional guard in case of None level.
                        let toml = match level.to_toml_string_pretty() {
                            Ok(s) => s,
                            Err(e) => {
                                self.status = format!("Serialize error: {e}");
                                String::new()
                            }
                        };
                        if !toml.is_empty() {
                            if let Some(path) = &self.current_path {
                                if let Err(e) = std::fs::write(path, toml) {
                                    self.status = format!("Failed to save: {e}");
                                } else {
                                    self.status = format!("Saved {}", path.display());
                                }
                            } else {
                                if let Some(path) = rfd::FileDialog::new().add_filter("TOML", &["toml"]).save_file() {
                                    if let Err(e) = std::fs::write(&path, toml) {
                                        self.status = format!("Failed to save: {e}");
                                    } else {
                                        self.current_path = Some(path.clone());
                                        self.status = format!("Saved {}", path.display());
                                    }
                                }
                            }
                        }
                    } else {
                        self.status = "No level to save".into();
                    }
                }
                if ui.button("Save As").clicked() {
                    if let Some(level) = &self.level {
                        if let Some(path) = rfd::FileDialog::new().add_filter("TOML", &["toml"]).save_file() {
                            // Enforce single start exists: Level always has one, so just proceed
                            match level.to_toml_string_pretty() {
                                Ok(s) => {
                                    if let Err(e) = std::fs::write(&path, s) {
                                        self.status = format!("Failed to save: {e}");
                                    } else {
                                        self.current_path = Some(path.clone());
                                        self.status = format!("Saved {}", path.display());
                                    }
                                }
                                Err(e) => self.status = format!("Serialize error: {e}"),
                            }
                        }
                    } else {
                        self.status = "No level to save".into();
                    }
                }
                ui.separator();
                ui.toggle_value(&mut self.snap_enabled, "Snap (10px)");
                ui.label("Hold Ctrl to temporarily disable snap");
                ui.separator();
                ui.label("Tool:");
                ui.selectable_value(&mut self.tool, Tool::Select, "Select");
                ui.selectable_value(&mut self.tool, Tool::Platform, "Platform");
                ui.selectable_value(&mut self.tool, Tool::Exit, "Exit");
                ui.selectable_value(&mut self.tool, Tool::Start, "Start");
            });
        });

        // Central canvas
        egui::CentralPanel::default().show(ctx, |ui| {
            let available = ui.available_rect_before_wrap();
            let painter = ui.painter_at(available);

            // Capture both click and drag so selection clicks register
            let response = ui.allocate_rect(available, egui::Sense::click_and_drag());

            // Auto-frame content after loading/new once we know viewport size
            if self.needs_frame {
                if let Some(level) = &self.level {
                    // Compute bounds across platforms and exits
                    let mut min_w = vec2(f32::INFINITY, f32::INFINITY);
                    let mut max_w = vec2(f32::NEG_INFINITY, f32::NEG_INFINITY);
                    for r in &level.platforms {
                        let half = vec2(r.w * 0.5, r.h * 0.5);
                        let rmin = vec2(r.x, r.y) - half;
                        let rmax = vec2(r.x, r.y) + half;
                        min_w.x = min_w.x.min(rmin.x); min_w.y = min_w.y.min(rmin.y);
                        max_w.x = max_w.x.max(rmax.x); max_w.y = max_w.y.max(rmax.y);
                    }
                    for e in &level.exits {
                        let half = vec2(e.w * 0.5, e.h * 0.5);
                        let rmin = vec2(e.x, e.y) - half;
                        let rmax = vec2(e.x, e.y) + half;
                        min_w.x = min_w.x.min(rmin.x); min_w.y = min_w.y.min(rmin.y);
                        max_w.x = max_w.x.max(rmax.x); max_w.y = max_w.y.max(rmax.y);
                    }
                    let have_any = min_w.x.is_finite();
                    let viewport_size = available.size();
                    if have_any {
                        let content_size = (max_w - min_w).max(vec2(1.0, 1.0));
                        let pad = 20.0;
                        let zx = (viewport_size.x - pad * 2.0) / content_size.x;
                        let zy = (viewport_size.y - pad * 2.0) / content_size.y;
                        self.camera.zoom = zx.min(zy).clamp(0.05, 10.0);
                        let center_w = (min_w + max_w) * 0.5;
                        let viewport_center_cam = panel_to_cam(viewport_size * 0.5, available);
                        self.camera.offset = center_w - viewport_center_cam / self.camera.zoom;
                    } else {
                        // No platforms/exits; center on start
                        let center_w = vec2(level.start.x, level.start.y);
                        self.camera.zoom = 1.0;
                        let viewport_center_cam = panel_to_cam(viewport_size * 0.5, available);
                        self.camera.offset = center_w - viewport_center_cam / self.camera.zoom;
                    }
                }
                self.needs_frame = false;
            }

            // Input: pan (space or middle mouse)
            let input = ui.input(|i| i.clone());
            let panning_now = input.modifiers.alt // alternative if no middle/space
                || input.pointer.middle_down()
                || input.key_down(egui::Key::Space);
            if panning_now && response.dragged() {
                let delta_panel = response.drag_delta();
                // Convert panel delta (Y down) to camera delta (Y up)
                let delta_cam = vec2(delta_panel.x, -delta_panel.y);
                // Pan so content follows the cursor (hand tool)
                self.camera.offset -= delta_cam / self.camera.zoom;
            }

            // Input: zoom with scroll, anchor at mouse position
            let scroll_delta = input.raw_scroll_delta.y; // y-axis scroll
            if scroll_delta != 0.0 {
                let mouse_pos = input.pointer.hover_pos().unwrap_or(available.center());
                let mouse_panel = mouse_pos - available.min; // panel coords
                let mouse_cam = panel_to_cam(mouse_panel, available);
                let pre_world = self.camera.screen_to_world(mouse_cam);
                let zoom_factor = (1.0 + (scroll_delta * 0.001)).clamp(0.1, 10.0);
                self.camera.zoom = (self.camera.zoom * zoom_factor).clamp(0.05, 10.0);
                let post_cam = self.camera.world_to_screen(pre_world);
                let post_panel = cam_to_panel(post_cam, available);
                let screen_delta_panel = mouse_panel - post_panel;
                // Convert to camera-space delta
                let screen_delta_cam = vec2(screen_delta_panel.x, -screen_delta_panel.y);
                // Adjust offset so the same world point stays under the cursor
                self.camera.offset += -screen_delta_cam / self.camera.zoom;
            }

            // Draw background
            painter.rect_filled(available, 0.0, Color32::from_gray(22));

            // Draw grid
            draw_grid(&painter, available, &self.camera, self.snap_size);

            // Draw world axes lines through origin for orientation
            draw_axes(&painter, available, &self.camera);

            // Draw level geometry
            if let Some(level) = &self.level {
                // Platforms
                for (i, r) in level.platforms.iter().enumerate() {
                    let color = Color32::from_rgb(80, 160, 255);
                    let selected = matches!(self.selection, Selection::Item(ItemKind::Platform, si) if si == i);
                    let stroke_w = if selected { 3.0 } else { 2.0 };
                    draw_rect_center_stroked(&painter, available, &self.camera, *r, color, stroke_w);
                    if selected { draw_handles(&painter, available, &self.camera, r); }
                }
                // Exits (orange)
                for (i, e) in level.exits.iter().enumerate() {
                    let r = LRect { x: e.x, y: e.y, w: e.w, h: e.h };
                    let selected = matches!(self.selection, Selection::Item(ItemKind::Exit, si) if si == i);
                    let stroke_w = if selected { 3.0 } else { 2.0 };
                    draw_rect_center_stroked(&painter, available, &self.camera, r, Color32::from_rgb(255, 160, 40), stroke_w);
                    if selected { draw_handles(&painter, available, &self.camera, &r); }
                }
                // Start marker (green cross)
                let start = vec2(level.start.x, level.start.y);
                draw_cross(&painter, available, &self.camera, start, 10.0, Color32::from_rgb(80, 220, 120));
            }

            // Handle interactions per tool
            if let Some(level) = self.level.as_mut() {
                let ctrl_down = ui.input(|i| i.modifiers.ctrl);
                let snap_now = self.snap_enabled && !ctrl_down;

                // Delete key removes selected item
                if ui.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)) {
                    if let Selection::Item(kind, idx) = self.selection {
                        match kind {
                            ItemKind::Platform => { if idx < level.platforms.len() { level.platforms.remove(idx); } }
                            ItemKind::Exit => { if idx < level.exits.len() { level.exits.remove(idx); } }
                        }
                        self.selection = Selection::None;
                    }
                }

                // Duplicate selected item (Cmd/Ctrl + D)
                if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::D)) {
                    if let Selection::Item(kind, idx) = self.selection {
                        match kind {
                            ItemKind::Platform => {
                                if let Some(r) = level.platforms.get(idx).cloned() {
                                    level.platforms.push(LRect { x: r.x + 10.0, y: r.y - 10.0, ..r });
                                }
                            }
                            ItemKind::Exit => {
                                if let Some(e) = level.exits.get(idx).cloned() {
                                    level.exits.push(level_format::Exit { x: e.x + 10.0, y: e.y - 10.0, ..e });
                                }
                            }
                        }
                    }
                }

                match self.tool {
                    Tool::Select => {
                        // Simple: click on rect to select, drag to move, handles to resize
                        if response.clicked_by(egui::PointerButton::Primary) {
                            let mouse = ui.input(|i| i.pointer.interact_pos()).unwrap_or(available.center());
                            let cam_pt = panel_to_cam(mouse - available.min, available);
                            let world = self.camera.screen_to_world(cam_pt);
                            // Test resize handles first
                            if let Some((kind, idx, handle)) = hit_test_handles(level, &self.camera, available, world) {
                                self.selection = Selection::Item(kind, idx);
                                self.resizing = Some(ResizeState::start(level, kind, idx, handle));
                            } else {
                                // hit test exits then platforms
                                if let Some((idx, kind)) = hit_test_level(level, &self.camera, available, world) {
                                    self.selection = Selection::Item(kind, idx);
                                } else {
                                    self.selection = Selection::None;
                                }
                            }
                        }

                        // Drag to move selected item
                        if let Selection::Item(kind, idx) = self.selection {
                            if response.dragged() && !is_panning(&ui, &response) && self.resizing.is_none() {
                                let delta_panel = response.drag_delta();
                                // Convert panel-space delta (Y down) to camera/world delta (Y up)
                                let mut delta_world = vec2(delta_panel.x, -delta_panel.y) / self.camera.zoom;
                                // Constrain axis with Shift: zero out smaller magnitude
                                if ui.input(|i| i.modifiers.shift) {
                                    if delta_world.x.abs() > delta_world.y.abs() { delta_world.y = 0.0; } else { delta_world.x = 0.0; }
                                }
                                if snap_now {
                                    delta_world.x = snap_value(delta_world.x, self.snap_size);
                                    delta_world.y = snap_value(delta_world.y, self.snap_size);
                                }
                                match kind {
                                    ItemKind::Platform => {
                                        if let Some(r) = level.platforms.get_mut(idx) {
                                            r.x += delta_world.x;
                                            r.y += delta_world.y;
                                        }
                                    }
                                    ItemKind::Exit => {
                                        if let Some(e) = level.exits.get_mut(idx) {
                                            e.x += delta_world.x;
                                            e.y += delta_world.y;
                                        }
                                    }
                                }
                            }
                        }

                        // Resizing if active
                        if let Some(state) = &mut self.resizing {
                            if response.dragged() {
                                let mouse = ui.input(|i| i.pointer.hover_pos()).unwrap_or(available.center());
                                let cam_pt = panel_to_cam(mouse - available.min, available);
                                let mut world = self.camera.screen_to_world(cam_pt);
                                if snap_now { world = snap_vec2(world, self.snap_size); }
                                state.apply(level, world);
                            }
                            if response.drag_stopped() {
                                self.resizing = None;
                            }
                        }
                    }
                    Tool::Platform | Tool::Exit => {
                        if response.drag_started() {
                            let mouse = ui.input(|i| i.pointer.interact_pos()).unwrap_or(available.center());
                            let cam_pt = panel_to_cam(mouse - available.min, available);
                            let world = self.camera.screen_to_world(cam_pt);
                            self.drag_start_world = Some(world);
                        }
                        if let Some(start) = self.drag_start_world {
                            let mouse = ui.input(|i| i.pointer.hover_pos()).unwrap_or(available.center());
                            let cam_pt = panel_to_cam(mouse - available.min, available);
                            let world = self.camera.screen_to_world(cam_pt);
                            // draw a preview rect
                            let center = (start + world) * 0.5;
                            let mut w = (world.x - start.x).abs();
                            let mut h = (world.y - start.y).abs();
                            if snap_now {
                                w = snap_positive(w, self.snap_size);
                                h = snap_positive(h, self.snap_size);
                            }
                            let preview = LRect { x: center.x, y: center.y, w: w.max(1.0), h: h.max(1.0) };
                            let color = if matches!(self.tool, Tool::Platform) { Color32::from_rgb(80,160,255) } else { Color32::from_rgb(255,160,40) };
                            draw_rect_center(&painter, available, &self.camera, preview, color);
                        }
                        if response.drag_stopped() {
                            if let Some(start) = self.drag_start_world.take() {
                                let mouse = ui.input(|i| i.pointer.hover_pos()).unwrap_or(available.center());
                                let cam_pt = panel_to_cam(mouse - available.min, available);
                                let world = self.camera.screen_to_world(cam_pt);
                                let center = (start + world) * 0.5;
                                let mut w = (world.x - start.x).abs();
                                let mut h = (world.y - start.y).abs();
                                if snap_now {
                                    w = snap_positive(w, self.snap_size);
                                    h = snap_positive(h, self.snap_size);
                                }
                                let rect = LRect { x: center.x, y: center.y, w: w.max(1.0), h: h.max(1.0) };
                                match self.tool {
                                    Tool::Platform => level.platforms.push(rect),
                                    Tool::Exit => level.exits.push(level_format::Exit { x: rect.x, y: rect.y, w: rect.w, h: rect.h, next: String::from("level2") }),
                                    _ => {}
                                }
                            }
                        }
                    }
                    Tool::Start => {
                        // Drag start point
                        if response.dragged() {
                            let mouse = ui.input(|i| i.pointer.hover_pos()).unwrap_or(available.center());
                            let cam_pt = panel_to_cam(mouse - available.min, available);
                            let mut world = self.camera.screen_to_world(cam_pt);
                            if snap_now { world = snap_vec2(world, self.snap_size); }
                            level.start.x = world.x;
                            level.start.y = world.y;
                        }
                    }
                }
            }

            // Status in corner
            let snap_temp_disabled = ui.input(|i| i.modifiers.ctrl) && self.snap_enabled;
            let info = format!(
                "zoom: {:.2}  offset: ({:.1},{:.1})  snap:{}{}  {}",
                self.camera.zoom,
                self.camera.offset.x,
                self.camera.offset.y,
                self.snap_size,
                if snap_temp_disabled { " (Ctrl held)" } else { "" },
                self.status
            );
            painter.text(
                available.min + vec2(8.0, 8.0),
                egui::Align2::LEFT_TOP,
                info,
                egui::FontId::monospace(12.0),
                Color32::LIGHT_GRAY,
            );
        });
        // Side panel for properties of selected Exit
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            if let (Some(level), Selection::Item(ItemKind::Exit, idx)) = (self.level.as_mut(), self.selection) {
                if let Some(exit) = level.exits.get_mut(idx) {
                    ui.horizontal(|ui| {
                        ui.label("Exit next:");
                        ui.text_edit_singleline(&mut exit.next);
                    });
                }
            }
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tool { Select, Platform, Exit, Start }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ItemKind { Platform, Exit }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Selection { None, Item(ItemKind, usize) }

fn is_panning(ui: &egui::Ui, _response: &egui::Response) -> bool {
    ui.input(|i| i.modifiers.alt) || ui.input(|i| i.pointer.middle_down()) || ui.input(|i| i.key_down(egui::Key::Space))
}

fn hit_test_level(level: &Level, _cam: &Camera, _viewport: Rect, world: Vec2) -> Option<(usize, ItemKind)> {
    // Prioritize exits (top), then platforms
    for (i, e) in level.exits.iter().enumerate().rev() {
        if point_in_center_rect(world, e.x, e.y, e.w, e.h) { return Some((i, ItemKind::Exit)); }
    }
    for (i, r) in level.platforms.iter().enumerate().rev() {
        if point_in_center_rect(world, r.x, r.y, r.w, r.h) { return Some((i, ItemKind::Platform)); }
    }
    None
}

fn point_in_center_rect(p: Vec2, x: f32, y: f32, w: f32, h: f32) -> bool {
    let half = vec2(w * 0.5, h * 0.5);
    p.x >= x - half.x && p.x <= x + half.x && p.y >= y - half.y && p.y <= y + half.y
}

fn snap_value(v: f32, grid: f32) -> f32 { (v / grid).round() * grid }
fn snap_positive(v: f32, grid: f32) -> f32 { (v / grid).round().abs() * grid }
fn snap_vec2(p: Vec2, grid: f32) -> Vec2 { vec2(snap_value(p.x, grid), snap_value(p.y, grid)) }

fn draw_grid(painter: &egui::Painter, rect: Rect, cam: &Camera, grid: f32) {
    let stroke = Stroke { width: 1.0, color: Color32::from_gray(40) };
    // Determine world bounds visible
    let cam_min = panel_to_cam(vec2(0.0, 0.0), rect);
    let cam_max = panel_to_cam(rect.size(), rect);
    let top_left_world = cam.screen_to_world(cam_min);
    let bottom_right_world = cam.screen_to_world(cam_max);
    let min_x = top_left_world.x.min(bottom_right_world.x).floor();
    let max_x = top_left_world.x.max(bottom_right_world.x).ceil();
    let min_y = top_left_world.y.min(bottom_right_world.y).floor();
    let max_y = top_left_world.y.max(bottom_right_world.y).ceil();

    // Start at nearest grid lines
    let start_x = (min_x / grid).floor() * grid;
    let start_y = (min_y / grid).floor() * grid;

    let mut x = start_x;
    while x <= max_x {
        let a_cam = cam.world_to_screen(vec2(x, min_y));
        let b_cam = cam.world_to_screen(vec2(x, max_y));
        let a = cam_to_panel(a_cam, rect);
        let b = cam_to_panel(b_cam, rect);
        painter.line_segment(
            [rect.min + a, rect.min + b],
            stroke,
        );
        x += grid;
    }
    let mut y = start_y;
    while y <= max_y {
        let a_cam = cam.world_to_screen(vec2(min_x, y));
        let b_cam = cam.world_to_screen(vec2(max_x, y));
        let a = cam_to_panel(a_cam, rect);
        let b = cam_to_panel(b_cam, rect);
        painter.line_segment(
            [rect.min + a, rect.min + b],
            stroke,
        );
        y += grid;
    }
}

fn draw_rect_center(painter: &egui::Painter, rect: Rect, cam: &Camera, r: LRect, color: Color32) {
    let half = vec2(r.w * 0.5, r.h * 0.5);
    let min = vec2(r.x, r.y) - half;
    let max = vec2(r.x, r.y) + half;
    let a_cam = cam.world_to_screen(min);
    let b_cam = cam.world_to_screen(max);
    let a = cam_to_panel(a_cam, rect);
    let b = cam_to_panel(b_cam, rect);
    let rr = Rect::from_two_pos(rect.min + a, rect.min + b);
    painter.rect(rr, Rounding::ZERO, Color32::TRANSPARENT, Stroke { width: 2.0, color });
}

fn draw_cross(painter: &egui::Painter, rect: Rect, cam: &Camera, world: Vec2, size: f32, color: Color32) {
    let s_cam = cam.world_to_screen(world);
    let p = rect.min + cam_to_panel(s_cam, rect);
    painter.line_segment([p + vec2(-size, 0.0), p + vec2(size, 0.0)], Stroke { width: 2.0, color });
    painter.line_segment([p + vec2(0.0, -size), p + vec2(0.0, size)], Stroke { width: 2.0, color });
}

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Level Editor",
        native_options,
        Box::new(|_cc| Ok(Box::new(EditorApp::default()))),
    )
}
