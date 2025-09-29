mod vm;
use anyhow::{bail, Context, Result};
use raylib::prelude::*;
use std::{collections::HashMap, env, fs::File, io::Read};

use crate::vm::{Chip8VM, SCREEN_HEIGHT, SCREEN_WIDTH};

const SCALE: i32 = 15;
const WINDOW_WIDTH: i32 = (SCREEN_WIDTH as i32) * SCALE;
const WINDOW_HEIGHT: i32 = (SCREEN_HEIGHT as i32) * SCALE;
const TICK_PER_FRAME: usize = 10;

// The main entry point calls run and handles any top-level error.
fn main() {
    // If run() returns an Err, we print the error chain to the console.
    if let Err(e) = run() {
        eprintln!("Application Error: {:?}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
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

    let args: Vec<_> = env::args().collect();

    if args.len() != 2 {
        bail!("Usage: cargo run -- <path/to/rom>. Please provide the path to a ROM file.");
    }

    let rom_path = &args[1];

    let mut rom =
        File::open(rom_path).context(format!("Failed to open ROM file at path: {}", rom_path))?;

    let mut buffer = Vec::new();

    rom.read_to_end(&mut buffer)
        .context("Failed to read ROM file content")?;

    let mut chip8 = Chip8VM::new();

    chip8
        .load(&buffer)
        .context("Failed to load ROM data into VM memory")?;

    let (mut rl, thread) = raylib::init()
        .size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .title("Chip 8 EMU")
        .build();

    rl.set_target_fps(30);

    // Main emulation loop
    while !rl.window_should_close() {
        // Input handling
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
        chip8.tick_timers();

        // Drawing
        let mut d = rl.begin_drawing(&thread);

        d.clear_background(Color::WHITE);
        let screen_buf = chip8.get_display();

        for (i, pixel) in screen_buf.iter().enumerate() {
            if *pixel {
                let x = (i % SCREEN_WIDTH) as i32;
                let y = (i / SCREEN_WIDTH) as i32;

                d.draw_rectangle(x * SCALE, y * SCALE, SCALE, SCALE, Color::BLACK);
            }
        }
    }

    // Successful exit
    Ok(())
}
