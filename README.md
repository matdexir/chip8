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

## 📊 Benchmarking

Two benchmarking tools are included to measure emulator performance.

### Criterion Benchmarks

Uses the [Criterion](https://docs.rs/criterion) statistical benchmarking framework for
precise, regression-trackable measurements. Benchmarks cover individual ticks, batch
execution, timer ticks, and Super-CHIP overhead.

```bash
cargo bench                          # run all benchmarks
cargo bench "tick/batch_10000"       # run a specific benchmark
cargo bench -- --verbose             # verbose output including plot details
```

Results include HTML reports (with plots) in `target/criterion/`.

### Standalone Benchmark Binary

A CLI tool for quick ad-hoc speed tests without Criterion's overhead.

```bash
# Basic usage (debug mode)
cargo run --bin benchmark -- test-roms/1-chip8-logo.ch8

# Release mode for accurate numbers
cargo run --release --bin benchmark -- test-roms/1-chip8-logo.ch8 -n 1000000

# All options
cargo run --release --bin benchmark -- <ROM> \
    -n <ticks>             # number of ticks (default: 1,000,000)
    -w <warmup>            # warmup ticks (default: 10,000)
    -t                     # include timer ticks in measurement
    -s                     # enable Super-CHIP extension
    --samples <N>          # record per-tick timing samples
```

### Example Results (release build on Apple Silicon)

| Benchmark | Time | Throughput |
|-----------|------|------------|
| `tick/individual` | 5.4 ns | ~185 MIPS |
| `tick/batch_10000` | 54.7 µs | ~183 MIPS |
| `tick_timers` | 1.6 ns | ~613M calls/s |
| `tick/batch_1000_schip` | 8.1 µs | ~123 MIPS |
