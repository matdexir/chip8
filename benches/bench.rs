use criterion::{criterion_group, criterion_main, Criterion};
use chip8::extensions::Extension;
use chip8::superchip::SuperChip8;
use chip8::vm::Chip8VM;
use std::hint::black_box;

fn load_rom(name: &str) -> Vec<u8> {
    let path = format!("test-roms/{}", name);
    std::fs::read(&path).unwrap_or_else(|_| panic!("Failed to read ROM at: {}", path))
}

fn bench_tick_individual(c: &mut Criterion) {
    let mut vm = Chip8VM::new(Vec::new());
    let rom = load_rom("1-chip8-logo.ch8");
    vm.load(&rom).unwrap();
    // run past initial setup to reach steady-state (NOPs after program end)
    for _ in 0..1000 {
        let _ = vm.tick();
    }
    c.bench_function("tick/individual", |b| {
        b.iter(|| black_box(vm.tick()).unwrap())
    });
}

fn bench_tick_batch(c: &mut Criterion) {
    let rom = load_rom("1-chip8-logo.ch8");
    for &count in &[100, 1000, 10000] {
        let label = format!("tick/batch_{}", count);
        c.bench_function(&label, |b| {
            b.iter(|| {
                let mut vm = Chip8VM::new(Vec::new());
                vm.load(&rom).unwrap();
                for _ in 0..count {
                    black_box(vm.tick()).unwrap();
                }
            })
        });
    }
}

fn bench_tick_timers(c: &mut Criterion) {
    let mut vm = Chip8VM::new(Vec::new());
    c.bench_function("tick_timers", |b| {
        b.iter(|| black_box(vm.tick_timers()))
    });
}

fn bench_combined(c: &mut Criterion) {
    let rom = load_rom("1-chip8-logo.ch8");
    c.bench_function("tick+batch_1000_with_timers", |b| {
        b.iter(|| {
            let mut vm = Chip8VM::new(Vec::new());
            vm.load(&rom).unwrap();
            for _ in 0..1000 {
                black_box(vm.tick()).unwrap();
            }
            black_box(vm.tick_timers());
        })
    });
}

fn bench_schip(c: &mut Criterion) {
    let rom = load_rom("1-chip8-logo.ch8");
    c.bench_function("tick/batch_1000_schip", |b| {
        b.iter(|| {
            let mut vm = Chip8VM::new(vec![Box::new(SuperChip8::new(true)) as Box<dyn Extension>]);
            vm.load(&rom).unwrap();
            for _ in 0..1000 {
                black_box(vm.tick()).unwrap();
            }
        })
    });
}

fn bench_different_roms(c: &mut Criterion) {
    for name in &["1-chip8-logo.ch8", "3-corax+.ch8"] {
        let rom = load_rom(name);
        let label = format!("tick/batch_10000/rom={}", name);
        c.bench_function(&label, |b| {
            b.iter(|| {
                let mut vm = Chip8VM::new(Vec::new());
                vm.load(&rom).unwrap();
                for _ in 0..10000 {
                    black_box(vm.tick()).unwrap();
                }
            })
        });
    }
}

criterion_group!(
    benches,
    bench_tick_individual,
    bench_tick_batch,
    bench_tick_timers,
    bench_combined,
    bench_schip,
    bench_different_roms,
);
criterion_main!(benches);
