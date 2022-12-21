use crate::{audio::Audio, debugger::Debugger, gpu::Gpu, ram::Ram};
use std::sync::RwLock;

pub struct Bus {
    ram: RwLock<Ram>,
    gpu: RwLock<Gpu>,
    audio: RwLock<Audio>,
    debugger: RwLock<Debugger>,
}
impl Bus {
    pub fn fetch(&self, index: u16) -> u8 {
        self.ram.read().unwrap()[index]
    }
    pub fn write_mem(&mut self, addr: u16, content: u8) {
        self.ram.write().unwrap()[addr] = content;
    }
    pub fn fetch_op(&self, index: u16) -> OpCode {
        OpCode(self.fetch(index))
    }
}
#[derive(Clone, Debug, Copy)]
pub struct OpCode(pub u8);

impl Default for Bus {
    fn default() -> Bus {
        Bus {
            ram: RwLock::new(Ram::default()),
            gpu: RwLock::new(Gpu {}),
            audio: RwLock::new(Audio),
            debugger: RwLock::new(Debugger),
        }
    }
}
