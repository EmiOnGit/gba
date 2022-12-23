use std::{
    sync::mpsc::{self, Receiver},
    thread::{self, JoinHandle},
};


use crate::{
    bus::Bus,
    cpu::Cpu,
    gpu::{DrawSignal, Gpu},
};

pub struct Gba {
    _cpu: JoinHandle<()>,
    gpu_receiver: Receiver<DrawSignal>,
}
impl Gba {
    pub async fn run(self) {
        let gpu = Gpu::new(self.gpu_receiver);
        gpu.run();
    }
}
impl Default for Gba {
    fn default() -> Gba {
        let (sender, rx) = mpsc::channel();

        Self {
            _cpu: thread::spawn(move || Cpu::new(Bus::default().with_gpu(sender)).run()),
            gpu_receiver: rx,
        }
    }
}

