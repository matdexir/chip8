use std::usize;
use rand::random();

const RAM_SIZE: usize = 4096;
const REGISTER_COUNT: usize = 16;
const STACK_SIZE: usize = 16;
const KEYS_COUNT: usize = 16;

pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;

const START_ADDR: u16 = 0x200;

const FONTSET_SIZE: usize = 80;
const FONTSET: [u8; FONTSET_SIZE] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

pub struct Chip8VM {
    pc: u16,
    memory: [u8; RAM_SIZE],
    screen: [bool; SCREEN_WIDTH * SCREEN_HEIGHT],
    registers: [u8; REGISTER_COUNT],
    i_register: u16,
    sp: u16,
    stack: [u16; STACK_SIZE],
    keys: [bool; KEYS_COUNT],
    delay_timer: u8,
    sound_timer: u8,
}

impl Chip8VM {
    pub fn new() -> Self {
        let mut chip8vm = Chip8VM {
            pc: START_ADDR,
            memory: [0; RAM_SIZE],
            screen: [false; SCREEN_HEIGHT * SCREEN_WIDTH],
            registers: [0; REGISTER_COUNT],
            i_register: 0,
            sp: 0,
            stack: [0; STACK_SIZE],
            keys: [false; KEYS_COUNT],
            delay_timer: 0,
            sound_timer: 0,
        };

        chip8vm.memory[..FONTSET_SIZE].copy_from_slice(&FONTSET);
        return chip8vm;
    }

    pub fn reset(&mut self) {
        self.pc = START_ADDR;
        self.memory = [0; RAM_SIZE];
        self.screen = [false; SCREEN_HEIGHT * SCREEN_WIDTH];
        self.registers = [0; REGISTER_COUNT];
        self.i_register = 0;
        self.sp = 0;
        self.stack = [0; STACK_SIZE];
        self.keys = [false; KEYS_COUNT];
        self.delay_timer = 0;
        self.sound_timer = 0;
    }

    pub fn tick(&mut self) {
        // FETCH
        let op = self.fetch();
        // DECODE
        // EXECUTE
    }

    pub fn tick_timers(&mut self) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }

        if self.sound_timer > 0 {
            if self.sound_timer == 1 {
                // BEEP
            }
            self.sound_timer -= 1;
        }
    }

    fn fetch(&mut self) -> u16 {
        let hi = self.memory[self.pc as usize] as u16;
        let lo = self.memory[(self.pc + 1) as usize] as u16;

        let op = (hi << 8) | lo;
        self.pc += 2;

        return op;
    }

    fn execute(&mut self, op: u16) {
        let d1 = (op & 0xF000) >> 12;
        let d2 = (op & 0x0F00) >> 8;
        let d3 = (op & 0x00F0) >> 4;
        let d4 = op & 0x000F;

        match (d1, d2, d3, d4) {
            // NOP
            (0, 0, 0, 0) => return,

            // CLS: 0x00E0
            (0, 0, 0xE, 0) => self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT],

            // RET: 0x00EE
            (0, 0, 0xE, 0xE) => {
                let ret = self.pop_from_stack();
                self.pc = ret;
            }

            // JMP NNN: 0x1NNN
            (1, _, _, _) => {
                let nnn = op & 0xFFF;
                self.pc = nnn;
            }

            // CALL NNN: 0x2NNN
            (2, _, _, _) => {
                let nnn = op & 0xFFF;
                self.push_to_stack(self.pc);
                self.pc = nnn;
            }
            // SKIP VX == NN: 0x3XNN
            (3, _, _, _) => {
                let x = d2 as usize;
                let nn = (op & 0xFF) as u8;

                if self.registers[x] == nn {
                    self.pc += 2;
                }
            }

            // SKIP VX == NN: 0x4XNN
            (4, _, _, _) => {
                let x = d2 as usize;
                let nn = (op & 0xFF) as u8;

                if self.registers[x] != nn {
                    self.pc += 2;
                }
            }

            // SKIP VX == VY: 0x5XY0
            (5, _, _, 0) => {
                let x = d2 as usize;
                let y = d3 as usize;

                if self.registers[x] == self.registers[y] {
                    self.pc += 2;
                }
            }

            // VX = NN: 0x6XNN
            (6, _, _, _) => {
                let x = d2 as usize;
                let nn = (op & 0xFF) as u8;

                self.registers[x] = nn;
            }

            // VX += NN: 0x7XNN
            (7, _, _, _) => {
                let x = d2 as usize;
                let nn = (op & 0xFF) as u8;

                self.registers[x] = self.registers[x].wrapping_add(nn);
            }
            // VX = VY: 0x8XY0
            (8, _, _, 0) => {
                let x = d2 as usize;
                let y = d3 as usize;

                self.registers[x] = self.registers[y];
            }

            // VX |= VY: 0x8XY1
            (8, _, _, 1) => {
                let x = d2 as usize;
                let y = d3 as usize;

                self.registers[x] |= self.registers[y];
            }

            // VX &= VY: 0x8XY2
            (8, _, _, 2) => {
                let x = d2 as usize;
                let y = d3 as usize;

                self.registers[x] &= self.registers[y];
            }

            // VX ^= VY: 0x8XY3
            (8, _, _, 3) => {
                let x = d2 as usize;
                let y = d3 as usize;

                self.registers[x] ^= self.registers[y];
            }

            // VX += VY: 0x8XY4
            // set VF(yes addr V[0xF]) to 1 if carry else 0
            (8, _, _, 4) => {
                let x = d2 as usize;
                let y = d3 as usize;

                let (new_vx, carry) = self.registers[x].overflowing_add(self.registers[y]);
                self.registers[x] = new_vx;
                self.registers[0xF] = if carry { 1 } else { 0 };
            }

            // VX -= VY: 0x8XY5
            // set VF(yes addr V[0xF]) to 0 if borrow else 1
            (8, _, _, 5) => {
                let x = d2 as usize;
                let y = d3 as usize;

                let (new_vx, borrow) = self.registers[x].overflowing_sub(self.registers[y]);
                self.registers[x] = new_vx;
                self.registers[0xF] = if borrow { 0 } else { 1 };
            }

            // VX >>=1: 0x8XN6
            // set VF to the dropped bit: LSB
            (8, _, _, 6) => {
                let x = d2 as usize;
                let lsb = self.registers[x] & 0x1;
                self.registers[x] >>= 1;
                self.registers[0xF] = lsb;
            }

            // VX = VY - VX: 0x8XY7
            // set VF(yes addr V[0xF]) to 0 if borrow else 1
            (8, _, _, 7) => {
                let x = d2 as usize;
                let y = d3 as usize;

                let (new_vx, borrow) = self.registers[y].overflowing_sub(self.registers[x]);
                self.registers[x] = new_vx;
                self.registers[0xF] = if borrow { 0 } else { 1 };
            }
            // VX <<=1: 0x8XNE
            // set VF to the dropped bit: MSB
            (8, _, _, 0xE) => {
                let x = d2 as usize;
                let msb = (self.registers[x] >> 7) & 0x1;
                self.registers[x] <<= 1;
                self.registers[0xF] = msb;
            }

            // SKIP if VX != VY: 0x9XY0
            (9, _, _, 0) => {
                let x = d2 as usize;
                let y = d3 as usize;
                if self.registers[x] != self.registers[y] {
                    self.pc += 2;
                }
            }
            // I = NNN: 0xANNN
            (0xA, _, _, _) => {
                let nnn = op & 0xFFF;
                self.i_register = nnn;
            }
            // JMP to V0 + NNN: 0xBNNN
            (0xB, _, _, _) => {
                let nnn = op & 0xFFF;
                self.pc = (self.registers[0] as u16) + nnn;
            }

            // VX = rand() & NN: 0xCXNN
            (0xC, _, _, _) => {
                let x = d2 as usize;
                let nn = (op & 0xFF) as u8;
                let rng: u8 = random();

                self.registers[x] = rng & nn;
            }

            // DRAW sprite: 0xDNNN
            (0xD, _, _, _) => {
                let x_coord = self.registers[d2 as usize] as u16;
                let y_coord = self.registers[d3 as usize] as u16;

                let num_rows = d4;
                let mut flipped = false;

                for y_line in 0..num_rows {
                    let addr  = self.i_register + y_line as u16;
                    let pixels = self.memory[addr as usize];

                    for x_line in 0..8 {
                        if (pixels & (0b1000_0000 >> x_line) )!= 0 {
                            let x = (x_coord + x_line) as usize % SCREEN_WIDTH;
                            let y = (y_coord + y_line) as usize % SCREEN_HEIGHT;

                            let idx = x + SCREEN_WIDTH * y;

                            flipped != self.screen[idx];
                            self.screen[idx] ^= true;
                        }
                    }
                }

                if flipped {
                    self.registers[0xF] = 1;
                } else {
                    self.registers[0xF] = 0;
                }
            }

            // SKIP KEY PRESS: 0xEX9E
            (0xE, _, 9, 0xE) => {
                let x = d2 as usize;
                let vx = self.registers[x];
                let key = self.keys[vx as usize];
                if key {
                    self.pc += 2;
                }
            }

            // SKIP NOT KEY PRESS: 0xEX9E
            (0xE, _, 0xA, 1) => {
                let x = d2 as usize;
                let vx = self.registers[x];
                let key = self.keys[vx as usize];
                if !key {
                    self.pc += 2;
                }
            }
            // VX = DT: 0xFX07
            (0xF, _, 0, 7) => {
                let x = d2 as usize;
                self.registers[x] = self.delay_timer;
            }
            // WAIT KEY: 0xFX0A
            (0xF, _, 0, 0xA) => {
                let x = d2 as usize;
                let mut pressed = false;
                for i in 0..self.keys.len() {
                    if self.keys[i] {
                        self.registers[x] = i as u8;
                        pressed = true
                        break;
                    }
                }

                if !pressed {
                    self.pc -=2;
                }
            }

            // DT = VX: 0xFX15
            (0xF, _, 1, 5) => {
                let x = d2 as usize;
                self.delay_timer = self.registers[x];
            }

            // ST = VX: 0xFX18
            (0xF, _, 1, 8) => {
                let x = d2 as usize;
                self.sound_timer = self.registers[x];
            }

            // I += VX: 0xFX1E
            (0xF, _, 1, 0xE) => {
                let x = d2 as usize;
                self.i_register = self.i_register.wrapping_add(self.registers[x] as u16) 
            }

            // I = FONT: 0xFX29
            (0xF, _, 2, 9) => {
                let x = d2 as usize;
                let c = self.registers[x] as u16;
                self.i_register = c * 5;
            }

            // I = BCD of  VX: 0xFX33
            // BCD = Binary Coded Decimal
            (0xF,_, 3, 3) => {
                let x = d2 as usize;
                let vx = self.registers[x] as f32;

                let hundreds = (vx / 100.0).floor() as u8;

                let tens = ((vx / 10.0) % 10.0).floor() as u8;

                let ones = (vx / 10.0) as u8;

                self.memory[self.i_register as usize] = hundreds;
                self.memory[(self.i_register + 1) as usize] = tens;
                self.memory[(self.i_register + 2) as usize] = ones;

           }
            // STORE V0 to VX: 0xFX55
            (0xF, _, 5, 5) => {
                let x = d2 as usize;
                let i = self.i_register as usize;

                for idx in 0..=x {
                    self.memory[i + idx] = self.registers[idx];
                }
            }

            // LOAD V0 to VX: 0xFX65
            (0xF, _, 6, 5) => {
                let x = d2 as usize;
                let i = self.i_register as usize;

                for idx in 0..=x{
                    self.registers[idx] = self.memory[i + idx];
                }
            }

            (_, _, _, _) => unimplemented!("Unimplemented opcode: {}", op),
        }
    }

    fn push_to_stack(self: &mut Self, val: u16) {
        self.stack[self.sp as usize] = val;
        self.sp += 1;
    }

    fn pop_from_stack(self: &mut Self) -> u16 {
        let val = self.stack[(self.sp - 1) as usize];
        self.sp -= 1;
        return val;
    }
}
