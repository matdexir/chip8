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

### For Cargo builds:
1. **Rust and Cargo:** The official Rust toolchain.

2. **Raylib Dependencies:** Since this project uses the `raylib` crate, you may need system-level dependencies for `raylib`'s underlying graphics framework (like common C/C++ build tools and necessary libraries for X11/Wayland on Linux, or development libraries on macOS/Windows).

### For Bazel builds:
1. **Bazel or Bazelisk:** The Bazel build system (Bazelisk is recommended for automatic version management).

2. **C++ Toolchain:** A working C++ compiler (GCC or Clang) as Bazel's Rust rules require it.

3. **Raylib Dependencies:** Same system dependencies as Cargo builds.

## 🚀 Building and Running

### Option 1: Building with Cargo

Clone the repository and build the project using Cargo. Building in release mode is recommended for performance.

```bash
# Clone the repository
git clone <repository-url>
cd chip8

# Build in debug mode (faster compilation)
cargo build

# Build in release mode (better performance)
cargo build --release

# Run the emulator
./target/release/chip8 <path-to-rom>
```

### Option 2: Building with Bazel

The project also supports Bazel build system. This is useful for reproducible builds and dependency management.

```bash
# Clone the repository
git clone <repository-url>
cd chip8

# Build everything (library + binary)
bazel build //...

# Build just the binary
bazel build //:chip8_bin

# Run the emulator
./bazel-bin/chip8_bin <path-to-rom>
```

**Bazel targets:**
- `//:chip8` - Rust library containing core VM modules
- `//:chip8_bin` - Main executable

### Build Configuration

Both build systems produce the same binary. The Bazel build uses `rules_rust` with bzlmod for dependency management and automatically resolves all Rust dependencies from `Cargo.toml`.

## 📁 Project Structure

```
chip8/
├── src/
│   ├── main.rs          # Main executable entry point
│   ├── lib.rs           # Library root (for wasm crate)
│   ├── conf.rs          # Configuration constants
│   ├── debugger.rs      # Debugger functionality
│   ├── extensions.rs    # VM extension traits
│   ├── superchip.rs     # Super-CHIP extension
│   └── vm.rs            # Core CHIP-8 VM implementation
├── wasm/                # WebAssembly build (separate Cargo project)
├── resources/           # Asset files (beep.mp3)
├── test-roms/           # Test ROM files
├── Cargo.toml           # Cargo configuration
├── Cargo.lock           # Cargo dependency lock file
├── MODULE.bazel         # Bazel module configuration
├── BUILD.bazel          # Bazel build targets
├── .bazelrc             # Bazel build settings
└── .gitignore           # Git ignore rules (includes Bazel)
```

## 🎮 Usage

```bash
# Run with a CHIP-8 ROM
./target/release/chip8 path/to/rom.ch8

# Enable Super-CHIP mode
./target/release/chip8 --enable-schip path/to/rom.ch8

# Start in debug mode
./target/release/chip8 --debug path/to/rom.ch8
```
