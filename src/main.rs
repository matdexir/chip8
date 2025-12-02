mod conf;
mod vm;

use anyhow::{Context, Result};
use clap::Parser;
use raylib::prelude::*;
use std::{collections::HashMap, fs::File, io::Read, path::PathBuf};

use crate::conf::{SCREEN_HEIGHT, SCREEN_WIDTH};
use crate::vm::Chip8VM;

const SCALE: i32 = 15;
const WINDOW_WIDTH: i32 = (SCREEN_WIDTH as i32) * SCALE;
const WINDOW_HEIGHT: i32 = (SCREEN_HEIGHT as i32) * SCALE;
const TICK_PER_FRAME: usize = 10;

// This struct defines the command-line arguments using clap's derive API.
#[derive(Parser, Debug)]
#[command(author, version, about = "A CHIP-8 emulator written in Rust.", long_about = None)]
struct Cli {
    /// Path to the CHIP-8 ROM file to load
    rom_path: PathBuf,
}

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(&cli.rom_path) {
        eprintln!("Application Error: {:?}", e);
        std::process::exit(1);
    }
}

// The run function now accepts the validated ROM path as an argument.
fn run(rom_path: &PathBuf) -> Result<()> {
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

    // 1. Load ROM file using the provided PathBuf
    let mut rom =
        File::open(rom_path).context(format!("Failed to open ROM file: {}", rom_path.display()))?;

    let mut buffer = Vec::new();
    rom.read_to_end(&mut buffer)
        .context("Failed to read ROM file content")?;

    let mut chip8 = Chip8VM::new();

    // 2. Load ROM into VM
    chip8
        .load(&buffer)
        .context("Failed to load ROM data into VM memory")?;

    // 3. Initialize Raylib window
    let (mut rl, thread) = raylib::init()
        .size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .title("Chip 8 EMU")
        .build();

    rl.set_target_fps(120);

    let audio = raylib::core::audio::RaylibAudio::init_audio_device()?;
    let beep = audio.new_sound("resources/beep.mp3")?;

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
        let (_, st) = chip8.tick_timers();
        if st == 1 {
            beep.play();
        }

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

    Ok(())
}
