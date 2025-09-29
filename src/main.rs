mod vm;
use raylib::prelude::*;
use std::env;

use crate::vm::{SCREEN_HEIGHT, SCREEN_WIDTH};

const SCALE: i32 = 15;
const WINDOW_WIDTH: i32 = (SCREEN_WIDTH as i32) * SCALE;
const WINDOW_HEIGHT: i32 = (SCREEN_HEIGHT as i32) * SCALE;

fn main() {
    let args: Vec<_> = env::args().collect();

    if args.len() != 2 {
        println!("usage: cargo run path/to/rom");
        return;
    }

    let (mut rl, thread) = raylib::init()
        .size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .title("Chip 8 EMU")
        .build();

    while !rl.window_should_close() {
        let mut d = rl.begin_drawing(&thread);

        d.clear_background(Color::WHITE);
        d.draw_text("Hello, world!", 12, 12, 20, Color::BLACK);
    }
}
