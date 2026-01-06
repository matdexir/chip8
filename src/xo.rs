use crate::conf::{
    HI_RES_HEIGHT, HI_RES_WIDTH, LARGE_FONT_BASE_ADDR, RAM_SIZE, SCREEN_HEIGHT, SCREEN_WIDTH,
    XO_RES_WIDTH, XO_SCREEN_SIZE,
};
use crate::extensions::{Extension, VmContext};
use anyhow::{bail, Ok, Result};

pub struct XoChip {
    active: bool,
}

impl XoChip {
    pub fn new(active: bool) -> Self {
        XoChip { active }
    }

    /// XO-Chip 00FD: Exit interpreter (not implemented, just returns error)
    fn exit_interpreter(&self) -> Result<bool> {
        bail!("XO-Chip Exit instruction (00FD) encountered.");
    }

    /// XO-Chip 00FE: Set low-resolution mode (64x32)
    fn set_low_resolution(ctx: &mut VmContext) -> Result<()> {
        *ctx.current_width = SCREEN_WIDTH;
        *ctx.current_height = SCREEN_HEIGHT;
        if *ctx.plane_mask & 0x1 != 0 {
            ctx.plane_1.fill(false);
        }
        if *ctx.plane_mask & 0x2 != 0 {
            ctx.plane_2.fill(false);
        }
        Ok(())
    }

    /// XO-Chip 00FF: Set high-resolution mode (128x64)
    fn set_high_resolution(ctx: &mut VmContext) -> Result<()> {
        *ctx.current_width = HI_RES_WIDTH;
        *ctx.current_height = HI_RES_HEIGHT;
        if *ctx.plane_mask & 0x1 != 0 {
            ctx.plane_1.fill(false);
        }
        if *ctx.plane_mask & 0x2 != 0 {
            ctx.plane_2.fill(false);
        }
        Ok(())
    }

    /// XO-Chip 00CN: Scroll down N lines
    fn scroll_down(ctx: &mut VmContext, n: u8) -> Result<()> {
        let scroll_lines = n as usize;
        if scroll_lines >= *ctx.current_height {
            if *ctx.plane_mask & 0x1 != 0 {
                ctx.plane_1.fill(false);
            }
            if *ctx.plane_mask & 0x2 != 0 {
                ctx.plane_2.fill(false);
            }
            ctx.screen.fill(false);
            return Ok(());
        }

        let screen_width = *ctx.current_width;
        let screen_height = *ctx.current_height;

        for row in (scroll_lines..screen_height).rev() {
            for col in 0..screen_width {
                let src_idx = col + (row - scroll_lines) * screen_width;
                let dst_idx = col + row * screen_width;

                if src_idx < XO_SCREEN_SIZE && dst_idx < XO_SCREEN_SIZE {
                    if *ctx.plane_mask & 0x1 != 0 {
                        ctx.plane_1[dst_idx] = ctx.plane_1[src_idx];
                    }
                    if *ctx.plane_mask & 0x2 != 0 {
                        ctx.plane_2[dst_idx] = ctx.plane_2[src_idx];
                    }
                }
            }
        }

        for row in 0..scroll_lines {
            for col in 0..screen_width {
                let idx = col + row * screen_width;
                if idx < XO_SCREEN_SIZE {
                    if *ctx.plane_mask & 0x1 != 0 {
                        ctx.plane_1[idx] = false;
                    }
                    if *ctx.plane_mask & 0x2 != 0 {
                        ctx.plane_2[idx] = false;
                    }
                }
            }
        }

        Ok(())
    }

    /// XO-Chip 00FB: Scroll right 4 pixels
    fn scroll_right(ctx: &mut VmContext) -> Result<()> {
        const SHIFT: usize = 4;
        let screen_width = *ctx.current_width;
        let screen_height = *ctx.current_height;

        for row in 0..screen_height {
            for col in (SHIFT..screen_width).rev() {
                let src_idx = (col - SHIFT) + row * screen_width;
                let dst_idx = col + row * screen_width;

                if src_idx < XO_SCREEN_SIZE && dst_idx < XO_SCREEN_SIZE {
                    if *ctx.plane_mask & 0x1 != 0 {
                        ctx.plane_1[dst_idx] = ctx.plane_1[src_idx];
                    }
                    if *ctx.plane_mask & 0x2 != 0 {
                        ctx.plane_2[dst_idx] = ctx.plane_2[src_idx];
                    }
                }
            }

            for col in 0..SHIFT {
                let idx = col + row * screen_width;
                if idx < XO_SCREEN_SIZE {
                    if *ctx.plane_mask & 0x1 != 0 {
                        ctx.plane_1[idx] = false;
                    }
                    if *ctx.plane_mask & 0x2 != 0 {
                        ctx.plane_2[idx] = false;
                    }
                }
            }
        }

        Ok(())
    }

    /// XO-Chip 00FC: Scroll left 4 pixels
    fn scroll_left(ctx: &mut VmContext) -> Result<()> {
        const SHIFT: usize = 4;
        let screen_width = *ctx.current_width;
        let screen_height = *ctx.current_height;

        for row in 0..screen_height {
            for col in 0..screen_width.saturating_sub(SHIFT) {
                let src_idx = (col + SHIFT) + row * screen_width;
                let dst_idx = col + row * screen_width;

                if src_idx < XO_SCREEN_SIZE && dst_idx < XO_SCREEN_SIZE {
                    if *ctx.plane_mask & 0x1 != 0 {
                        ctx.plane_1[dst_idx] = ctx.plane_1[src_idx];
                    }
                    if *ctx.plane_mask & 0x2 != 0 {
                        ctx.plane_2[dst_idx] = ctx.plane_2[src_idx];
                    }
                }
            }

            for col in (screen_width.saturating_sub(SHIFT))..screen_width {
                let idx = col + row * screen_width;
                if idx < XO_SCREEN_SIZE {
                    if *ctx.plane_mask & 0x1 != 0 {
                        ctx.plane_1[idx] = false;
                    }
                    if *ctx.plane_mask & 0x2 != 0 {
                        ctx.plane_2[idx] = false;
                    }
                }
            }
        }

        Ok(())
    }

    /// XO-Chip 00FCN: Scroll left N pixels (4-bit value in NN)
    fn scroll_left_n(ctx: &mut VmContext, n: u8) -> Result<()> {
        let shift = n as usize;
        if shift == 0 {
            return Ok(());
        }

        let screen_width = *ctx.current_width;
        let screen_height = *ctx.current_height;

        for row in 0..screen_height {
            for col in 0..screen_width.saturating_sub(shift) {
                let src_idx = (col + shift) + row * screen_width;
                let dst_idx = col + row * screen_width;

                if src_idx < XO_SCREEN_SIZE && dst_idx < XO_SCREEN_SIZE {
                    if *ctx.plane_mask & 0x1 != 0 {
                        ctx.plane_1[dst_idx] = ctx.plane_1[src_idx];
                    }
                    if *ctx.plane_mask & 0x2 != 0 {
                        ctx.plane_2[dst_idx] = ctx.plane_2[src_idx];
                    }
                }
            }

            for col in (screen_width.saturating_sub(shift))..screen_width {
                let idx = col + row * screen_width;
                if idx < XO_SCREEN_SIZE {
                    if *ctx.plane_mask & 0x1 != 0 {
                        ctx.plane_1[idx] = false;
                    }
                    if *ctx.plane_mask & 0x2 != 0 {
                        ctx.plane_2[idx] = false;
                    }
                }
            }
        }

        Ok(())
    }

    /// XO-Chip DXYK: Draw sprite with K lines to both planes
    fn draw_sprite(ctx: &mut VmContext, x_reg: usize, y_reg: usize, k: usize) -> Result<()> {
        ctx.registers[0xF] = 0;

        let x_coord = ctx.registers[x_reg] as usize;
        let y_coord = ctx.registers[y_reg] as usize;
        let screen_width = *ctx.current_width;
        let screen_height = *ctx.current_height;
        let plane_mask = *ctx.plane_mask;

        for row in 0..k {
            let addr = *ctx.i_register as usize + row;

            if addr >= RAM_SIZE {
                bail!("Memory access out of bounds for sprite draw");
            }

            let pixels = ctx.memory[addr];

            for col in 0..8 {
                if (pixels & (0b1000_0000 >> col)) != 0 {
                    let px = (x_coord + col) % screen_width;
                    let py = (y_coord + row) % screen_height;
                    let idx = px + py * XO_RES_WIDTH;

                    if idx >= XO_SCREEN_SIZE {
                        continue;
                    }

                    if plane_mask & 0x1 != 0 {
                        let previous_plane_1 = ctx.plane_1[idx];
                        ctx.plane_1[idx] ^= true;
                        if previous_plane_1 && !ctx.plane_1[idx] {
                            ctx.registers[0xF] = 1;
                        }
                    }

                    if plane_mask & 0x2 != 0 {
                        let previous_plane_2 = ctx.plane_2[idx];
                        ctx.plane_2[idx] ^= true;
                        if previous_plane_2 && !ctx.plane_2[idx] {
                            ctx.registers[0xF] = 1;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// XO-Chip DXY0: Draw 16x16 sprite to both planes
    fn draw_16x16_sprite(ctx: &mut VmContext, x_reg: usize, y_reg: usize) -> Result<()> {
        const SPRITE_SIZE: usize = 16;
        ctx.registers[0xF] = 0;

        let x_coord = ctx.registers[x_reg] as usize;
        let y_coord = ctx.registers[y_reg] as usize;
        let screen_width = *ctx.current_width;
        let screen_height = *ctx.current_height;
        let plane_mask = *ctx.plane_mask;
        let use_both_planes = plane_mask == 0x3;

        for row in 0..SPRITE_SIZE {
            let base_addr = *ctx.i_register as usize + (row * 2);

            if base_addr + 1 >= RAM_SIZE {
                bail!("Memory access out of bounds for 16x16 sprite draw");
            }

            let pixels_hi = ctx.memory[base_addr];
            let pixels_lo = ctx.memory[base_addr + 1];

            for col in 0..SPRITE_SIZE {
                let pixel_bit = if col < 8 {
                    (pixels_hi & (0b1000_0000 >> col)) != 0
                } else {
                    (pixels_lo & (0b1000_0000 >> (col - 8))) != 0
                };

                if pixel_bit {
                    let px = (x_coord + col) % screen_width;
                    let py = (y_coord + row) % screen_height;
                    let idx = px + py * XO_RES_WIDTH;

                    if idx >= XO_SCREEN_SIZE {
                        continue;
                    }

                    if plane_mask & 0x1 != 0 {
                        let previous_plane_1 = ctx.plane_1[idx];
                        ctx.plane_1[idx] ^= true;
                        if previous_plane_1 && !ctx.plane_1[idx] {
                            ctx.registers[0xF] = 1;
                        }
                    }

                    if plane_mask & 0x2 != 0 {
                        let pixel_bit_2 = if use_both_planes {
                            let plane2_addr =
                                *ctx.i_register as usize + (SPRITE_SIZE * 2) + (row * 2);
                            if plane2_addr + 1 >= RAM_SIZE {
                                bail!("Memory access out of bounds for 16x16 sprite plane 2");
                            }
                            let pixels_hi_2 = ctx.memory[plane2_addr];
                            let pixels_lo_2 = ctx.memory[plane2_addr + 1];
                            if col < 8 {
                                (pixels_hi_2 & (0b1000_0000 >> col)) != 0
                            } else {
                                (pixels_lo_2 & (0b1000_0000 >> (col - 8))) != 0
                            }
                        } else {
                            pixel_bit
                        };

                        let previous_plane_2 = ctx.plane_2[idx];
                        if pixel_bit_2 {
                            ctx.plane_2[idx] ^= true;
                        }
                        if previous_plane_2 && !ctx.plane_2[idx] {
                            ctx.registers[0xF] = 1;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// XO-Chip FX0F: Read 16-bit audio from memory and play
    fn play_audio(&mut self, ctx: &mut VmContext, x: usize) -> Result<()> {
        let addr_start = *ctx.i_register as usize;
        let addr_end = addr_start + (ctx.registers[x] as usize);

        if addr_end > RAM_SIZE {
            bail!("Audio buffer out of bounds");
        }

        let audio_buffer = &ctx.memory[addr_start..addr_end];

        let mut waveform: Vec<i16> = Vec::with_capacity(audio_buffer.len());
        for &sample in audio_buffer {
            waveform.push(((sample as i16) - 128) * 256);
        }

        drop(waveform);
        Ok(())
    }
}

impl Extension for XoChip {
    fn name(&self) -> &'static str {
        "XO-Chip"
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn initialize(&mut self, _ctx: &mut VmContext) {
        // NoOp
    }

    fn handle_instruction(&mut self, ctx: &mut VmContext, opcode: u16) -> Result<bool> {
        if !self.active {
            return Ok(false);
        }

        let d1 = (opcode & 0xF000) >> 12;
        let d2 = ((opcode & 0x0F00) >> 8) as u8;
        let d3 = ((opcode & 0x00F0) >> 4) as u8;
        let d4 = (opcode & 0x000F) as u8;

        let x = d2 as usize;
        let y = d3 as usize;
        let _n = d4;
        let nn = opcode & 0xFF;

        match (d1, d2, d3, d4) {
            // 00FD: Exit interpreter
            (0, 0, 0xF, 0xD) => self.exit_interpreter(),

            // 00FE: Set low-resolution mode (64x32)
            (0, 0, 0xF, 0xE) => {
                Self::set_low_resolution(ctx)?;
                Ok(true)
            }

            // 00FF: Set high-resolution mode (128x64)
            (0, 0, 0xF, 0xF) => {
                Self::set_high_resolution(ctx)?;
                Ok(true)
            }

            // 00FB: Scroll right 4 pixels
            (0, 0, 0xF, 0xB) => {
                Self::scroll_right(ctx)?;
                Ok(true)
            }

            // 00CN: Scroll down N lines (includes 00FC which also matches scroll-left-4)
            (0, 0, 0xC, n) => {
                if nn == 0xFC {
                    Self::scroll_left(ctx)?;
                } else {
                    Self::scroll_down(ctx, n)?;
                }
                Ok(true)
            }

            // DXY0: Draw 16x16 sprite
            (0xD, _, _, 0) => {
                Self::draw_16x16_sprite(ctx, x, y)?;
                Ok(true)
            }

            // DXYK: Draw sprite with K lines to both planes
            (0xD, _, _, n) if n != 0 => {
                Self::draw_sprite(ctx, x, y, n as usize)?;
                Ok(true)
            }

            // FX01: Set drawing plane bitmask (XO-Chip)
            (0xF, _, 0, 1) => {
                *ctx.plane_mask = ctx.registers[x] & 0x3;
                Ok(true)
            }

            // FX30: Set I to high-res sprite location
            (0xF, _, 3, 0) => {
                let c = ctx.registers[x] as u16;
                *ctx.i_register = LARGE_FONT_BASE_ADDR + (c * 10);
                Ok(true)
            }

            // FX0F: Read 16-bit audio from memory
            (0xF, _, 0, 0xF) => {
                self.play_audio(ctx, x)?;
                Ok(true)
            }

            // 5XY2: Save registers Vx..Vy to memory starting at I
            (5, _, _, 2) => {
                if x > y {
                    return Ok(false);
                }

                let mut current_i = *ctx.i_register;
                for reg_idx in x..=y {
                    ctx.memory[current_i as usize] = ctx.registers[reg_idx];
                    current_i += 1;
                }
                *ctx.i_register = current_i;
                Ok(true)
            }

            // 5XY3: Load registers Vx..Vy from memory starting at I
            (5, _, _, 3) => {
                if x > y {
                    return Ok(false);
                }

                let mut current_i = *ctx.i_register;
                for reg_idx in x..=y {
                    ctx.registers[reg_idx] = ctx.memory[current_i as usize];
                    current_i += 1;
                }
                *ctx.i_register = current_i;
                Ok(true)
            }

            _ => Ok(false),
        }
    }
}
