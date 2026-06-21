use std::path::PathBuf;
use std::time::Instant;

use clap::Parser;
use chip8::extensions::Extension;
use chip8::superchip::SuperChip8;
use chip8::vm::Chip8VM;

#[derive(Parser, Debug)]
#[command(name = "benchmark", about = "CHIP-8 speed benchmark harness")]
struct Args {
    /// Path to the ROM file
    rom: PathBuf,

    /// Number of ticks to run in the benchmark
    #[arg(short = 'n', long, default_value = "1000000")]
    count: u64,

    /// Number of warmup ticks before measurement
    #[arg(short = 'w', long, default_value = "10000")]
    warmup: u64,

    /// Enable Super-CHIP extension
    #[arg(short = 's', long)]
    schip: bool,

    /// Include timer ticks in the measurement
    #[arg(short = 't', long)]
    timers: bool,

    /// Print per-tick timing samples (first N)
    #[arg(long, default_value = "0")]
    samples: usize,
}

fn main() {
    let args = Args::parse();

    let rom = std::fs::read(&args.rom)
        .unwrap_or_else(|e| panic!("Failed to read ROM {}: {}", args.rom.display(), e));

    let mut extensions: Vec<Box<dyn Extension>> = Vec::new();
    if args.schip {
        extensions.push(Box::new(SuperChip8::new(true)));
    }

    let mut vm = Chip8VM::new(extensions);
    vm.load(&rom).unwrap();

    // === Warmup ===
    if args.warmup > 0 {
        for _ in 0..args.warmup {
            vm.tick().unwrap();
            if args.timers {
                vm.tick_timers();
            }
        }
    }

    // === Timed run ===
    let mut timings = Vec::with_capacity(args.samples as usize);
    let start = Instant::now();

    for i in 0..args.count {
        let tick_start = if args.samples > 0 && i < args.samples as u64 {
            Some(Instant::now())
        } else {
            None
        };

        vm.tick().unwrap();

        if let Some(ts) = tick_start {
            timings.push(ts.elapsed());
        }

        if args.timers {
            vm.tick_timers();
        }
    }

    let elapsed = start.elapsed();
    let total_ns = elapsed.as_nanos() as f64;
    let avg_ns = total_ns / args.count as f64;
    let ips = (args.count as f64 / elapsed.as_secs_f64()) as u64;

    // === Output ===
    println!("─── Benchmark Results ───");
    println!("  ROM:           {}", args.rom.display());
    println!("  Mode:          {}CHIP-8", if args.schip { "Super-" } else { "" });
    println!("  With timers:   {}", args.timers);
    println!();
    println!("  Ticks run:     {}", args.count);
    println!("  Warmup:        {} ticks", args.warmup);
    println!("  Total time:    {:?}", elapsed);
    println!("  Avg per tick:  {:.1} ns", avg_ns);
    println!("  IPS:           {}", ips);
    println!("  MHz (equiv):   {:.3}", ips as f64 / 1_000_000.0);

    if !timings.is_empty() {
        let min = timings.iter().min().unwrap().as_nanos() as f64;
        let max = timings.iter().max().unwrap().as_nanos() as f64;
        let sum: f64 = timings.iter().map(|t| t.as_nanos() as f64).sum();
        let mean = sum / timings.len() as f64;
        println!();
        println!("  Sample timing ({}/{} ticks):", timings.len(), args.count);
        println!("    Min:    {:.1} ns", min);
        println!("    Max:    {:.1} ns", max);
        println!("    Mean:   {:.1} ns", mean);
        println!(
            "    Spread: {:.1}%",
            (max - min) / mean * 100.0
        );
    }
}
