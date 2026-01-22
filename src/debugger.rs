use crate::vm::CpuState;
use std::collections::HashSet;

pub enum DebugAction {
    Quit,
    Step,
    Continue,
    ShowRegisters,
    ShowMemory(u16, usize),
    ShowBreakpoints,
    Help,
}

pub struct Debugger {
    breakpoints: HashSet<u16>,
}

impl Debugger {
    pub fn new() -> Self {
        Self {
            breakpoints: HashSet::new(),
        }
    }

    pub fn should_break(&self, pc: u16) -> bool {
        self.breakpoints.contains(&pc)
    }

    pub fn set_breakpoint(&mut self, addr: u16) {
        self.breakpoints.insert(addr);
    }

    pub fn clear_breakpoint(&mut self, addr: u16) {
        self.breakpoints.remove(&addr);
    }

    pub fn parse_and_execute(
        &mut self,
        input: &str,
        _cpu: &CpuState,
    ) -> Result<DebugAction, String> {
        let input = input.trim();
        if input.is_empty() {
            return Ok(DebugAction::Continue);
        }

        let parts: Vec<&str> = input.split_whitespace().collect();

        match parts.get(0).map(|s| *s) {
            Some("q") | Some("quit") => Ok(DebugAction::Quit),
            Some("s") | Some("step") => Ok(DebugAction::Step),
            Some("c") | Some("continue") => Ok(DebugAction::Continue),
            Some("i") | Some("info") => self.parse_info(&parts),
            Some("b") | Some("break") => self.parse_breakpoint(&parts),
            Some("clear") => self.parse_clear(&parts),
            Some("help") | Some("h") => self.show_help(),
            _ => Err(format!("Unknown command: {}", parts[0])),
        }
    }

    fn parse_breakpoint(&mut self, parts: &[&str]) -> Result<DebugAction, String> {
        if parts.len() != 2 {
            return Err("Usage: break <addr>".to_string());
        }

        let addr = parse_addr(parts[1])?;
        self.set_breakpoint(addr);
        Ok(DebugAction::ShowBreakpoints)
    }

    fn parse_clear(&mut self, parts: &[&str]) -> Result<DebugAction, String> {
        if parts.len() != 2 {
            return Err("Usage: clear <addr>".to_string());
        }

        let addr = parse_addr(parts[1])?;
        self.clear_breakpoint(addr);
        Ok(DebugAction::ShowBreakpoints)
    }

    fn parse_info(&self, parts: &[&str]) -> Result<DebugAction, String> {
        if parts.len() < 2 {
            return Err("Usage: info <registers|memory|breakpoints>".to_string());
        }

        match parts[1] {
            "r" | "registers" => Ok(DebugAction::ShowRegisters),
            "m" | "memory" => {
                if parts.len() != 4 {
                    return Err("Usage: info memory <addr> <len>".to_string());
                }
                let addr = parse_addr(parts[2])?;
                let len: usize = parts[3].parse().map_err(|_| "Invalid length".to_string())?;
                Ok(DebugAction::ShowMemory(addr, len))
            }
            "b" | "breakpoints" => Ok(DebugAction::ShowBreakpoints),
            _ => Err("Unknown info command. Try: registers, memory, breakpoints".to_string()),
        }
    }

    fn show_help(&self) -> Result<DebugAction, String> {
        println!("Commands:");
        println!("  break <addr> | b <addr>      - Set breakpoint at address");
        println!("  clear <addr>                 - Clear breakpoint at address");
        println!("  step | s                     - Single step");
        println!("  continue | c                 - Continue execution");
        println!("  info registers | i r         - Show registers");
        println!("  info memory <addr> <len>     - Dump memory");
        println!("  info breakpoints | i b       - List breakpoints");
        println!("  quit | q                     - Quit debugger");
        Ok(DebugAction::Help)
    }

    pub fn show_registers(&self, cpu: &CpuState) {
        println!("PC:    0x{:04X}", cpu.pc);
        println!("I:     0x{:04X}", cpu.i_register);
        println!("SP:    {}", cpu.sp);
        println!("DT:    {}", cpu.delay_timer);
        println!("ST:    {}", cpu.sound_timer);
        println!();
        println!("Registers:");
        for i in 0..16 {
            if i % 8 == 0 {
                print!("  V{:X}: ", i);
            }
            print!("{:02X} ", cpu.registers[i]);
            if i % 8 == 7 {
                println!();
            }
        }
    }

    pub fn show_memory(&self, cpu: &CpuState, addr: u16, len: usize) {
        let start = addr as usize;
        let end = std::cmp::min(start + len, cpu.memory.len());

        for i in (start..end).step_by(16) {
            print!("{:04X}: ", i as u16);
            let row_end = std::cmp::min(i + 16, end);
            for j in i..row_end {
                print!("{:02X} ", cpu.memory[j]);
            }
            println!();
        }
    }

    pub fn show_breakpoints(&self) {
        if self.breakpoints.is_empty() {
            println!("No breakpoints set");
        } else {
            println!("Breakpoints:");
            for bp in &self.breakpoints {
                println!("  0x{:04X}", bp);
            }
        }
    }
}

fn parse_addr(s: &str) -> Result<u16, String> {
    let s = s.trim_start_matches("0x");
    u16::from_str_radix(s, 16).map_err(|_| format!("Invalid address: {}", s))
}
