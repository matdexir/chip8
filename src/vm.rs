use crate::{
    conf::{
        FLAG_COUNT, FONTSET, FONTSET_SIZE, HI_RES_HEIGHT, HI_RES_WIDTH, KEYS_COUNT, RAM_SIZE,
        REGISTER_COUNT, SCREEN_HEIGHT, SCREEN_WIDTH, STACK_SIZE, START_ADDR, XO_RES_HEIGHT,
        XO_RES_WIDTH,
    },
    extensions::{Extension, VmContext},
};
use anyhow::{bail, Result};
use rand::random;

const MAX_SCREEN_SIZE: usize = HI_RES_HEIGHT * HI_RES_WIDTH;
const XO_SCREEN_SIZE: usize = XO_RES_HEIGHT * XO_RES_WIDTH;

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
    // XO-Chip specific
    plane_1: [bool; XO_SCREEN_SIZE],
    plane_2: [bool; XO_SCREEN_SIZE],
    plane_mask: u8,
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
            plane_1: [false; XO_SCREEN_SIZE],
            plane_2: [false; XO_SCREEN_SIZE],
            plane_mask: 1,
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
            plane_1: &mut self.plane_1,
            plane_2: &mut self.plane_2,
            plane_mask: &mut self.plane_mask,
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
        self.plane_1.fill(false);
        self.plane_2.fill(false);
        self.plane_mask = 1;
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

        if end > RAM_SIZE {
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

    pub fn get_xo_planes(&self) -> Option<(&[bool; XO_SCREEN_SIZE], &[bool; XO_SCREEN_SIZE])> {
        for extension in &self.extensions {
            if extension.name() == "XO-Chip" && extension.is_active() {
                return Some((&self.cpu.plane_1, &self.cpu.plane_2));
            }
        }
        None
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
                if extension.is_active() && extension.handle_instruction(&mut ctx, op)? {
                    return Ok(());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_initialization() {
        let vm = Chip8VM::new(Vec::new());
        assert_eq!(vm.cpu.pc, START_ADDR);
        assert_eq!(vm.cpu.i_register, 0);
        assert_eq!(vm.cpu.sp, 0);
        assert!(vm.cpu.delay_timer == 0);
        assert!(vm.cpu.sound_timer == 0);
    }

    #[test]
    fn test_load_rom() {
        let mut vm = Chip8VM::new(Vec::new());
        let rom_data = vec![0x12, 0x34, 0x56, 0x78];
        vm.load(&rom_data).unwrap();

        let start = START_ADDR as usize;
        assert_eq!(vm.cpu.memory[start], 0x12);
        assert_eq!(vm.cpu.memory[start + 1], 0x34);
        assert_eq!(vm.cpu.memory[start + 2], 0x56);
        assert_eq!(vm.cpu.memory[start + 3], 0x78);
    }

    #[test]
    fn test_load_rom_too_large() {
        let mut vm = Chip8VM::new(Vec::new());
        let rom_data = vec![0u8; 4000];
        let result = vm.load(&rom_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_nop() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.memory[0x200] = 0x00;
        vm.cpu.memory[0x201] = 0x00;
        vm.tick().unwrap();
        assert_eq!(vm.cpu.pc, 0x202);
    }

    #[test]
    fn test_cls() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.screen[0] = true;
        vm.cpu.screen[63] = true;
        vm.cpu.memory[0x200] = 0x00;
        vm.cpu.memory[0x201] = 0xE0;
        vm.tick().unwrap();
        assert!(!vm.cpu.screen[0]);
        assert!(!vm.cpu.screen[63]);
    }

    #[test]
    fn test_jmp() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.memory[0x200] = 0x12;
        vm.cpu.memory[0x201] = 0x34;
        vm.tick().unwrap();
        assert_eq!(vm.cpu.pc, 0x234);
    }

    #[test]
    fn test_call_and_ret() {
        let mut vm = Chip8VM::new(Vec::new());

        vm.cpu.memory[0x200] = 0x22;
        vm.cpu.memory[0x201] = 0x34;
        vm.tick().unwrap();
        assert_eq!(vm.cpu.pc, 0x234);
        assert_eq!(vm.cpu.sp, 1);
        assert_eq!(vm.cpu.stack[0], 0x202);

        vm.cpu.memory[0x234] = 0x00;
        vm.cpu.memory[0x235] = 0xEE;
        vm.tick().unwrap();
        assert_eq!(vm.cpu.pc, 0x202);
        assert_eq!(vm.cpu.sp, 0);
    }

    #[test]
    fn test_stack_overflow() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.memory[0x200] = 0x22;
        vm.cpu.memory[0x201] = 0x34;
        for _ in 0..16 {
            vm.cpu.pc = 0x200;
            vm.tick().unwrap();
        }
        vm.cpu.pc = 0x200;
        let result = vm.tick();
        assert!(result.is_err());
    }

    #[test]
    fn test_stack_underflow() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.memory[0x200] = 0x00;
        vm.cpu.memory[0x201] = 0xEE;
        let result = vm.tick();
        assert!(result.is_err());
    }

    #[test]
    fn test_skip_if_equal() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.registers[0] = 0x42;

        vm.cpu.memory[0x200] = 0x30;
        vm.cpu.memory[0x201] = 0x42;
        vm.tick().unwrap();
        assert_eq!(vm.cpu.pc, 0x204);

        vm.cpu.pc = 0x200;
        vm.cpu.memory[0x201] = 0x43;
        vm.tick().unwrap();
        assert_eq!(vm.cpu.pc, 0x202);
    }

    #[test]
    fn test_skip_if_not_equal() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.registers[0] = 0x42;

        vm.cpu.memory[0x200] = 0x40;
        vm.cpu.memory[0x201] = 0x43;
        vm.tick().unwrap();
        assert_eq!(vm.cpu.pc, 0x204);

        vm.cpu.pc = 0x200;
        vm.cpu.memory[0x201] = 0x42;
        vm.tick().unwrap();
        assert_eq!(vm.cpu.pc, 0x202);
    }

    #[test]
    fn test_skip_if_registers_equal() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.registers[0] = 0x42;
        vm.cpu.registers[1] = 0x42;

        vm.cpu.memory[0x200] = 0x50;
        vm.cpu.memory[0x201] = 0x10;
        vm.tick().unwrap();
        assert_eq!(vm.cpu.pc, 0x204);

        vm.cpu.pc = 0x200;
        vm.cpu.registers[1] = 0x43;
        vm.tick().unwrap();
        assert_eq!(vm.cpu.pc, 0x202);
    }

    #[test]
    fn test_skip_if_registers_not_equal() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.registers[0] = 0x42;
        vm.cpu.registers[1] = 0x43;

        vm.cpu.memory[0x200] = 0x90;
        vm.cpu.memory[0x201] = 0x10;
        vm.tick().unwrap();
        assert_eq!(vm.cpu.pc, 0x204);

        vm.cpu.pc = 0x200;
        vm.cpu.registers[1] = 0x42;
        vm.tick().unwrap();
        assert_eq!(vm.cpu.pc, 0x202);
    }

    #[test]
    fn test_set_register() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.memory[0x200] = 0x60;
        vm.cpu.memory[0x201] = 0x42;
        vm.tick().unwrap();
        assert_eq!(vm.cpu.registers[0], 0x42);
    }

    #[test]
    fn test_add_register() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.registers[0] = 0x10;
        vm.cpu.memory[0x200] = 0x70;
        vm.cpu.memory[0x201] = 0x20;
        vm.tick().unwrap();
        assert_eq!(vm.cpu.registers[0], 0x30);
    }

    #[test]
    fn test_add_register_overflow() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.registers[0] = 0xF0;
        vm.cpu.memory[0x200] = 0x70;
        vm.cpu.memory[0x201] = 0x20;
        vm.tick().unwrap();
        assert_eq!(vm.cpu.registers[0], 0x10);
    }

    #[test]
    fn test_set_i_register() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.memory[0x200] = 0xA1;
        vm.cpu.memory[0x201] = 0x23;
        vm.tick().unwrap();
        assert_eq!(vm.cpu.i_register, 0x123);
    }

    #[test]
    fn test_add_to_i_register() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.i_register = 0x100;
        vm.cpu.registers[0] = 0x50;
        vm.cpu.memory[0x200] = 0xF0;
        vm.cpu.memory[0x201] = 0x1E;
        vm.tick().unwrap();
        assert_eq!(vm.cpu.i_register, 0x150);
    }

    #[test]
    fn test_add_to_i_register_overflow() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.i_register = 0xFFF;
        vm.cpu.registers[0] = 0x10;
        vm.cpu.memory[0x200] = 0xF0;
        vm.cpu.memory[0x201] = 0x1E;
        vm.tick().unwrap();
        assert_eq!(vm.cpu.i_register, 0x100F);
    }

    #[test]
    fn test_jump_v0_offset() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.registers[0] = 0x10;
        vm.cpu.memory[0x200] = 0xB2;
        vm.cpu.memory[0x201] = 0x34;
        vm.tick().unwrap();
        assert_eq!(vm.cpu.pc, 0x244);
    }

    #[test]
    fn test_copy_register() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.registers[1] = 0x42;
        vm.cpu.memory[0x200] = 0x80;
        vm.cpu.memory[0x201] = 0x10;
        vm.tick().unwrap();
        assert_eq!(vm.cpu.registers[0], 0x42);
    }

    #[test]
    fn test_or_register() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.registers[0] = 0x0F;
        vm.cpu.registers[1] = 0xF0;
        vm.cpu.memory[0x200] = 0x80;
        vm.cpu.memory[0x201] = 0x11;
        vm.tick().unwrap();
        assert_eq!(vm.cpu.registers[0], 0xFF);
    }

    #[test]
    fn test_and_register() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.registers[0] = 0xFF;
        vm.cpu.registers[1] = 0xF0;
        vm.cpu.memory[0x200] = 0x80;
        vm.cpu.memory[0x201] = 0x12;
        vm.tick().unwrap();
        assert_eq!(vm.cpu.registers[0], 0xF0);
    }

    #[test]
    fn test_xor_register() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.registers[0] = 0xFF;
        vm.cpu.registers[1] = 0xFF;
        vm.cpu.memory[0x200] = 0x80;
        vm.cpu.memory[0x201] = 0x13;
        vm.tick().unwrap();
        assert_eq!(vm.cpu.registers[0], 0x00);
    }

    #[test]
    fn test_add_registers_no_carry() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.registers[0] = 0x10;
        vm.cpu.registers[1] = 0x20;
        vm.cpu.memory[0x200] = 0x80;
        vm.cpu.memory[0x201] = 0x14;
        vm.tick().unwrap();
        assert_eq!(vm.cpu.registers[0], 0x30);
        assert_eq!(vm.cpu.registers[0xF], 0);
    }

    #[test]
    fn test_add_registers_with_carry() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.registers[0] = 0xF0;
        vm.cpu.registers[1] = 0x20;
        vm.cpu.memory[0x200] = 0x80;
        vm.cpu.memory[0x201] = 0x14;
        vm.tick().unwrap();
        assert_eq!(vm.cpu.registers[0], 0x10);
        assert_eq!(vm.cpu.registers[0xF], 1);
    }

    #[test]
    fn test_sub_registers_no_borrow() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.registers[0] = 0x20;
        vm.cpu.registers[1] = 0x10;
        vm.cpu.memory[0x200] = 0x80;
        vm.cpu.memory[0x201] = 0x15;
        vm.tick().unwrap();
        assert_eq!(vm.cpu.registers[0], 0x10);
        assert_eq!(vm.cpu.registers[0xF], 1);
    }

    #[test]
    fn test_sub_registers_with_borrow() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.registers[0] = 0x10;
        vm.cpu.registers[1] = 0x20;
        vm.cpu.memory[0x200] = 0x80;
        vm.cpu.memory[0x201] = 0x15;
        vm.tick().unwrap();
        assert_eq!(vm.cpu.registers[0], 0xF0);
        assert_eq!(vm.cpu.registers[0xF], 0);
    }

    #[test]
    fn test_shift_right() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.registers[0] = 0b1000_0001;
        vm.cpu.memory[0x200] = 0x80;
        vm.cpu.memory[0x201] = 0x16;
        vm.tick().unwrap();
        assert_eq!(vm.cpu.registers[0], 0b0100_0000);
        assert_eq!(vm.cpu.registers[0xF], 1);
    }

    #[test]
    fn test_shift_left() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.registers[0] = 0b1000_0000;
        vm.cpu.memory[0x200] = 0x80;
        vm.cpu.memory[0x201] = 0x1E;
        vm.tick().unwrap();
        assert_eq!(vm.cpu.registers[0], 0b0000_0000);
        assert_eq!(vm.cpu.registers[0xF], 1);
    }

    #[test]
    fn test_reverse_sub_no_borrow() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.registers[0] = 0x10;
        vm.cpu.registers[1] = 0x20;
        vm.cpu.memory[0x200] = 0x80;
        vm.cpu.memory[0x201] = 0x17;
        vm.tick().unwrap();
        assert_eq!(vm.cpu.registers[0], 0x10);
        assert_eq!(vm.cpu.registers[0xF], 1);
    }

    #[test]
    fn test_timer_operations() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.registers[0] = 0x10;

        vm.cpu.memory[0x200] = 0xF0;
        vm.cpu.memory[0x201] = 0x15;
        vm.tick().unwrap();
        assert_eq!(vm.cpu.delay_timer, 0x10);

        vm.cpu.memory[0x202] = 0xF0;
        vm.cpu.memory[0x203] = 0x18;
        vm.tick().unwrap();
        assert_eq!(vm.cpu.sound_timer, 0x10);

        vm.cpu.memory[0x204] = 0xF0;
        vm.cpu.memory[0x205] = 0x07;
        vm.tick().unwrap();
        assert_eq!(vm.cpu.registers[0], 0x10);
    }

    #[test]
    fn test_tick_timers() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.delay_timer = 5;
        vm.cpu.sound_timer = 3;

        let (dt, st) = vm.tick_timers();
        assert_eq!(dt, 4);
        assert_eq!(st, 2);

        let (dt, st) = vm.tick_timers();
        assert_eq!(dt, 3);
        assert_eq!(st, 1);
    }

    #[test]
    fn test_bcd_conversion() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.i_register = 0x300;
        vm.cpu.registers[0] = 123;

        vm.cpu.memory[0x200] = 0xF0;
        vm.cpu.memory[0x201] = 0x33;
        vm.tick().unwrap();

        assert_eq!(vm.cpu.memory[0x300], 1);
        assert_eq!(vm.cpu.memory[0x301], 2);
        assert_eq!(vm.cpu.memory[0x302], 3);
    }

    #[test]
    fn test_store_registers() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.i_register = 0x300;
        vm.cpu.registers[0] = 0x10;
        vm.cpu.registers[1] = 0x20;
        vm.cpu.registers[2] = 0x30;

        vm.cpu.memory[0x200] = 0xF2;
        vm.cpu.memory[0x201] = 0x55;
        vm.tick().unwrap();

        assert_eq!(vm.cpu.memory[0x300], 0x10);
        assert_eq!(vm.cpu.memory[0x301], 0x20);
        assert_eq!(vm.cpu.memory[0x302], 0x30);
    }

    #[test]
    fn test_load_registers() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.i_register = 0x300;
        vm.cpu.memory[0x300] = 0x10;
        vm.cpu.memory[0x301] = 0x20;
        vm.cpu.memory[0x302] = 0x30;

        vm.cpu.memory[0x200] = 0xF2;
        vm.cpu.memory[0x201] = 0x65;
        vm.tick().unwrap();

        assert_eq!(vm.cpu.registers[0], 0x10);
        assert_eq!(vm.cpu.registers[1], 0x20);
        assert_eq!(vm.cpu.registers[2], 0x30);
    }

    #[test]
    fn test_font_address() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.registers[0] = 0x0A;

        vm.cpu.memory[0x200] = 0xF0;
        vm.cpu.memory[0x201] = 0x29;
        vm.tick().unwrap();

        assert_eq!(vm.cpu.i_register, 50);
    }

    #[test]
    fn test_key_press() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.keypress(0, true).unwrap();
        assert!(vm.cpu.keys[0]);

        vm.keypress(0, false).unwrap();
        assert!(!vm.cpu.keys[0]);
    }

    #[test]
    fn test_invalid_key_press() {
        let mut vm = Chip8VM::new(Vec::new());
        let result = vm.keypress(16, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_skip_if_key_pressed() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.registers[0] = 5;
        vm.cpu.keys[5] = true;

        vm.cpu.memory[0x200] = 0xE0;
        vm.cpu.memory[0x201] = 0x9E;
        vm.tick().unwrap();
        assert_eq!(vm.cpu.pc, 0x204);
    }

    #[test]
    fn test_skip_if_key_not_pressed() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.registers[0] = 5;
        vm.cpu.keys[5] = false;

        vm.cpu.memory[0x200] = 0xE0;
        vm.cpu.memory[0x201] = 0xA1;
        vm.tick().unwrap();
        assert_eq!(vm.cpu.pc, 0x204);
    }

    #[test]
    fn test_draw_sprite_no_collision() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.registers[0] = 0;
        vm.cpu.registers[1] = 0;
        vm.cpu.i_register = 0x320;
        vm.cpu.memory[0x320] = 0x80;

        vm.cpu.memory[0x200] = 0xD0;
        vm.cpu.memory[0x201] = 0x01;
        vm.tick().unwrap();

        assert!(vm.cpu.screen[0]);
        assert_eq!(vm.cpu.registers[0xF], 0);
    }

    #[test]
    fn test_draw_sprite_with_collision() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.registers[0] = 0;
        vm.cpu.registers[1] = 0;
        vm.cpu.i_register = 0x320;
        vm.cpu.memory[0x320] = 0x80;
        vm.cpu.screen[0] = true;

        vm.cpu.memory[0x200] = 0xD0;
        vm.cpu.memory[0x201] = 0x01;
        vm.tick().unwrap();

        assert!(!vm.cpu.screen[0]);
        assert_eq!(vm.cpu.registers[0xF], 1);
    }

    #[test]
    fn test_invalid_opcode() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.memory[0x200] = 0xFF;
        vm.cpu.memory[0x201] = 0xFF;
        let result = vm.tick();
        assert!(result.is_err());
    }

    #[test]
    fn test_reset() {
        let mut vm = Chip8VM::new(Vec::new());
        vm.cpu.registers[0] = 0x42;
        vm.cpu.memory[0x300] = 0x12;
        vm.cpu.screen[0] = true;

        vm.cpu.reset();

        assert_eq!(vm.cpu.pc, START_ADDR);
        assert_eq!(vm.cpu.registers[0], 0);
        assert_eq!(vm.cpu.memory[0x300], 0);
        assert!(!vm.cpu.screen[0]);
    }

    #[test]
    fn test_get_display_config() {
        let vm = Chip8VM::new(Vec::new());
        let (width, height, screen) = vm.get_display_config();
        assert_eq!(width, SCREEN_WIDTH);
        assert_eq!(height, SCREEN_HEIGHT);
        assert_eq!(screen.len(), HI_RES_HEIGHT * HI_RES_WIDTH);
    }
}
