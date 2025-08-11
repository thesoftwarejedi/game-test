use eframe::egui::{self, vec2, Color32, Rect, Rounding, Stroke, Vec2};
use level_format::{Level, Meta, Rect as LRect, Start};

#[derive(Debug, Default)]
struct Camera {
    // World-space offset of screen origin (0,0) in world coords
    offset: Vec2,
    // Pixels per world unit
    zoom: f32,
}

impl Camera {
    fn world_to_screen(&self, world: Vec2) -> Vec2 {
        (world - self.offset) * self.zoom
    }
    fn screen_to_world(&self, screen: Vec2) -> Vec2 {
        screen / self.zoom + self.offset
    }
}

struct EditorApp {
    level: Option<Level>,
    camera: Camera,
    snap_enabled: bool,
    snap_size: f32,
    status: String,
}

impl Default for EditorApp {
    fn default() -> Self {
        Self {
            level: None,
            camera: Camera { offset: vec2(0.0, 0.0), zoom: 1.0 },
            snap_enabled: true,
            snap_size: 10.0,
            status: String::new(),
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
                }
                if ui.button("Open").clicked() {
                    if let Some(path) = rfd::FileDialog::new().add_filter("TOML", &["toml"]).pick_file() {
                        match std::fs::read_to_string(&path) {
                            Ok(contents) => match Level::from_toml_str(&contents) {
                                Ok(level) => {
                                    self.level = Some(level);
                                    self.status = format!("Opened {}", path.display());
                                }
                                Err(e) => self.status = format!("Failed to parse: {e}"),
                            },
                            Err(e) => self.status = format!("Failed to read: {e}"),
                        }
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
            });
        });

        // Central canvas
        egui::CentralPanel::default().show(ctx, |ui| {
            let available = ui.available_rect_before_wrap();
            let painter = ui.painter_at(available);

            let response = ui.allocate_rect(available, egui::Sense::drag());

            // Input: pan (space or middle mouse)
            let input = ui.input(|i| i.clone());
            let is_panning = input.modifiers.alt // alternative if no middle/space
                || input.pointer.middle_down()
                || input.key_down(egui::Key::Space);
            if is_panning && response.dragged() {
                let delta = response.drag_delta();
                // Move camera offset opposite to screen movement
                self.camera.offset -= delta / self.camera.zoom;
            }

            // Input: zoom with scroll, anchor at mouse position
            let scroll_delta = input.raw_scroll_delta.y; // y-axis scroll
            if scroll_delta != 0.0 {
                let mouse_pos = input.pointer.hover_pos().unwrap_or(available.center());
                let mouse_screen = mouse_pos - available.min; // Vec2
                let pre_world = self.camera.screen_to_world(mouse_screen);
                let zoom_factor = (1.0 + (scroll_delta * 0.001)).clamp(0.1, 10.0);
                self.camera.zoom = (self.camera.zoom * zoom_factor).clamp(0.05, 10.0);
                let post_screen = self.camera.world_to_screen(pre_world);
                let screen_delta = mouse_screen - post_screen;
                self.camera.offset -= screen_delta / self.camera.zoom;
            }

            // Draw background
            painter.rect_filled(available, 0.0, Color32::from_gray(22));

            // Draw grid
            draw_grid(&painter, available, &self.camera, self.snap_size);

            // Draw level geometry
            if let Some(level) = &self.level {
                // Platforms
                for r in &level.platforms {
                    draw_rect_center(&painter, available, &self.camera, *r, Color32::from_rgb(80, 160, 255));
                }
                // Exits (orange)
                for e in &level.exits {
                    let r = LRect { x: e.x, y: e.y, w: e.w, h: e.h };
                    draw_rect_center(&painter, available, &self.camera, r, Color32::from_rgb(255, 160, 40));
                }
                // Start marker (green cross)
                let start = vec2(level.start.x, level.start.y);
                draw_cross(&painter, available, &self.camera, start, 10.0, Color32::from_rgb(80, 220, 120));
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
    }
}

fn draw_grid(painter: &egui::Painter, rect: Rect, cam: &Camera, grid: f32) {
    let stroke = Stroke { width: 1.0, color: Color32::from_gray(40) };
    // Determine world bounds visible
    let top_left_world = cam.screen_to_world((rect.min).to_vec2());
    let bottom_right_world = cam.screen_to_world((rect.max).to_vec2());
    let min_x = top_left_world.x.min(bottom_right_world.x).floor();
    let max_x = top_left_world.x.max(bottom_right_world.x).ceil();
    let min_y = top_left_world.y.min(bottom_right_world.y).floor();
    let max_y = top_left_world.y.max(bottom_right_world.y).ceil();

    // Start at nearest grid lines
    let start_x = (min_x / grid).floor() * grid;
    let start_y = (min_y / grid).floor() * grid;

    let mut x = start_x;
    while x <= max_x {
        let a = cam.world_to_screen(vec2(x, min_y));
        let b = cam.world_to_screen(vec2(x, max_y));
        painter.line_segment(
            [rect.min + a, rect.min + b],
            stroke,
        );
        x += grid;
    }
    let mut y = start_y;
    while y <= max_y {
        let a = cam.world_to_screen(vec2(min_x, y));
        let b = cam.world_to_screen(vec2(max_x, y));
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
    let a = cam.world_to_screen(min);
    let b = cam.world_to_screen(max);
    let rr = Rect::from_min_max(rect.min + a, rect.min + b);
    painter.rect(rr, Rounding::ZERO, Color32::TRANSPARENT, Stroke { width: 2.0, color });
}

fn draw_cross(painter: &egui::Painter, rect: Rect, cam: &Camera, world: Vec2, size: f32, color: Color32) {
    let s = cam.world_to_screen(world);
    let p = rect.min + s;
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
