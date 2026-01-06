mod conf;
mod extensions;
mod superchip;
mod vm;
mod xo;

use anyhow::{Context, Result};
use clap::Parser;
use raylib::prelude::*;
use std::{collections::HashMap, fs::File, io::Read, path::PathBuf};

use crate::conf::{HI_RES_HEIGHT, HI_RES_WIDTH, XO_RES_WIDTH};
use crate::extensions::Extension;
use crate::superchip::SuperChip8;
use crate::vm::Chip8VM;
use crate::xo::XoChip;

const SCALE: i32 = 10;
const TICK_PER_FRAME: usize = 10;

const COLOR_00: Color = Color::BLACK;
const COLOR_01: Color = Color::DARKGREEN;
const COLOR_10: Color = Color::GREEN;
const COLOR_11: Color = Color::WHITE;

// This struct defines the command-line arguments using clap's derive API.
#[derive(Parser, Debug)]
#[command(author, version, about = "A CHIP-8 emulator written in Rust.", long_about = None)]
struct Cli {
    /// Path to the CHIP-8 ROM file to load
    rom_path: PathBuf,

    #[arg(short = 's', long)]
    enable_schip: bool,

    #[arg(short = 'x', long)]
    enable_xochip: bool,
}

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(&cli) {
        eprintln!("Application Error: {:?}", e);
        std::process::exit(1);
    }
}

// The run function now accepts the validated ROM path as an argument.
fn run(cli: &Cli) -> Result<()> {
    let keytobtn: HashMap<KeyboardKey, u8> = HashMap::from([
        (KeyboardKey::KEY_ONE, 0x1),
        (KeyboardKey::KEY_TWO, 0x2),
        (KeyboardKey::KEY_THREE, 0x3),
        (KeyboardKey::KEY_FOUR, 0xC),
        (KeyboardKey::KEY_Q, 0x4),
        (KeyboardKey::KEY_W, 0x5),
        (KeyboardKey::KEY_E, 0x6),
        (KeyboardKey::KEY_R, 0xD),
        (KeyboardKey::KEY_A, 0x7),
        (KeyboardKey::KEY_S, 0x8),
        (KeyboardKey::KEY_D, 0x9),
        (KeyboardKey::KEY_F, 0xE),
        (KeyboardKey::KEY_Z, 0xA),
        (KeyboardKey::KEY_X, 0x0),
        (KeyboardKey::KEY_C, 0xB),
        (KeyboardKey::KEY_V, 0xF),
    ]);
    let mut extensions = Vec::new();
    if cli.enable_schip {
        extensions.push(Box::new(SuperChip8::new(true)) as Box<dyn Extension>);
    }
    if cli.enable_xochip {
        extensions.push(Box::new(XoChip::new(true)) as Box<dyn Extension>);
    }

    let mut rom = File::open(&cli.rom_path).context(format!(
        "Failed to open ROM file: {}",
        &cli.rom_path.display()
    ))?;

    let mut buffer = Vec::new();
    rom.read_to_end(&mut buffer)
        .context("Failed to read ROM file content")?;

    let mut chip8 = Chip8VM::new(extensions);

    chip8
        .load(&buffer)
        .context("Failed to load ROM data into VM memory")?;

    let window_width = (HI_RES_WIDTH as i32) * SCALE;
    let window_height = (HI_RES_HEIGHT as i32) * SCALE;

    let (mut rl, thread) = raylib::init()
        .size(window_width, window_height)
        .title("Chip 8 EMU(Extensible)")
        .build();

    // let mut gui = RaylibGui::new(&mut rl, &thread);
    // let mut open = true;

    rl.set_target_fps(120);

    let audio = raylib::core::audio::RaylibAudio::init_audio_device()?;
    let beep = audio.new_sound("resources/beep.mp3")?;

    // Main emulation loop
    while !rl.window_should_close() {
        // Input handling
        // gui.update(&mut rl);
        //
        // let ui = gui.new_frame();
        // ui.show_demo_window(&mut open);

        for (keyboard_key, chip8_key) in &keytobtn {
            let key_index = *chip8_key as usize;

            if rl.is_key_down(*keyboard_key) {
                if let Err(e) = chip8.keypress(key_index, true) {
                    eprintln!("Input error (down): {}", e);
                }
            } else if rl.is_key_up(*keyboard_key) {
                if let Err(e) = chip8.keypress(key_index, false) {
                    eprintln!("Input error (up): {}", e);
                }
            }
        }

        // VM Ticks
        for _ in 0..TICK_PER_FRAME {
            chip8.tick()?;
        }

        // Timer update
        let (_, st) = chip8.tick_timers();
        if st == 1 {
            beep.play();
        }

        // Drawing
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::BLACK);

        let (screen_width, screen_height, screen_buf) = chip8.get_display_config();
        let xo_planes = chip8.get_xo_planes();

        let x_offset = (window_width - (screen_width as i32) * SCALE) / 2;
        let y_offset = (window_height - (screen_height as i32) * SCALE) / 2;

        for y in 0..screen_height {
            for x in 0..screen_width {
                let pixel_color = if let Some((plane_1, plane_2)) = xo_planes {
                    let idx = x + y * XO_RES_WIDTH;
                    let p1 = plane_1[idx];
                    let p2 = plane_2[idx];
                    match (p1, p2) {
                        (false, false) => COLOR_00,
                        (false, true) => COLOR_01,
                        (true, false) => COLOR_10,
                        (true, true) => COLOR_11,
                    }
                } else {
                    let idx = x + y * HI_RES_WIDTH;
                    if screen_buf[idx] {
                        Color::GREEN
                    } else {
                        Color::BLACK
                    }
                };

                d.draw_rectangle(
                    x_offset + (x as i32) * SCALE,
                    y_offset + (y as i32) * SCALE,
                    SCALE,
                    SCALE,
                    pixel_color,
                );
            }
        }

        let screen_rect = Rectangle::new(
            x_offset as f32,
            y_offset as f32,
            (screen_width as i32 * SCALE) as f32,
            (screen_height as i32 * SCALE) as f32,
        );

        d.draw_rectangle_lines_ex(screen_rect, 2.0, Color::GRAY);
        // gui.render();
    }

    Ok(())
}
