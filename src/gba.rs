use std::time::Duration;

use crate::{bus::Bus, cpu::Cpu};

pub struct Gba {
    cpu: Cpu,
}
impl Gba {
    pub fn run(mut self) {
        for _ in 0..1000000 {
            self.cpu.step();
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}
impl Default for Gba {
    fn default() -> Gba {
        Self {
            cpu: Cpu::new(Bus::default()),
        }
    }
}
