use std::{sync::mpsc::Receiver};

use self::game_window::{GameWindow, GAME_SCREEN_HEIGHT, GAME_SCREEN_SCALE, GAME_SCREEN_WIDTH};
use eframe::{egui, epaint::vec2};
mod game_window;

const _BUFFER_SIZE: usize = 0;
const WINDOW_HEIGHT: f32 = 400.;
const WINDOW_WIDTH: f32 = 700.;
pub struct Gpu {
    signal_receiver: Receiver<DrawSignal>,
    window: Window,
}
impl Gpu {
    pub fn new(receiver: Receiver<DrawSignal>) -> Self {
        Gpu {
            signal_receiver: receiver,
            window: Window::default(),
        }
    }
    pub fn init_window(mut self, cc: &eframe::CreationContext) -> Self {
        self.window.init(&cc.egui_ctx);
        self
    }

    pub fn run(self) {
        let options = eframe::NativeOptions {
            initial_window_size: Some(egui::vec2(WINDOW_WIDTH, WINDOW_HEIGHT)),
            ..Default::default()
        };
        eframe::run_native(
            "Gameboy Emulator",
            options,
            Box::new(|cc| Box::new(self.init_window(cc))),
        )
    }
    
}

struct Window {
    game_window: GameWindow,
}
impl Window {
    pub fn init(&mut self, ctx: &egui::Context) {
        self.game_window.init_texture(ctx);
    }
    pub fn view(&mut self, ui: &mut egui::Ui) {
        self.game_window.view(ui)
    }
    pub fn process_draw_signal(&mut self, draw_signal: DrawSignal) {
        match draw_signal {
            DrawSignal::DrawPixel(x, y, color) => {
                self.game_window.draw_pixel(x, y, color);
            }
        }
    }
}
impl Default for Window {
    fn default() -> Self {
        Self {
            game_window: GameWindow::default(),
        }
    }
}

impl eframe::App for Gpu {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let iter = self.signal_receiver.try_iter();
        for signal in iter {
            self.window.process_draw_signal(signal.clone());
        }
        self.window.game_window.update_texture(ctx);
        let size = vec2(
            GAME_SCREEN_WIDTH as f32 * GAME_SCREEN_SCALE as f32,
            GAME_SCREEN_HEIGHT as f32 * GAME_SCREEN_SCALE as f32,
        );
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("This is the main window");
            egui::Window::new("Emulator")
                .default_size(size)
                .vscroll(false)
                .show(ctx, |ui| {
                    self.window.view(ui);
                });
        });
        egui::Window::new("Colors")
            .default_size(size)
            .vscroll(false)
            .show(ctx, |ui| {
                ui.color_edit_button_srgb(&mut self.window.game_window.color_palette[0]);
                ui.color_edit_button_srgb(&mut self.window.game_window.color_palette[1]);
                ui.color_edit_button_srgb(&mut self.window.game_window.color_palette[2]);
                ui.color_edit_button_srgb(&mut self.window.game_window.color_palette[3]);
                //self.window.view(ui);
            });
    }
}
#[derive(Debug, Clone)]
pub enum DrawSignal {
    DrawPixel(usize, usize, usize),
}
