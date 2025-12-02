use crate::{
    conf::{
        FLAG_COUNT, FONTSET, FONTSET_SIZE, HI_RES_HEIGHT, HI_RES_WIDTH, KEYS_COUNT, RAM_SIZE,
        REGISTER_COUNT, SCREEN_HEIGHT, SCREEN_WIDTH, STACK_SIZE, START_ADDR,
    },
    extensions::{Extension, VmContext},
};
use anyhow::{bail, Result};
use rand::random;

const MAX_SCREEN_SIZE: usize = HI_RES_HEIGHT * HI_RES_WIDTH;

pub struct CpuState {
    pc: u16,
    memory: [u8; RAM_SIZE],
    screen: [bool; MAX_SCREEN_SIZE],
    current_width: usize,
    current_height: usize,
    registers: [u8; REGISTER_COUNT],
    i_register: u16,
    sp: u16,
    stack: [u16; STACK_SIZE],
    keys: [bool; KEYS_COUNT],
    delay_timer: u8,
    sound_timer: u8,
    // S-CHIP specific
    rpl_flags: [u8; FLAG_COUNT],
}

impl Default for CpuState {
    fn default() -> Self {
        Self::new()
    }
}

impl CpuState {
    pub fn new() -> Self {
        CpuState {
            pc: START_ADDR,
            memory: [0; RAM_SIZE],
            screen: [false; MAX_SCREEN_SIZE],
            current_width: SCREEN_WIDTH,
            current_height: SCREEN_HEIGHT,
            registers: [0; REGISTER_COUNT],
            i_register: 0,
            sp: 0,
            stack: [0; STACK_SIZE],
            keys: [false; KEYS_COUNT],
            delay_timer: 0,
            sound_timer: 0,
            rpl_flags: [0; FLAG_COUNT],
        }
    }
    fn get_context(&mut self) -> VmContext<'_> {
        VmContext {
            pc: &mut self.pc,
            registers: &mut self.registers,
            i_register: &mut self.i_register,
            stack: &mut self.stack,
            sp: &mut self.sp,
            memory: &mut self.memory,
            screen: &mut self.screen,
            keys: &mut self.keys,
            delay_timer: &mut self.delay_timer,
            sound_timer: &mut self.sound_timer,
            current_width: &mut self.current_width,
            current_height: &mut self.current_height,
            rpl_flags: &mut self.rpl_flags,
        }
    }
    pub fn reset(&mut self) {
        self.pc = START_ADDR;
        self.memory.fill(0);
        self.screen.fill(false);
        self.current_width = SCREEN_WIDTH;
        self.current_height = SCREEN_HEIGHT;
        self.registers.fill(0);
        self.i_register = 0;
        self.sp = 0;
        self.stack.fill(0);
        self.keys.fill(false);
        self.delay_timer = 0;
        self.sound_timer = 0;
        self.memory[..FONTSET_SIZE].copy_from_slice(&FONTSET);
        self.rpl_flags.fill(0);
    }
}

pub struct Chip8VM {
    cpu: CpuState,
    extensions: Vec<Box<dyn Extension>>,
}

impl Default for Chip8VM {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl Chip8VM {
    pub fn new(mut extensions: Vec<Box<dyn Extension>>) -> Self {
        let mut chip8vm = Chip8VM {
            cpu: CpuState::new(),
            extensions: Vec::new(),
        };
        for mut ext in extensions.drain(..) {
            let mut ctx = chip8vm.cpu.get_context();
            ext.initialize(&mut ctx);
            chip8vm.extensions.push(ext);
        }

        chip8vm.cpu.reset();
        chip8vm
    }

    pub fn load(&mut self, data: &[u8]) -> Result<()> {
        let start = START_ADDR as usize;
        let end = start + data.len();

        if end >= RAM_SIZE {
            bail!("ROM size exceeds available memory.");
        }

        self.cpu.memory[start..end].copy_from_slice(data);
        Ok(())
    }

    pub fn tick(&mut self) -> Result<()> {
        let op = self.fetch();
        self.execute(op)
    }

    pub fn tick_timers(&mut self) -> (u8, u8) {
        if self.cpu.delay_timer > 0 {
            self.cpu.delay_timer -= 1;
        }

        if self.cpu.sound_timer > 0 {
            if self.cpu.sound_timer == 1 {
                // BEEP
            }
            self.cpu.sound_timer -= 1;
        }

        (self.cpu.delay_timer, self.cpu.sound_timer)
    }

    pub fn get_display_config(&self) -> (usize, usize, &[bool]) {
        (
            self.cpu.current_width,
            self.cpu.current_height,
            &self.cpu.screen,
        )
    }

    pub fn keypress(&mut self, idx: usize, pressed: bool) -> Result<()> {
        if idx >= KEYS_COUNT {
            bail!("Invalid key index: {}", idx);
        }
        self.cpu.keys[idx] = pressed;
        Ok(())
    }

    fn fetch(&mut self) -> u16 {
        let hi = self.cpu.memory[self.cpu.pc as usize] as u16;
        let lo = self.cpu.memory[(self.cpu.pc + 1) as usize] as u16;
        let op = (hi << 8) | lo;
        self.cpu.pc += 2;
        op
    }

    fn execute(&mut self, op: u16) -> Result<()> {
        {
            let mut ctx = self.cpu.get_context();
            let extensions = &mut self.extensions;

            for extension in extensions.iter_mut() {
                if extension.is_active() {
                    if extension.handle_instruction(&mut ctx, op)? {
                        return Ok(());
                    }
                }
            }
        }
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
            (0, 0, 0xE, 0) => {
                let current_w = self.cpu.current_width;
                let current_h = self.cpu.current_height;

                for y in 0..current_h {
                    for x in 0..current_w {
                        self.cpu.screen[x + y * HI_RES_WIDTH] = false;
                    }
                }
            }

            // RET: 0x00EE
            (0, 0, 0xE, 0xE) => self.cpu.pc = self.pop_from_stack()?,

            // JMP NNN: 0x1NNN
            (1, _, _, _) => {
                let nnn = op & 0xFFF;
                self.cpu.pc = nnn;
            }

            // CALL NNN: 0x2NNN
            (2, _, _, _) => {
                let nnn = op & 0xFFF;
                self.push_to_stack(self.cpu.pc)?;
                self.cpu.pc = nnn;
            }

            // SKIP VX == NN: 0x3XNN
            (3, _, _, _) => {
                let nn = (op & 0xFF) as u8;
                if self.cpu.registers[x] == nn {
                    self.cpu.pc += 2;
                }
            }

            // SKIP VX != NN: 0x4XNN
            (4, _, _, _) => {
                let nn = (op & 0xFF) as u8;
                if self.cpu.registers[x] != nn {
                    self.cpu.pc += 2;
                }
            }

            // SKIP VX == VY: 0x5XY0
            (5, _, _, 0) => {
                if self.cpu.registers[x] == self.cpu.registers[y] {
                    self.cpu.pc += 2;
                }
            }

            // VX = NN: 0x6XNN
            (6, _, _, _) => {
                let nn = (op & 0xFF) as u8;
                self.cpu.registers[x] = nn;
            }

            // VX += NN: 0x7XNN
            (7, _, _, _) => {
                let nn = (op & 0xFF) as u8;
                self.cpu.registers[x] = self.cpu.registers[x].wrapping_add(nn);
            }

            // 8XYN Opcode Group
            (8, _, _, 0) => self.cpu.registers[x] = self.cpu.registers[y],
            (8, _, _, 1) => self.cpu.registers[x] |= self.cpu.registers[y],
            (8, _, _, 2) => self.cpu.registers[x] &= self.cpu.registers[y],
            (8, _, _, 3) => self.cpu.registers[x] ^= self.cpu.registers[y],
            (8, _, _, 4) => {
                let (new_vx, carry) = self.cpu.registers[x].overflowing_add(self.cpu.registers[y]);
                self.cpu.registers[x] = new_vx;
                self.cpu.registers[0xF] = if carry { 1 } else { 0 };
            }
            (8, _, _, 5) => {
                let (new_vx, borrow) = self.cpu.registers[x].overflowing_sub(self.cpu.registers[y]);
                self.cpu.registers[x] = new_vx;
                self.cpu.registers[0xF] = if borrow { 0 } else { 1 };
            }
            (8, _, _, 6) => {
                self.cpu.registers[0xF] = self.cpu.registers[x] & 0x1;
                self.cpu.registers[x] >>= 1;
            }
            (8, _, _, 7) => {
                let (new_vx, borrow) = self.cpu.registers[y].overflowing_sub(self.cpu.registers[x]);
                self.cpu.registers[x] = new_vx;
                self.cpu.registers[0xF] = if borrow { 0 } else { 1 };
            }
            (8, _, _, 0xE) => {
                self.cpu.registers[0xF] = (self.cpu.registers[x] >> 7) & 0x1;
                self.cpu.registers[x] <<= 1;
            }

            // SKIP if VX != VY: 0x9XY0
            (9, _, _, 0) => {
                if self.cpu.registers[x] != self.cpu.registers[y] {
                    self.cpu.pc += 2;
                }
            }

            // I = NNN: 0xANNN
            (0xA, _, _, _) => {
                self.cpu.i_register = op & 0xFFF;
            }

            // JMP to V0 + NNN: 0xBNNN
            (0xB, _, _, _) => {
                let nnn = op & 0xFFF;
                self.cpu.pc = (self.cpu.registers[0] as u16) + nnn;
            }

            // VX = rand() & NN: 0xCXNN
            (0xC, _, _, _) => {
                let nn = (op & 0xFF) as u8;
                let rng: u8 = random();
                self.cpu.registers[x] = rng & nn;
            }

            // DRAW sprite: 0xDNNN
            (0xD, _, _, n) => {
                self.cpu.registers[0xF] = 0;
                let x_coord = self.cpu.registers[x] as usize;
                let y_coord = self.cpu.registers[y] as usize;
                let screen_width = self.cpu.current_width;
                let screen_height = self.cpu.current_height;

                for y_line in 0..n as usize {
                    let addr = self.cpu.i_register as usize + y_line;

                    if addr >= RAM_SIZE {
                        bail!("Memory access out of bounds for sprite draw");
                    }
                    let pixels = self.cpu.memory[addr];

                    for x_line in 0..8 {
                        if (pixels & (0b1000_0000 >> x_line)) != 0 {
                            let px = (x_coord + x_line) % screen_width;
                            let py = (y_coord + y_line) % screen_height;
                            let idx = px + py * HI_RES_WIDTH;
                            if self.cpu.screen[idx] {
                                self.cpu.registers[0xF] = 1;
                            }
                            self.cpu.screen[idx] ^= true;
                        }
                    }
                }
            }

            // EX9E: Skip if key pressed
            (0xE, _, 9, 0xE) => {
                let vx = self.cpu.registers[x] as usize;
                if vx >= KEYS_COUNT {
                    bail!("Invalid key index in register VX: {}", vx);
                }
                if self.cpu.keys[vx] {
                    self.cpu.pc += 2;
                }
            }

            // EXA1: Skip if key not pressed
            (0xE, _, 0xA, 1) => {
                let vx = self.cpu.registers[x] as usize;
                if vx >= KEYS_COUNT {
                    bail!("Invalid key index in register VX: {}", vx);
                }
                if !self.cpu.keys[vx] {
                    self.cpu.pc += 2;
                }
            }

            // FX07: VX = DT
            (0xF, _, 0, 7) => {
                self.cpu.registers[x] = self.cpu.delay_timer;
            }

            // FX0A: Wait for key press
            (0xF, _, 0, 0xA) => {
                let mut pressed_key = None;
                for i in 0..KEYS_COUNT {
                    if self.cpu.keys[i] {
                        pressed_key = Some(i as u8);
                        break;
                    }
                }

                if let Some(key) = pressed_key {
                    self.cpu.registers[x] = key;
                } else {
                    self.cpu.pc -= 2;
                }
            }

            // FX15: DT = VX
            (0xF, _, 1, 5) => {
                self.cpu.delay_timer = self.cpu.registers[x];
            }

            // FX18: ST = VX
            (0xF, _, 1, 8) => {
                self.cpu.sound_timer = self.cpu.registers[x];
            }

            // FX1E: I += VX
            (0xF, _, 1, 0xE) => {
                self.cpu.i_register = self
                    .cpu
                    .i_register
                    .wrapping_add(self.cpu.registers[x] as u16)
            }

            // FX29: I = font addr for VX
            (0xF, _, 2, 9) => {
                let c = self.cpu.registers[x] as u16;
                self.cpu.i_register = c * 5;
            }

            // FX33: Store BCD representation of VX
            (0xF, _, 3, 3) => {
                let vx = self.cpu.registers[x];
                let i_usize = self.cpu.i_register as usize;
                self.cpu.memory[i_usize] = vx / 100;
                self.cpu.memory[i_usize + 1] = (vx / 10) % 10;
                self.cpu.memory[i_usize + 2] = vx % 10;
            }

            // FX55: Store V0..VX in memory
            (0xF, _, 5, 5) => {
                let i = self.cpu.i_register as usize;
                if i + x >= RAM_SIZE {
                    bail!("Memory store out of bounds");
                }
                for idx in 0..=x {
                    self.cpu.memory[i + idx] = self.cpu.registers[idx];
                }
            }

            // FX65: Load V0..VX from memory
            (0xF, _, 6, 5) => {
                let i = self.cpu.i_register as usize;
                if i + x >= RAM_SIZE {
                    bail!("Memory load out of bounds");
                }
                for idx in 0..=x {
                    self.cpu.registers[idx] = self.cpu.memory[i + idx];
                }
            }

            _ => bail!("Unimplemented or unknown opcode: {:#X}", op),
        }
        Ok(())
    }

    fn push_to_stack(&mut self, val: u16) -> Result<()> {
        if self.cpu.sp as usize >= STACK_SIZE {
            bail!("Stack overflow");
        }
        self.cpu.stack[self.cpu.sp as usize] = val;
        self.cpu.sp += 1;
        Ok(())
    }

    fn pop_from_stack(&mut self) -> Result<u16> {
        if self.cpu.sp == 0 {
            bail!("Stack underflow");
        }
        self.cpu.sp -= 1;
        Ok(self.cpu.stack[self.cpu.sp as usize])
    }
}
