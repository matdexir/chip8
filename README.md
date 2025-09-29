# 👾 Rust Chip-8 Emulator

A simple, fast, and full-featured Chip-8 emulator built in Rust using the `raylib` library for graphics and `clap` for command-line argument parsing.

## ✨ Features

* Full Chip-8 instruction set emulation.

* Real-time drawing using the `raylib` windowing and graphics library.

* Graceful error handling for file operations and invalid opcodes via `anyhow`.

* Clean command-line interface using `clap` for easy ROM loading.

* Accurate timer and sound timer decrement logic.

## 🛠️ Prerequisites

To build and run this emulator, you need:

1. **Rust and Cargo:** The official Rust toolchain.

2. **Raylib Dependencies:** Since this project uses the `raylib` crate, you may need system-level dependencies for `raylib`'s underlying graphics framework (like common C/C++ build tools and necessary libraries for X11/Wayland on Linux, or development libraries on macOS/Windows).

## 🚀 Building and Running

### 1. Build the Project

Clone the repository and build the project using Cargo. Building in release mode is recommended for performance.
