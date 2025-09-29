mod vm;
use raylib::prelude::*;
use std::{collections::HashMap, env, fs::File, io::Read};

use crate::vm::{Chip8VM, SCREEN_HEIGHT, SCREEN_WIDTH};

const SCALE: i32 = 15;
const WINDOW_WIDTH: i32 = (SCREEN_WIDTH as i32) * SCALE;
const WINDOW_HEIGHT: i32 = (SCREEN_HEIGHT as i32) * SCALE;
const TICK_PER_FRAME: usize = 10;

fn main() {
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
        println!("usage: cargo run path/to/rom");
        return;
    }

    let mut chip8 = Chip8VM::new();
    let mut rom = File::open(&args[1]).expect("Unable to open file");
    let mut buffer = Vec::new();
    rom.read_to_end(&mut buffer).unwrap();
    chip8.load(&buffer);

    let (mut rl, thread) = raylib::init()
        .size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .title("Chip 8 EMU")
        .build();

    rl.set_target_fps(30);
    while !rl.window_should_close() {
        for (keyboard_key, chip8_key) in &keytobtn {
            if rl.is_key_down(*keyboard_key) {
                chip8.keypress(*chip8_key as usize, true);
            } else if rl.is_key_up(*keyboard_key) {
                chip8.keypress(*chip8_key as usize, false);
            }
        }

        for _ in 0..TICK_PER_FRAME {
            chip8.tick();
        }
        chip8.tick_timers();

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
}
