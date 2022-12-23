use std::time::{Duration, Instant};

use crate::{
    bus::{Bus, OpCode},
    gpu::DrawSignal,
    instruction::{AddressMove, Instruction},
};
const CLOCK_SPEED: usize = 4194304;
const _FPS: f32 = 60.;
pub struct Cpu {
    bus: Bus,
    // memory model for the registers:
    // [
    //  [A,F], 0
    //  [B,C], 1
    //  [D,E], 2
    //  [H,L], 3
    //  [PC],  4
    //  [SP],  5
    // ]
    registers: [u16; 6],
    cycles: usize,
    mode: CpuMode,
}
#[derive(PartialEq, Debug, Clone)]
pub enum CpuMode {
    Run,
    _Halt,
    _DebugGpu,
    Shutdown,
}
impl Cpu {
    pub fn new(bus: Bus) -> Self {
        Self {
            bus,
            registers: [0; 6],
            cycles: 0,
            mode: CpuMode::Run,
        }
    }
    pub fn run(mut self) {
        while self.mode != CpuMode::Shutdown {
            self.cycles = 0;
            let now = Instant::now();
            while self.cycles < CLOCK_SPEED {
                self.cycles += 1;
                for _i in 0..10 {
                    let y = rand::random::<usize>();
                    let x = rand::random::<usize>();
                    let signal = DrawSignal::DrawPixel(x % 100, y % 100, self.cycles % 4);
                    self.bus.send_gpu_signal(signal);
                    let signal = DrawSignal::DrawPixel((x % 100), (y % 100) + 1, self.cycles % 4);
                    self.bus.send_gpu_signal(signal);
                    let signal = DrawSignal::DrawPixel((x % 100) + 1, (y % 100), self.cycles % 4);
                    self.bus.send_gpu_signal(signal);
                    let signal =
                        DrawSignal::DrawPixel((x % 100) + 1, (y % 100) + 1, self.cycles % 4);
                    self.bus.send_gpu_signal(signal);
                }
                self.cycles += self.step();
            }
            let elapsed = now.elapsed();
            println!("elapsed {}", elapsed.as_millis());
            if elapsed.as_secs() < 1 {
                std::thread::sleep(Duration::from_secs(1) - elapsed);
            }
        }
    }
    pub fn set_mode(&mut self, mode: CpuMode) {
        self.mode = mode;
    }
    pub fn r<R: Read>(&mut self, reg: R) -> R::Value {
        self.incr_cycles();
        reg.read(self)
    }
    pub fn w<W: Write>(&mut self, reg: W, value: W::Value) {
        self.incr_cycles();
        reg.write(self, value);
    }
    pub fn incr_cycles(&mut self) {
        self.cycles += 1;
    }
    /// Writes the number in the content register to memory at the address saved in addr.
    /// Costs 2 cycles (two reads)
    pub fn write_mem8(&mut self, addr: V16, content: V8) {
        let addr = self.r(addr);
        let content = self.r(content);
        self.bus.write_mem(addr, content);
    }
    /// Writes the raw content to the addr
    /// Costs doesnt cost any cycles
    pub fn write_mem16_raw(&mut self, addr: u16, content: u8) {
        self.bus.write_mem(addr, content);
    }
    /// Return the current value of the program counter register
    fn pc(&mut self) -> u16 {
        self.r(V16::PC)
    }
    /// fetches the next byte from memory at the progam counter.
    /// PC will be incremented by 1
    fn next_byte(&mut self) -> u8 {
        let pc = self.pc();
        self.bus.fetch(pc)
    }
    /// fetches the next word(2 bytes) from memory at the progam counter.
    /// PC will be incremented by 2
    fn next_word(&mut self) -> u16 {
        u16::from_ne_bytes([self.next_byte(), self.next_byte()])
    }
    /// returns the cycles needed for this step
    pub fn step(&mut self) -> usize {
        if self.mode != CpuMode::Run {
            return 0;
        }
        self.cycles = 0;
        let pc = self.pc();
        let op = self.bus.fetch_op(pc);
        let instruction = Instruction::from(op);
        let address_move = self.execute(instruction, op);
        self.w(V16::PC, address_move.apply(pc));
        self.cycles
    }
    fn execute(&mut self, instruction: Instruction, op: OpCode) -> AddressMove {
        let op = op.0;
        let n0 = (op & 0xF0) >> 4; // first nibble of op
        let n1 = op & 0x0F; // second nibble of op
        use Instruction::*;
        match instruction {
            Nop => AddressMove::Add(1),
            Load16Mem => {
                let new = self.next_word();
                self.w(V16::from(n0), new);
                AddressMove::Add(3)
            }
            Store8Mem => {
                let address = match (n0, n1) {
                    (0x0, 0x2) => V16::BC,
                    (0x1, 0x2) => V16::DE,
                    (0x7, _) => V16::HL,
                    _ => panic!(),
                };
                let content = match (n0, n1) {
                    (0x0..=0x2, 0x2) => V8::A,
                    (0x7, 0x0) => V8::B,
                    (0x7, 0x1) => V8::C,
                    (0x7, 0x2) => V8::D,
                    (0x7, 0x3) => V8::E,
                    (0x7, 0x4) => V8::H,
                    (0x7, 0x5) => V8::L,
                    (0x7, 0x7) => V8::A,
                    _ => panic!(),
                };
                self.write_mem8(address, content);
                AddressMove::Add(1)
            }
            Decrement16 => {
                let old_reg = match n0 {
                    0x0 => V16::BC,
                    0x1 => V16::DE,
                    0x2 => V16::HL,
                    0x3 => V16::SP,
                    _ => panic!(),
                };

                let old = self.r(old_reg);
                self.set_zero(old == 1);
                self.set_subtract(true);
                self.set_half_carry(old == 0x100);
                self.w(old_reg, old.wrapping_sub(1));
                AddressMove::Add(1)
            }
            Increment16 => {
                let old = self.r(V16::from(n0));
                self.set_zero(old == u16::MAX);
                self.set_subtract(false);
                self.set_half_carry(old == 0xff);
                self.w(V16::from(n0), old.wrapping_add(1));
                AddressMove::Add(1)
            }
            Increment8 => {
                let reg = match (n0, n1) {
                    (0x0, 0x4) => V8::B,
                    (0x1, 0x4) => V8::D,
                    (0x2, 0x4) => V8::H,
                    (0x0, 0xC) => V8::C,
                    (0x1, 0xC) => V8::E,
                    (0x2, 0xC) => V8::L,
                    (0x3, 0xC) => V8::A,
                    _ => panic!(),
                };
                let old = self.r(reg);
                self.set_zero(old == u8::MAX);
                self.set_subtract(false);
                self.set_half_carry(old == 0xf);
                self.w(reg, old.wrapping_add(1));
                AddressMove::Add(1)
            }
            Decrement8 => {
                let reg = match (n0, n1) {
                    (0x0, 0x5) => V8::B,
                    (0x1, 0x5) => V8::D,
                    (0x2, 0x5) => V8::H,
                    (0x0, 0xD) => V8::C,
                    (0x1, 0xD) => V8::E,
                    (0x2, 0xD) => V8::L,
                    (0x3, 0xD) => V8::A,
                    _ => panic!(),
                };
                let old = self.r(reg);
                self.set_zero(old == 1);
                self.set_subtract(true);
                self.set_half_carry(old == 0x10);
                self.w(reg, old.wrapping_sub(1));
                AddressMove::Add(1)
            }
            Load8Mem => {
                let n = self.next_byte();
                let reg = match (n0, n1) {
                    (0x0, 0x6) => V8::B,
                    (0x1, 0x6) => V8::D,
                    (0x2, 0x6) => V8::H,
                    (0x0, 0xE) => V8::C,
                    (0x1, 0xE) => V8::E,
                    (0x2, 0xE) => V8::L,
                    (0x3, 0xE) => V8::A,
                    _ => panic!(),
                };
                self.w(reg, n);
                AddressMove::Add(2)
            }
            RotateLeftCircle => {
                let mut a = self.r(V8::A);
                a = self.rotate_left_circle(a);
                self.w(V8::A, a);
                AddressMove::Add(1)
            }
            StoreSP => {
                let pos: u16 = self.next_word();
                let content = self.r(V16::SP).to_ne_bytes();
                self.write_mem16_raw(pos, content[0]);
                self.write_mem16_raw(pos + 1, content[1]);
                AddressMove::Add(1)
            }
            Add16toHL => {
                let current = self.r(V16::HL);
                let add = self.r(match n0 {
                    0 => V16::BC,
                    1 => V16::DE,
                    2 => V16::HL,
                    3 => V16::SP,
                    _ => panic!(),
                });
                let (new, overflow) = current.overflowing_add(add);
                self.set_carry(overflow);
                self.set_subtract(false);
                self.set_half_carry((current ^ add ^ new) & 0x1000 != 0);
                self.w(V16::HL, new);

                AddressMove::Add(1)
            }
            RotateRightCircle => {
                let mut a = self.r(V8::A);
                a = self.rotate_right_circle(a);
                self.w(V8::A, a);
                AddressMove::Add(1)
            }
            RotateLeft => {
                let mut a = self.r(V8::A);
                a = self.rotate_left(a);
                self.w(V8::A, a);
                AddressMove::Add(1)
            }
            RotateRight => {
                let mut a = self.r(V8::A);
                a = self.rotate_right(a);
                self.w(V8::A, a);
                AddressMove::Add(1)
            }
            Stop => {
                todo!()
            }
            JumpRelative => {
                let distance = self.next_byte();
                let old = self.pc();
                match (n0, n1) {
                    (0x1, 0x8) => return AddressMove::To(old + distance as u16),
                    (0x2, 0x8) => {
                        if self.zero_flag() {
                            return AddressMove::To(old + distance as u16);
                        } else {
                            return AddressMove::Add(2);
                        }
                    }
                    (0x3, 0x8) => {
                        if self.carry_flag() {
                            return AddressMove::To(old + distance as u16);
                        } else {
                            return AddressMove::Add(2);
                        }
                    }
                    (0x2, 0x0) => {
                        if !self.zero_flag() {
                            return AddressMove::To(old + distance as u16);
                        } else {
                            return AddressMove::Add(2);
                        }
                    }
                    (0x3, 0x0) => {
                        if !self.carry_flag() {
                            return AddressMove::To(old + distance as u16);
                        } else {
                            return AddressMove::Add(2);
                        }
                    }
                    _ => panic!(),
                }
            }
            StoreHlIncr => {
                let hl = V16::HL;
                let a = V8::A;
                self.write_mem8(hl, a);
                let hl = self.r(hl);
                self.w(V16::HL, hl.wrapping_add(1));
                AddressMove::Add(1)
            }
            StoreHlDecr => {
                let hl = V16::HL;
                let a = V8::A;
                self.write_mem8(hl, a);
                let hl = self.r(hl);
                self.w(V16::HL, hl.wrapping_sub(1));
                AddressMove::Add(1)
            }
            Daa => {
                let a = self.r(V8::A);
                let mut adjust = 0;
                if self.half_carry_flag() {
                    adjust |= 0x06;
                }
                if self.carry_flag() {
                    adjust |= 0x60;
                }
                if self.subtract_flag() {
                    if a & 0xF > 0x9 {
                        adjust |= 0x06;
                    }
                    if a >= 0xa0 {
                        adjust |= 0x60;
                    }
                }
                let (res, overflow) = a.overflowing_add(adjust);
                self.w(V8::A, res);
                self.set_carry(overflow);
                self.set_zero(res == 0);
                self.set_half_carry(false);
                AddressMove::Add(1)
            }
            Load8MemHlIncr => {
                let position = self.r(V16::HL);
                let content = self.bus.fetch(position);
                self.w(V8::A, content);
                self.w(V16::HL, position.wrapping_add(1));
                AddressMove::Add(1)
            }
            Load8MemHlDecr => {
                let position = self.r(V16::HL);
                let content = self.bus.fetch(position);
                self.w(V8::A, content);
                self.w(V16::HL, position.wrapping_sub(1));
                AddressMove::Add(1)
            }
            ComplementA => {
                let old = self.r(V8::A);
                let new = !old;
                self.w(V8::A, new);
                self.set_half_carry(true);
                self.set_subtract(true);
                AddressMove::Add(1)
            }
            IncMemHl => {
                let hl = self.r(V16::HL);
                let n = self.bus.fetch(hl);
                let (res, overflow) = n.overflowing_add(1);
                self.write_mem16_raw(hl, res);
                self.set_carry(overflow);
                self.set_subtract(false);
                self.set_half_carry(n == 0xf);
                self.write_mem16_raw(hl, res);
                AddressMove::Add(1)
            }
            DecMemHl => {
                let hl = self.r(V16::HL);
                let n = self.bus.fetch(hl);
                let (res, overflow) = n.overflowing_sub(1);
                self.write_mem16_raw(hl, res);
                self.set_carry(overflow);
                self.set_subtract(true);
                self.set_half_carry(n == 0xf);
                self.write_mem16_raw(hl, res);
                AddressMove::Add(1)
            }
            StoreXMemHl => {
                let x = self.next_byte();
                let hl = self.r(V16::HL);
                self.write_mem16_raw(hl, x);
                AddressMove::Add(1)
            }
            SetCarryFlag => {
                self.set_carry(true);
                self.set_half_carry(false);
                self.set_zero(false);
                AddressMove::Add(1)
            }
            FlipCarryFlag => {
                let current = self.carry_flag();
                self.set_carry(!current);
                AddressMove::Add(1)
            }
            Load8into8 => {
                let into = match n0 {
                    4 => match n1 {
                        0 | 1 | 2 | 3 | 4 | 5 | 7 => V8::B,
                        8 | 9 | 0xA | 0xB | 0xC | 0xD | 0xF => V8::C,
                        _ => panic!(),
                    },
                    5 => match n1 {
                        0 | 1 | 2 | 3 | 4 | 5 | 7 => V8::D,
                        8 | 9 | 0xA | 0xB | 0xC | 0xD | 0xF => V8::E,
                        _ => panic!(),
                    },
                    6 => match n1 {
                        0 | 1 | 2 | 3 | 4 | 5 | 7 => V8::H,
                        8 | 9 | 0xA | 0xB | 0xC | 0xD | 0xF => V8::L,
                        _ => panic!(),
                    },
                    7 => V8::A,
                    _ => panic!(),
                };
                let from = match n1 {
                    0 => V8::B,
                    1 => V8::C,
                    2 => V8::D,
                    3 => V8::E,
                    4 => V8::H,
                    5 => V8::L,
                    7 => V8::A,
                    8 => V8::B,
                    9 => V8::C,
                    0xA => V8::D,
                    0xB => V8::E,
                    0xC => V8::H,
                    0xD => V8::L,
                    0xF => V8::A,
                    _ => panic!(),
                };
                let from_value = self.r(from);
                self.w(into, from_value);
                AddressMove::Add(1)
            }
            Load16Meminto8 => {
                let addr_reg = match (n0, n1) {
                    (0x0, 0xA) => V16::BC,
                    (0x1, 0xA) => V16::DE,
                    (0x4, 0x6) => V16::HL,
                    (0x5, 0x6) => V16::HL,
                    (0x6, 0x6) => V16::HL,
                    (0x4, 0xE) => V16::HL,
                    (0x5, 0xE) => V16::HL,
                    (0x6, 0xE) => V16::HL,
                    (0x7, 0xE) => V16::HL,
                    _ => panic!(),
                };
                let into = match (n0, n1) {
                    (0x0, 0xA) => V8::A,
                    (0x1, 0xA) => V8::A,
                    (0x4, 0x6) => V8::B,
                    (0x5, 0x6) => V8::D,
                    (0x6, 0x6) => V8::H,
                    (0x4, 0xE) => V8::C,
                    (0x5, 0xE) => V8::E,
                    (0x6, 0xE) => V8::L,
                    (0x7, 0xE) => V8::A,
                    _ => panic!(),
                };
                let addr = self.r(addr_reg);
                let content = self.bus.fetch(addr);
                self.w(into, content);

                AddressMove::Add(1)
            }
            Halt => {
                todo!()
            }
            Add8toA => {
                let reg = match n1 {
                    0 => V8::B,
                    1 => V8::C,
                    2 => V8::D,
                    3 => V8::E,
                    4 => V8::H,
                    5 => V8::L,
                    7 => V8::A,
                    _ => panic!(),
                };
                let a = self.r(V8::A);
                let add = self.r(reg);
                let (res, overflow) = a.overflowing_add(add);
                self.set_carry(overflow);
                self.set_zero(res == 0);
                self.set_subtract(false);
                self.set_half_carry_add(a, add);
                self.w(V8::A, res);
                AddressMove::Add(1)
            }
            AddMemToA => {
                let hl = self.r(V16::HL);
                let content = self.bus.fetch(hl);
                let a = self.r(V8::A);
                let (res, overflow) = a.overflowing_add(content);
                self.set_carry(overflow);
                self.set_zero(res == 0);
                self.set_subtract(false);
                self.set_half_carry_add(a, content);
                self.w(V8::A, res);
                AddressMove::Add(1)
            }
            Add8AndFlagToA => {
                let add_reg = match n1 {
                    0x8 => V8::B,
                    0x9 => V8::C,
                    0xA => V8::D,
                    0xB => V8::E,
                    0xC => V8::H,
                    0xD => V8::L,
                    0xF => V8::A,
                    _ => panic!(),
                };
                let adder = self.r(add_reg);

                let carry = if self.carry_flag() { 1 } else { 0 };
                let a = self.r(V8::A);
                let (temp, overflow1) = a.overflowing_add(adder);
                let (res, overflow2) = temp.overflowing_add(carry);
                self.set_carry(overflow1 || overflow2);
                self.set_zero(res == 0);
                self.set_subtract(false);
                self.set_half_carry_add(a, adder + carry);
                self.w(V8::A, res);
                AddressMove::Add(1)
            }
            AddMemHlAndFlagToA => {
                let position = self.r(V16::HL);
                let adder = self.bus.fetch(position);

                let carry = if self.carry_flag() { 1 } else { 0 };
                let a = self.r(V8::A);
                let (temp, overflow1) = a.overflowing_add(adder);
                let (res, overflow2) = temp.overflowing_add(carry);
                self.set_carry(overflow1 || overflow2);
                self.set_zero(res == 0);
                self.set_subtract(false);
                self.set_half_carry_add(a, adder + carry);
                AddressMove::Add(1)
            }
            Sub8fromA => {
                let sub_reg = match n1 {
                    0 => V8::B,
                    1 => V8::C,
                    2 => V8::D,
                    3 => V8::E,
                    4 => V8::H,
                    5 => V8::L,
                    7 => V8::A,
                    _ => panic!(),
                };
                let sub = self.r(sub_reg);
                let a = self.r(V8::A);
                let (res, overflow) = a.overflowing_sub(sub);
                self.set_carry(overflow);
                self.set_zero(res == 0);
                self.set_subtract(true);
                self.set_half_carry_sub(a, sub);
                AddressMove::Add(1)
            }
            SubMemToA => {
                let hl = self.r(V16::HL);
                let content = self.bus.fetch(hl);
                let a = self.r(V8::A);
                let (res, overflow) = a.overflowing_sub(content);
                self.set_carry(overflow);
                self.set_zero(res == 0);
                self.set_subtract(true);
                self.set_half_carry_sub(a, content);
                self.w(V8::A, res);
                AddressMove::Add(1)
            }
            Sub8AndFlagToA => {
                let sub_reg = match n1 {
                    0x8 => V8::B,
                    0x9 => V8::C,
                    0xA => V8::D,
                    0xB => V8::E,
                    0xC => V8::H,
                    0xD => V8::L,
                    0xF => V8::A,
                    _ => panic!(),
                };
                let content = self.r(sub_reg);
                let a = self.r(V8::A);
                let (res, overflow) = a.overflowing_sub(content);
                self.set_carry(overflow);
                self.set_zero(res == 0);
                self.set_subtract(true);
                self.set_half_carry_sub(a, content);
                self.w(V8::A, res);

                AddressMove::Add(1)
            }
            SubMemHlAndFlagToA => {
                let hl = self.r(V16::HL);
                let content = self.bus.fetch(hl);
                let a = self.r(V8::A);
                let (res, overflow) = a.overflowing_sub(content);
                self.set_carry(overflow);
                self.set_zero(res == 0);
                self.set_subtract(true);
                self.set_half_carry_sub(a, content);
                self.w(V8::A, res);
                AddressMove::Add(1)
            }
            And8A => {
                let sec = match n1 {
                    0 => V8::B,
                    1 => V8::C,
                    2 => V8::D,
                    3 => V8::E,
                    4 => V8::H,
                    5 => V8::L,
                    7 => V8::A,
                    _ => panic!(),
                };
                let sec = self.r(sec);
                let a = self.r(V8::A);
                let res = a & sec;
                self.set_zero(res == 0);
                self.set_subtract(false);
                self.set_half_carry(true);
                self.set_carry(false);
                self.w(V8::A, res);
                AddressMove::Add(1)
            }
            AndMemHlA => {
                let position = self.r(V16::HL);
                let content = self.bus.fetch(position);
                let a = self.r(V8::A);
                let res = a & content;
                self.set_zero(res == 0);
                self.set_subtract(false);
                self.set_half_carry(true);
                self.set_carry(false);
                self.w(V8::A, res);
                AddressMove::Add(1)
            }
            Xor8A => {
                let xor_reg = match n1 {
                    0x8 => V8::B,
                    0x9 => V8::C,
                    0xA => V8::D,
                    0xB => V8::E,
                    0xC => V8::H,
                    0xD => V8::L,
                    0xF => V8::A,
                    _ => panic!(),
                };
                let xor = self.r(xor_reg);
                let a = self.r(V8::A);
                let res = a ^ xor;
                self.set_zero(res == 0);
                self.set_subtract(false);
                self.set_half_carry(false);
                self.set_carry(false);
                self.w(V8::A, res);
                AddressMove::Add(1)
            }
            XorMemHlA => {
                let hl = self.r(V16::HL);
                let xor = self.bus.fetch(hl);
                let a = self.r(V8::A);
                let res = a ^ xor;
                self.set_zero(res == 0);
                self.set_subtract(false);
                self.set_half_carry(false);
                self.set_carry(false);
                self.w(V8::A, res);
                AddressMove::Add(1)
            }
            Or8A => {
                let or = match n1 {
                    0 => V8::B,
                    1 => V8::C,
                    2 => V8::D,
                    3 => V8::E,
                    4 => V8::H,
                    5 => V8::L,
                    7 => V8::A,
                    _ => panic!(),
                };
                let or = self.r(or);
                let a = self.r(V8::A);
                let res = a | or;
                self.set_zero(res == 0);
                self.set_subtract(false);
                self.set_half_carry(false);
                self.set_carry(false);
                self.w(V8::A, res);
                AddressMove::Add(1)
            }
            OrMemHlA => {
                let position = self.r(V16::HL);
                let content = self.bus.fetch(position);
                let a = self.r(V8::A);
                let res = a | content;
                self.set_zero(res == 0);
                self.set_subtract(false);
                self.set_half_carry(false);
                self.set_carry(false);
                self.w(V8::A, res);
                AddressMove::Add(1)
            }
            Compare8A => {
                let cmp = match n1 {
                    0 => V8::B,
                    1 => V8::C,
                    2 => V8::D,
                    3 => V8::E,
                    4 => V8::H,
                    5 => V8::L,
                    7 => V8::A,
                    _ => panic!(),
                };
                let a = self.r(V8::A);
                let cmp = self.r(cmp);
                self.set_zero(a == cmp);
                self.set_half_carry_sub(a, cmp);
                self.set_subtract(true);
                self.set_carry(a < cmp);
                AddressMove::Add(1)
            }
            CompareMemHlA => {
                let position = self.r(V16::HL);
                let cmp = self.bus.fetch(position);
                let a = self.r(V8::A);
                self.set_zero(a == cmp);
                self.set_half_carry_sub(a, cmp);
                self.set_subtract(true);
                self.set_carry(a < cmp);
                AddressMove::Add(1)
            }
            ReturnIfFlag => {
                let should_return = match (n0, n1) {
                    (0xC, 0x0) => !self.zero_flag(),
                    (0xD, 0x0) => !self.carry_flag(),
                    (0xC, 0x8) => self.zero_flag(),
                    (0xD, 0x8) => self.carry_flag(),
                    _ => panic!(),
                };
                if should_return {
                    let sp = self.r(V16::SP);
                    let lower = self.bus.fetch(sp);
                    let upper = self.bus.fetch(sp + 1);
                    self.w(V16::SP, sp + 2);
                    AddressMove::To(u16::from_ne_bytes([lower, upper]))
                } else {
                    AddressMove::Add(1)
                }
            }
            Pop16 => {
                let to = match n0 {
                    0xC => V16::BC,
                    0xD => V16::DE,
                    0xE => V16::HL,
                    0xF => V16::AF,
                    _ => panic!(),
                };
                let sp = self.r(V16::SP);
                let lower = self.bus.fetch(sp);
                let upper = self.bus.fetch(sp + 1);
                self.w(to, u16::from_ne_bytes([lower, upper]));
                self.w(V16::SP, sp + 2);
                AddressMove::Add(1)
            }
            JumpIfFlag => {
                // increments pc by 2
                let addr = self.next_word();
                let should_jump = match (n0, n1) {
                    (0xC, 0x2) => !self.zero_flag(),
                    (0xC, 0xA) => self.zero_flag(),
                    (0xD, 0x2) => !self.carry_flag(),
                    (0xD, 0xA) => self.carry_flag(),
                    _ => panic!(),
                };
                if should_jump {
                    AddressMove::To(addr)
                } else {
                    AddressMove::Add(1)
                }
            }
            Jump16 => {
                let addr = match n1 {
                    3 => self.next_word(),
                    9 => self.r(V16::HL),
                    _ => panic!(),
                };
                AddressMove::To(addr)
            }
            CallIfFlag => {
                let addr = self.next_word();
                let should_call = match (n0, n1) {
                    (0xC, 0x4) => !self.zero_flag(),
                    (0xC, 0xC) => self.zero_flag(),
                    (0xD, 0x4) => !self.carry_flag(),
                    (0xD, 0xC) => self.carry_flag(),
                    _ => panic!(),
                };
                if should_call {
                    let pc = self.r(V16::PC);
                    let pc_bytes = pc.to_ne_bytes();
                    let sp = self.r(V16::SP);
                    self.w(V16::SP, sp - 2);
                    self.write_mem16_raw(sp - 2, pc_bytes[0]);
                    self.write_mem16_raw(sp - 1, pc_bytes[1]);
                    AddressMove::To(addr)
                } else {
                    AddressMove::Add(1)
                }
            }
            Push16 => {
                let reg = match n0 {
                    0xC => V16::BC,
                    0xD => V16::DE,
                    0xE => V16::HL,
                    0xF => V16::AF,
                    _ => panic!(),
                };
                let content = self.r(reg).to_ne_bytes();
                let sp = self.r(V16::SP);
                self.write_mem16_raw(sp - 2, content[0]);
                self.write_mem16_raw(sp - 1, content[1]);
                self.w(V16::SP, sp - 2);

                AddressMove::Add(1)
            }
            Add8ImmToA => {
                let add = self.next_byte();
                let a = self.r(V8::A);
                let (res, overflow) = a.overflowing_add(add);
                self.set_carry(overflow);
                self.set_zero(res == 0);
                self.set_subtract(false);
                self.set_half_carry_add(a, add);
                self.w(V8::A, res);
                AddressMove::Add(1)
            }
            Sub8ImmToA => {
                let sub = self.next_byte();
                let a = self.r(V8::A);
                let (res, overflow) = a.overflowing_sub(sub);
                self.set_carry(overflow);
                self.set_zero(res == 0);
                self.set_subtract(true);
                self.set_half_carry_sub(a, sub);
                AddressMove::Add(1)
            }
            And8ImmToA => {
                let sec = self.next_byte();
                let a = self.r(V8::A);
                let res = a & sec;
                self.set_zero(res == 0);
                self.set_subtract(false);
                self.set_half_carry(true);
                self.set_carry(false);
                self.w(V8::A, res);
                AddressMove::Add(1)
            }
            Or8ImmToA => {
                let or = self.next_byte();
                let a = self.r(V8::A);
                let res = a | or;
                self.set_zero(res == 0);
                self.set_subtract(false);
                self.set_half_carry(false);
                self.set_carry(false);
                self.w(V8::A, res);
                AddressMove::Add(1)
            }
            CallN => {
                let content = self.r(V16::PC).to_ne_bytes();
                let sp = self.r(V16::SP);
                self.write_mem16_raw(sp - 2, content[0]);
                self.write_mem16_raw(sp - 1, content[1]);
                self.w(V16::SP, sp - 2);
                let dest = match (n0, n1) {
                    (0xC, 0x7) => 0x00,
                    (0xD, 0x7) => 0x10,
                    (0xE, 0x7) => 0x20,
                    (0xF, 0x7) => 0x30,
                    (0xC, 0xF) => 0x08,
                    (0xD, 0xF) => 0x18,
                    (0xE, 0xF) => 0x28,
                    (0xF, 0xF) => 0x38,
                    _ => panic!(),
                };
                AddressMove::To(dest)
            }
            Return => {
                let sp = self.r(V16::SP);
                let lower = self.bus.fetch(sp);
                let upper = self.bus.fetch(sp + 1);

                self.w(V16::SP, sp + 2);
                AddressMove::To(u16::from_ne_bytes([lower, upper]))
            }
            ReturnInterrupt => {
                todo!()
            }
            Call => {
                let new_pc = self.next_word();
                let position = self.pc().to_ne_bytes();
                let sp = self.r(V16::SP);
                self.write_mem16_raw(sp - 2, position[0]);
                self.write_mem16_raw(sp - 1, position[1]);
                self.w(V16::SP, sp - 2);
                AddressMove::To(new_pc)
            }
            AddImmAndFlagToA => {
                let add = self.next_byte();
                let carry = if self.carry_flag() { 1 } else { 0 };
                let a = self.r(V8::A);
                let (temp, overflow1) = a.overflowing_add(add);
                let (res, overflow2) = temp.overflowing_add(carry);
                self.set_carry(overflow1 || overflow2);
                self.set_zero(res == 0);
                self.set_subtract(false);
                self.set_half_carry_add(a, add + carry);
                self.w(V8::A, res);
                AddressMove::Add(1)
            }
            SubImmAndFlagToA => {
                let sub = self.next_byte();
                let carry = if self.carry_flag() { 1 } else { 0 };
                let a = self.r(V8::A);
                let (temp, overflow1) = a.overflowing_sub(sub);
                let (res, overflow2) = temp.overflowing_sub(carry);
                self.set_carry(overflow1 || overflow2);
                self.set_zero(res == 0);
                self.set_subtract(true);
                self.set_half_carry_sub(a, sub + carry);
                self.w(V8::A, res);
                AddressMove::Add(1)
            }
            XorImmToA => {
                let xor = self.next_byte();
                let a = self.r(V8::A);
                let res = a ^ xor;
                self.set_carry(false);
                self.set_zero(res == 0);
                self.set_subtract(false);
                self.set_half_carry(false);
                self.w(V8::A, res);
                AddressMove::Add(1)
            }
            CompareImmToA => {
                let cmp = self.next_byte();
                let a = self.r(V8::A);
                self.set_zero(a == cmp);
                self.set_half_carry_sub(a, cmp);
                self.set_carry(a < cmp);
                self.set_subtract(true);
                AddressMove::Add(1)
            }
            StoreAToIoImm => {
                todo!()
            }
            ReadAFromIoImm => {
                let lower = 0xff;
                let upper = self.next_byte();
                let content = self.bus.fetch(u16::from_ne_bytes([lower, upper]));
                self.w(V8::A, content);
                AddressMove::Add(1)
            }
            StoreAToIoC => {
                todo!()
            }
            ReadAFromIoC => {
                let lower = 0xff;
                let upper = self.r(V8::C);
                let content = self.bus.fetch(u16::from_ne_bytes([lower, upper]));
                self.w(V8::A, content);
                AddressMove::Add(1)
            }
            DisableInterrupts => {
                todo!()
            }
            AddImmAsSignedToSp => {
                todo!() // scary command..
            }
            StoreAinMemHl => {
                self.write_mem8(V16::HL, V8::A);
                AddressMove::Add(1)
            }
            LoadAfromMemHl => {
                let hl = self.r(V16::HL);
                let content = self.bus.fetch(hl);
                self.w(V8::A, content);
                AddressMove::Add(1)
            }
            LoadSignedImmPlusSpInHl => {
                todo!()
            }
            LoadHlinSp => {
                todo!()
            }
            EnableInterrupts => {
                todo!()
            }
            TwoByteInstruction => {
                todo!()
            }
        }
    }
    /// returns true if the subtraction flag is set
    /// increments the cycles by one
    fn subtract_flag(&mut self) -> bool {
        let current = self.r(V8::F);
        current & 0x80 != 0
    }
    /// returns true if the zero flag is set
    /// increments the cycles by one
    fn zero_flag(&mut self) -> bool {
        let current = self.r(V8::F);
        current & 0x40 != 0
    }
    /// returns true if the half carry flag is set
    /// increments the cycles by one
    fn half_carry_flag(&mut self) -> bool {
        let current = self.r(V8::F);
        current & 0x20 != 0
    }
    /// returns true if the carry flag is set
    /// increments the cycles by one
    fn carry_flag(&mut self) -> bool {
        let current = self.r(V8::F);
        current & 0x10 != 0
    }

    fn set_subtract(&mut self, v: bool) {
        let flag_reg = V8::F;
        let current = flag_reg.read(self);
        if v {
            flag_reg.write(self, current | 0x80);
        } else {
            flag_reg.write(self, current ^ 0x80);
        }
    }
    fn set_carry(&mut self, v: bool) {
        let flag_reg = V8::F;
        let current = flag_reg.read(self);
        if v {
            flag_reg.write(self, current | 0x10);
        } else {
            flag_reg.write(self, current ^ 0x10);
        }
    }
    fn set_half_carry(&mut self, v: bool) {
        let flag_reg = V8::F;
        let current = flag_reg.read(self);
        if v {
            flag_reg.write(self, current | 0x20);
        } else {
            flag_reg.write(self, current ^ 0x20);
        }
    }
    fn set_half_carry_add(&mut self, v1: u8, v2: u8) {
        let v = (v1 & 0xf) + (v2 & 0xf) > 0xf;
        let flag_reg = V8::F;
        let current = flag_reg.read(self);
        if v {
            flag_reg.write(self, current | 0x20);
        } else {
            flag_reg.write(self, current ^ 0x20);
        }
    }
    fn set_half_carry_sub(&mut self, v1: u8, v2: u8) {
        let v = (v2 & 0xf) > (v1 & 0xf);
        let flag_reg = V8::F;
        let current = flag_reg.read(self);
        if v {
            flag_reg.write(self, current | 0x20);
        } else {
            flag_reg.write(self, current ^ 0x20);
        }
    }
    fn set_zero(&mut self, v: bool) {
        let flag_reg = V8::F;
        let current = flag_reg.read(self);
        if v {
            flag_reg.write(self, current | 0x40);
        } else {
            flag_reg.write(self, current ^ 0x40);
        }
    }
    fn rotate_left_circle(&mut self, v: u8) -> u8 {
        self.set_carry(v & 0xf0 != 0);
        self.set_zero(v == 0);
        self.set_subtract(false);
        self.set_half_carry(false);
        (v << 1) | (v >> 7)
    }
    fn rotate_left(&mut self, v: u8) -> u8 {
        self.set_carry(v & 0xf0 != 0);
        self.set_zero(v == 0);
        self.set_subtract(false);
        self.set_half_carry(false);
        v << 1
    }
    fn rotate_right_circle(&mut self, v: u8) -> u8 {
        self.set_carry(v & 0x0 != 0);
        self.set_zero(v == 0);
        self.set_subtract(false);
        self.set_half_carry(false);
        (v >> 1) | ((v % 2) << 7)
    }
    fn rotate_right(&mut self, v: u8) -> u8 {
        self.set_carry(v & 0x0 != 0);
        self.set_zero(v == 0);
        self.set_subtract(false);
        self.set_half_carry(false);
        v >> 1
    }
}

pub trait Read {
    type Value;
    fn read(&self, cpu: &Cpu) -> Self::Value;
}
pub trait Write {
    type Value;
    fn write(&self, cpu: &mut Cpu, v: Self::Value);
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum V8 {
    A,
    B,
    C,
    D,
    E,
    F,
    H,
    L,
}
impl Read for V8 {
    type Value = u8;

    fn read(&self, cpu: &Cpu) -> Self::Value {
        let left = |i: usize| cpu.registers[i].to_ne_bytes()[0];
        let right = |i: usize| cpu.registers[i].to_ne_bytes()[1];
        match self {
            V8::B => left(0),
            V8::C => right(0),
            V8::D => left(1),
            V8::E => right(1),
            V8::H => left(2),
            V8::L => right(2),
            V8::A => left(3),
            V8::F => right(3),
        }
    }
}
impl Write for V8 {
    type Value = u8;
    fn write(&self, cpu: &mut Cpu, v: Self::Value) {
        let set_left = |i: usize, v: u8| {
            let right = cpu.registers[i].to_ne_bytes()[1];

            return u16::from_ne_bytes([v, right]);
        };
        let set_right = |i: usize, v: u8| {
            let left = cpu.registers[i].to_ne_bytes()[0];
            return u16::from_ne_bytes([left, v]);
        };
        use V8::*;
        match self {
            B => cpu.registers[0] = set_left(0, v),
            C => cpu.registers[0] = set_right(0, v),
            D => cpu.registers[0] = set_left(1, v),
            E => cpu.registers[0] = set_right(1, v),
            H => cpu.registers[0] = set_left(2, v),
            L => cpu.registers[0] = set_right(2, v),
            A => cpu.registers[0] = set_left(3, v),
            F => cpu.registers[0] = set_right(3, v),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum V16 {
    AF,
    BC,
    DE,
    HL,
    PC,
    SP,
}
impl From<u8> for V16 {
    fn from(v: u8) -> Self {
        match v {
            0 => V16::BC,
            1 => V16::DE,
            2 => V16::HL,
            3 => V16::AF,
            4 => V16::PC,
            5 => V16::SP,
            _ => panic!("tried to access 16bit register {v}"),
        }
    }
}
impl Read for V16 {
    type Value = u16;
    fn read(&self, cpu: &Cpu) -> Self::Value {
        match self {
            V16::BC => cpu.registers[0],
            V16::DE => cpu.registers[1],
            V16::HL => cpu.registers[2],
            V16::AF => cpu.registers[3],
            V16::PC => cpu.registers[4],
            V16::SP => cpu.registers[5],
        }
    }
}
impl Write for V16 {
    type Value = u16;

    fn write(&self, cpu: &mut Cpu, v: Self::Value) {
        match self {
            V16::BC => cpu.registers[0] = v,
            V16::DE => cpu.registers[1] = v,
            V16::HL => cpu.registers[2] = v,
            V16::AF => cpu.registers[3] = v,
            V16::PC => cpu.registers[4] = v,
            V16::SP => cpu.registers[5] = v,
        }
    }
}
