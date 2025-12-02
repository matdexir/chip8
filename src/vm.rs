use crate::conf::{
    FONTSET, FONTSET_SIZE, KEYS_COUNT, RAM_SIZE, REGISTER_COUNT, SCREEN_HEIGHT, SCREEN_WIDTH,
    STACK_SIZE, START_ADDR,
};
use anyhow::{bail, Result};
use rand::random;

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

impl Default for Chip8VM {
    fn default() -> Self {
        Self::new()
    }
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
        chip8vm.reset();
        chip8vm
    }

    pub fn reset(&mut self) {
        self.pc = START_ADDR;
        self.memory.fill(0);
        self.screen.fill(false);
        self.registers.fill(0);
        self.i_register = 0;
        self.sp = 0;
        self.stack.fill(0);
        self.keys.fill(false);
        self.delay_timer = 0;
        self.sound_timer = 0;
        self.memory[..FONTSET_SIZE].copy_from_slice(&FONTSET);
    }

    pub fn load(&mut self, data: &[u8]) -> Result<()> {
        let start = START_ADDR as usize;
        let end = start + data.len();

        if end >= RAM_SIZE {
            bail!("ROM size exceeds available memory.");
        }

        self.memory[start..end].copy_from_slice(data);
        Ok(())
    }

    pub fn tick(&mut self) -> Result<()> {
        let op = self.fetch();
        self.execute(op)
    }

    pub fn tick_timers(&mut self) -> (u8, u8) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }

        if self.sound_timer > 0 {
            if self.sound_timer == 1 {
                // BEEP
            }
            self.sound_timer -= 1;
        }

        (self.delay_timer, self.sound_timer)
    }

    pub fn get_display(&self) -> &[bool] {
        &self.screen
    }

    pub fn keypress(&mut self, idx: usize, pressed: bool) -> Result<()> {
        if idx >= KEYS_COUNT {
            bail!("Invalid key index: {}", idx);
        }
        self.keys[idx] = pressed;
        Ok(())
    }

    fn fetch(&mut self) -> u16 {
        let hi = self.memory[self.pc as usize] as u16;
        let lo = self.memory[(self.pc + 1) as usize] as u16;
        let op = (hi << 8) | lo;
        self.pc += 2;
        op
    }

    fn execute(&mut self, op: u16) -> Result<()> {
        let d1 = (op & 0xF000) >> 12;
        let d2 = ((op & 0x0F00) >> 8) as u8;
        let d3 = ((op & 0x00F0) >> 4) as u8;
        let d4 = (op & 0x000F) as u8;
        let x = d2 as usize;
        let y = d3 as usize;

        match (d1, d2, d3, d4) {
            // NOP
            (0, 0, 0, 0) => (),

            // CLS: 0x00E0
            (0, 0, 0xE, 0) => self.screen.fill(false),

            // RET: 0x00EE
            (0, 0, 0xE, 0xE) => self.pc = self.pop_from_stack()?,

            // JMP NNN: 0x1NNN
            (1, _, _, _) => {
                let nnn = op & 0xFFF;
                self.pc = nnn;
            }

            // CALL NNN: 0x2NNN
            (2, _, _, _) => {
                let nnn = op & 0xFFF;
                self.push_to_stack(self.pc)?;
                self.pc = nnn;
            }

            // SKIP VX == NN: 0x3XNN
            (3, _, _, _) => {
                let nn = (op & 0xFF) as u8;
                if self.registers[x] == nn {
                    self.pc += 2;
                }
            }

            // SKIP VX != NN: 0x4XNN
            (4, _, _, _) => {
                let nn = (op & 0xFF) as u8;
                if self.registers[x] != nn {
                    self.pc += 2;
                }
            }

            // SKIP VX == VY: 0x5XY0
            (5, _, _, 0) => {
                if self.registers[x] == self.registers[y] {
                    self.pc += 2;
                }
            }

            // VX = NN: 0x6XNN
            (6, _, _, _) => {
                let nn = (op & 0xFF) as u8;
                self.registers[x] = nn;
            }

            // VX += NN: 0x7XNN
            (7, _, _, _) => {
                let nn = (op & 0xFF) as u8;
                self.registers[x] = self.registers[x].wrapping_add(nn);
            }

            // 8XYN Opcode Group
            (8, _, _, 0) => self.registers[x] = self.registers[y],
            (8, _, _, 1) => self.registers[x] |= self.registers[y],
            (8, _, _, 2) => self.registers[x] &= self.registers[y],
            (8, _, _, 3) => self.registers[x] ^= self.registers[y],
            (8, _, _, 4) => {
                let (new_vx, carry) = self.registers[x].overflowing_add(self.registers[y]);
                self.registers[x] = new_vx;
                self.registers[0xF] = if carry { 1 } else { 0 };
            }
            (8, _, _, 5) => {
                let (new_vx, borrow) = self.registers[x].overflowing_sub(self.registers[y]);
                self.registers[x] = new_vx;
                self.registers[0xF] = if borrow { 0 } else { 1 };
            }
            (8, _, _, 6) => {
                self.registers[0xF] = self.registers[x] & 0x1;
                self.registers[x] >>= 1;
            }
            (8, _, _, 7) => {
                let (new_vx, borrow) = self.registers[y].overflowing_sub(self.registers[x]);
                self.registers[x] = new_vx;
                self.registers[0xF] = if borrow { 0 } else { 1 };
            }
            (8, _, _, 0xE) => {
                self.registers[0xF] = (self.registers[x] >> 7) & 0x1;
                self.registers[x] <<= 1;
            }

            // SKIP if VX != VY: 0x9XY0
            (9, _, _, 0) => {
                if self.registers[x] != self.registers[y] {
                    self.pc += 2;
                }
            }

            // I = NNN: 0xANNN
            (0xA, _, _, _) => {
                self.i_register = op & 0xFFF;
            }

            // JMP to V0 + NNN: 0xBNNN
            (0xB, _, _, _) => {
                let nnn = op & 0xFFF;
                self.pc = (self.registers[0] as u16) + nnn;
            }

            // VX = rand() & NN: 0xCXNN
            (0xC, _, _, _) => {
                let nn = (op & 0xFF) as u8;
                let rng: u8 = random();
                self.registers[x] = rng & nn;
            }

            // DRAW sprite: 0xDNNN
            (0xD, _, _, n) => {
                self.registers[0xF] = 0;
                let x_coord = self.registers[x] as usize;
                let y_coord = self.registers[y] as usize;

                for y_line in 0..n as usize {
                    let addr = self.i_register as usize + y_line;

                    if addr >= RAM_SIZE {
                        bail!("Memory access out of bounds for sprite draw");
                    }
                    let pixels = self.memory[addr];

                    for x_line in 0..8 {
                        if (pixels & (0b1000_0000 >> x_line)) != 0 {
                            let px = (x_coord + x_line) % SCREEN_WIDTH;
                            let py = (y_coord + y_line) % SCREEN_HEIGHT;
                            let idx = px + py * SCREEN_WIDTH;
                            if self.screen[idx] {
                                self.registers[0xF] = 1;
                            }
                            self.screen[idx] ^= true;
                        }
                    }
                }
            }

            // EX9E: Skip if key pressed
            (0xE, _, 9, 0xE) => {
                let vx = self.registers[x] as usize;
                if vx >= KEYS_COUNT {
                    bail!("Invalid key index in register VX: {}", vx);
                }
                if self.keys[vx] {
                    self.pc += 2;
                }
            }

            // EXA1: Skip if key not pressed
            (0xE, _, 0xA, 1) => {
                let vx = self.registers[x] as usize;
                if vx >= KEYS_COUNT {
                    bail!("Invalid key index in register VX: {}", vx);
                }
                if !self.keys[vx] {
                    self.pc += 2;
                }
            }

            // FX07: VX = DT
            (0xF, _, 0, 7) => {
                self.registers[x] = self.delay_timer;
            }

            // FX0A: Wait for key press
            (0xF, _, 0, 0xA) => {
                let mut pressed_key = None;
                for i in 0..KEYS_COUNT {
                    if self.keys[i] {
                        pressed_key = Some(i as u8);
                        break;
                    }
                }

                if let Some(key) = pressed_key {
                    self.registers[x] = key;
                } else {
                    self.pc -= 2;
                }
            }

            // FX15: DT = VX
            (0xF, _, 1, 5) => {
                self.delay_timer = self.registers[x];
            }

            // FX18: ST = VX
            (0xF, _, 1, 8) => {
                self.sound_timer = self.registers[x];
            }

            // FX1E: I += VX
            (0xF, _, 1, 0xE) => {
                self.i_register = self.i_register.wrapping_add(self.registers[x] as u16)
            }

            // FX29: I = font addr for VX
            (0xF, _, 2, 9) => {
                let c = self.registers[x] as u16;
                self.i_register = c * 5;
            }

            // FX33: Store BCD representation of VX
            (0xF, _, 3, 3) => {
                let vx = self.registers[x];
                let i_usize = self.i_register as usize;
                self.memory[i_usize] = vx / 100;
                self.memory[i_usize + 1] = (vx / 10) % 10;
                self.memory[i_usize + 2] = vx % 10;
            }

            // FX55: Store V0..VX in memory
            (0xF, _, 5, 5) => {
                let i = self.i_register as usize;
                if i + x >= RAM_SIZE {
                    bail!("Memory store out of bounds");
                }
                for idx in 0..=x {
                    self.memory[i + idx] = self.registers[idx];
                }
            }

            // FX65: Load V0..VX from memory
            (0xF, _, 6, 5) => {
                let i = self.i_register as usize;
                if i + x >= RAM_SIZE {
                    bail!("Memory load out of bounds");
                }
                for idx in 0..=x {
                    self.registers[idx] = self.memory[i + idx];
                }
            }

            _ => bail!("Unimplemented or unknown opcode: {:#X}", op),
        }
        Ok(())
    }

    fn push_to_stack(&mut self, val: u16) -> Result<()> {
        if self.sp as usize >= STACK_SIZE {
            bail!("Stack overflow");
        }
        self.stack[self.sp as usize] = val;
        self.sp += 1;
        Ok(())
    }

    fn pop_from_stack(&mut self) -> Result<u16> {
        if self.sp == 0 {
            bail!("Stack underflow");
        }
        self.sp -= 1;
        Ok(self.stack[self.sp as usize])
    }
}
