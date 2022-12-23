use eframe::{
    egui::{self, Frame, TextureOptions},
    emath,
    epaint::{pos2, vec2, Color32, ColorImage, ImageDelta, Pos2, Rect, Stroke, TextureId},
};

pub const GAME_SCREEN_WIDTH: usize = 160;
pub const GAME_SCREEN_SCALE: usize = 3;
pub const GAME_SCREEN_HEIGHT: usize = 144;
pub struct GameWindow {
    pub color_palette: [[u8; 3]; 4],
    screen_buffer: [u8; GAME_SCREEN_HEIGHT * GAME_SCREEN_WIDTH],
    texture_id: Option<TextureId>,
    update_texture: bool,
}
impl GameWindow {
    pub fn init_texture(&mut self, ctx: &egui::Context) {
        let tex_manager = ctx.tex_manager();
        let colors = self
            .screen_buffer
            .iter()
            .map(|c| self.color_palette[*c as usize])
            .flatten()
            .collect::<Vec<u8>>();
        let color_image =
            ColorImage::from_rgb([GAME_SCREEN_WIDTH, GAME_SCREEN_HEIGHT], &colors[..]);
        let texture_id = tex_manager.write().alloc(
            "GameWindowTexture".into(),
            color_image.into(),
            egui::TextureOptions::default(),
        );
        self.texture_id = Some(texture_id);
    }
    pub fn update_texture(&mut self, ctx: &egui::Context) {
        let tex_manager = ctx.tex_manager();
        let colors = self
            .screen_buffer
            .iter()
            .map(|c| self.color_palette[*c as usize])
            .flatten()
            .collect::<Vec<u8>>();
        let color_image =
            ColorImage::from_rgb([GAME_SCREEN_WIDTH, GAME_SCREEN_HEIGHT], &colors[..]);
        tex_manager.write().set(
            self.texture_id.unwrap(),
            ImageDelta::full(color_image, TextureOptions::default()),
        );
    }
    pub fn draw_pixel(&mut self, x: usize, y: usize, color: usize) {
        self.screen_buffer[x * GAME_SCREEN_WIDTH + y] = color as u8;
    }
    pub fn view(&mut self, ui: &mut egui::Ui) {
        Frame::canvas(ui.style()).show(ui, |ui| {
            let tex_size = vec2(
                (GAME_SCREEN_WIDTH * GAME_SCREEN_SCALE) as f32,
                (GAME_SCREEN_HEIGHT * GAME_SCREEN_SCALE) as f32,
            );
            if let Some(texture_id) = self.texture_id {
                ui.add(egui::Image::new(texture_id, tex_size));
            }

            let color = if ui.visuals().dark_mode {
                Color32::from_additive_luminance(096)
            } else {
                Color32::from_black_alpha(040)
            };
            ui.ctx().request_repaint();
            let time = ui.input().time;

            let desired_size = ui.available_width() * vec2(1.0, 0.35);
            let (_id, rect) = ui.allocate_space(desired_size);

            let to_screen =
                emath::RectTransform::from_to(Rect::from_x_y_ranges(0.0..=1.0, -1.0..=1.0), rect);

            let mut shapes = vec![];

            for &mode in &[2, 3, 5] {
                let mode = mode as f64;
                let n = 120;
                let speed = 1.5;

                let points: Vec<Pos2> = (0..=n)
                    .map(|i| {
                        let t = i as f64 / (n as f64);
                        let amp = (time * speed * mode).sin() / mode;
                        let y = amp * (t * std::f64::consts::TAU / 2.0 * mode).sin();
                        to_screen * pos2(t as f32, y as f32)
                    })
                    .collect();

                let thickness = 10.0 / mode as f32;
                shapes.push(eframe::epaint::Shape::line(
                    points,
                    Stroke::new(thickness, color),
                ));
            }
            ui.painter().extend(shapes);
        });
    }
}

impl Default for GameWindow {
    fn default() -> Self {
        GameWindow {
            color_palette: [
                Color::blue().into(),
                Color::dark_grey().into(),
                Color::grey().into(),
                Color::light_grey().into(),
            ],
            update_texture: false,
            texture_id: None,
            screen_buffer: [0x0; GAME_SCREEN_HEIGHT * GAME_SCREEN_WIDTH],
        }
    }
}

#[derive(Clone, Debug, Copy)]
struct Color(u8, u8, u8);

impl Color {
    const fn blue() -> Color {
        Color(0x90, 0x90, 0xcc)
    }
    const fn light_grey() -> Color {
        Color(0xcc, 0xcc, 0xcc)
    }
    const fn grey() -> Color {
        Color(0x66, 0x66, 0x66)
    }
    const fn dark_grey() -> Color {
        Color(0x22, 0x22, 0x22)
    }
    const fn white() -> Color {
        Color(0x0, 0x0, 0x0)
    }
}
impl Into<[u8; 3]> for Color {
    fn into(self) -> [u8; 3] {
        [self.0, self.1, self.2]
    }
}

