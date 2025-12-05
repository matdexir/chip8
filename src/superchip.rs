use anyhow::{bail, Ok, Result};

use crate::{
    conf::{
        HI_RES_HEIGHT, HI_RES_WIDTH, LARGE_FONT_BASE_ADDR, RAM_SIZE, SCREEN_HEIGHT, SCREEN_WIDTH,
    },
    extensions::{Extension, VmContext},
};

pub struct SuperChip8 {
    active: bool,
}

impl SuperChip8 {
    pub fn new(active: bool) -> Self {
        SuperChip8 { active }
    }

    /// Implements the S-CHIP DXY0 instruction (Draw 16x16 sprite)
    fn draw_16x16_sprite(&mut self, ctx: &mut VmContext, x_reg: usize, y_reg: usize) -> Result<()> {
        const SPRITE_SIZE: usize = 16;
        ctx.registers[0xF] = 0;

        let x_coord = ctx.registers[x_reg] as usize;
        let y_coord = ctx.registers[y_reg] as usize;

        let screen_width = *ctx.current_width;
        let screen_height = *ctx.current_height;

        for row in 0..SPRITE_SIZE {
            let addr = *ctx.i_register as usize + (row * 2);

            if addr + 1 >= RAM_SIZE {
                bail!("Memory access out of bounds for 16x16 sprite draw");
            }

            let pixels_hi = ctx.memory[addr];
            let pixels_lo = ctx.memory[addr + 1];

            for col in 0..SPRITE_SIZE {
                let pixel_bit = if col < 8 {
                    (pixels_hi & (0b1000_0000 >> col)) != 0
                } else {
                    (pixels_lo & (0b1000_0000 >> (col - 8))) != 0
                };

                if pixel_bit {
                    let px = (x_coord + col) % screen_width;
                    let py = (y_coord + col) % screen_height;

                    let idx_in_buffer = px + py * HI_RES_WIDTH;

                    if ctx.screen[idx_in_buffer] {
                        ctx.registers[0xF] = 1;
                    }
                    ctx.screen[idx_in_buffer] ^= true;
                }
            }
        }

        Ok(())
    }
}

impl Extension for SuperChip8 {
    fn name(&self) -> &'static str {
        "Super-CHIP"
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
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let y = ((opcode & 0x00F0) >> 4) as usize;
        let n = (opcode & 0x000F) as u8;

        match (d1, x, y, n) {
            // 00FD: Exit interpreter
            (0, 0, 0xF, 0xD) => {
                bail!("S-CHIP Exit instruction (00FD) encountered.");
            }

            // 00FE: Disable extended screen (64x32 mode)
            (0, 0, 0xF, 0xE) => {
                *ctx.current_width = SCREEN_WIDTH;
                *ctx.current_height = SCREEN_HEIGHT;
                Ok(true)
            }
            // 00FF: Enable extended screen (128x64 mode)
            (0, 0, 0xF, 0xF) => {
                *ctx.current_width = HI_RES_WIDTH;
                *ctx.current_height = HI_RES_HEIGHT;
                Ok(true)
            }
            // 0DXY0:
            (0xD, _, _, 0) => {
                self.draw_16x16_sprite(ctx, x, y)?;
                Ok(true)
            }
            // TODO:  see https://chip-8.github.io/extensions/#super-chip-10
            // 00CN: scroll down n
            (0, 0, 0xC, _) => {
                let n = n as usize;
                if n >= SCREEN_HEIGHT {
                    ctx.screen.fill(false);
                    return Ok(true);
                }

                let mut new_screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];

                for row in 0..(SCREEN_HEIGHT - n) {
                    let src_start = row * SCREEN_WIDTH;
                    let dst_start = (row + n) * SCREEN_WIDTH;

                    new_screen[dst_start..dst_start + SCREEN_WIDTH]
                        .copy_from_slice(&ctx.screen[src_start..src_start + SCREEN_WIDTH]);
                }
                ctx.screen.copy_from_slice(&new_screen);
                Ok(true)
            }
            // 00FB: scroll right 4 pixels
            (0, 0, 0xF, 0xB) => {
                const SHIFT: usize = 4;
                let mut new_screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];

                for row in 0..SCREEN_HEIGHT {
                    let src_row_start = row * SCREEN_WIDTH;
                    let dst_row_start = (row + 4) * SCREEN_HEIGHT;

                    for col in 0..(SCREEN_WIDTH - SHIFT) {
                        let src = src_row_start + col;
                        let dst = dst_row_start + col + SHIFT;
                        new_screen[dst] = ctx.screen[src];
                    }
                }
                Ok(true)
            }
            // 00FC: scroll left 4 pixels
            (0, 0, 0xF, 0xC) => {
                const SHIFT: usize = 4;
                let mut new_screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];

                for row in 0..SCREEN_HEIGHT {
                    let src_row_start = row * SCREEN_WIDTH;
                    let dst_row_start = src_row_start;

                    for col in 0..(SCREEN_WIDTH + SHIFT) {
                        let src = src_row_start + col;
                        let dst = dst_row_start + col - SHIFT;
                        new_screen[dst] = ctx.screen[src];
                    }
                }
                Ok(true)
            }
            // FX30: I = bighex based on VX
            (0xF, _, 3, 0) => {
                let c = ctx.registers[x] as u16;
                *ctx.i_register = LARGE_FONT_BASE_ADDR + c * 10;
                Ok(true)
            }
            // FX75:
            (0xF, _, 7, 5) => {
                for i in 0..=x {
                    ctx.rpl_flags[i] = ctx.registers[i];
                }
                // *ctx.pc += 2;
                Ok(true)
            }

            // FX85:
            (0xF, _, 8, 5) => {
                for i in 0..=x {
                    ctx.registers[i] = ctx.rpl_flags[i];
                }
                // *ctx.pc += 2;
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}
