use crate::{
    audio::Audio,
    gpu::{DrawSignal, Gpu},
    ram::Ram,
};
use std::sync::{mpsc::Sender, RwLock};

pub struct Bus {
    ram: RwLock<Ram>,
    // gpu: RwLock<Gpu>,
    _audio: RwLock<Audio>,
    gpu_sender: Option<Sender<DrawSignal>>,
}
impl Bus {
    pub fn with_gpu(mut self, gpu_sender: Sender<DrawSignal>) -> Self {
        self.gpu_sender = Some(gpu_sender);
        self
    }
    pub fn fetch(&self, index: u16) -> u8 {
        self.ram.read().unwrap()[index]
    }
    pub fn write_mem(&mut self, addr: u16, content: u8) {
        self.ram.write().unwrap()[addr] = content;
    }
    pub fn send_gpu_signal(&self, signal: DrawSignal) {
        if let Some(sender) = &self.gpu_sender {
            // println!("send {signal:?} to gpu");
            let _ = sender.send(signal);
        }
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
            gpu_sender: None,
            _audio: RwLock::new(Audio),
        }
    }
}
