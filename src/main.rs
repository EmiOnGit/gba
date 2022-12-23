use gba::Gba;

mod audio;
mod bus;
mod cpu;
mod debugger;
mod gba;
mod gpu;
mod instruction;
mod ram;

fn main() {
    let gba = Gba::default();
    pollster::block_on(gba.run());
}