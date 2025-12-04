use crate::conf::{
    FLAG_COUNT, HI_RES_HEIGHT, HI_RES_WIDTH, KEYS_COUNT, RAM_SIZE, REGISTER_COUNT, STACK_SIZE,
};
use anyhow::Result;

pub struct VmContext<'a> {
    pub pc: &'a mut u16,
    pub registers: &'a mut [u8; REGISTER_COUNT],
    pub i_register: &'a mut u16,
    pub stack: &'a mut [u16; STACK_SIZE],
    pub sp: &'a mut u16,
    pub memory: &'a mut [u8; RAM_SIZE],

    pub screen: &'a mut [bool; HI_RES_HEIGHT * HI_RES_WIDTH],
    pub keys: &'a [bool; KEYS_COUNT],
    pub delay_timer: &'a mut u8,
    pub sound_timer: &'a mut u8,

    pub current_width: &'a mut usize,
    pub current_height: &'a mut usize,
    // S-CHIP specific
    pub rpl_flags: &'a mut [u8; FLAG_COUNT],
}

pub trait Extension {
    /// Returns the name of the extension(e.g., "Super-CHIP").
    fn name(&self) -> &'static str;

    /// Checks if the extension is currently enabled.
    fn is_active(&self) -> bool;

    /// Attemps to execute an instruction.
    /// Returns `Ok(true)` if the opcode was handled and the execution should stop.
    /// Returns `Ok(false)` if the opcode was not handled(falls through to the base CHIP 8 or next
    /// extension)
    fn handle_instruction(&mut self, ctx: &mut VmContext, opcode: u16) -> Result<bool>;

    /// Hook for initialization, called once after the VM creation
    fn initialize(&mut self, ctx: &mut VmContext);
}
