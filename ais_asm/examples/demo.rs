extern crate ais_asm;

use ais_asm::ais::Register;
use ais_asm::asm;
use ais_asm::dynasm::{DynAsm, DynAsmError, Sym};

use std::fs::File;
use std::io::Write;
use std::process::Command;

#[derive(Debug)]
enum TopError {
    DynAsmError(DynAsmError),
    IoError(std::io::Error),
}

impl From<DynAsmError> for TopError {
    fn from(x: DynAsmError) -> Self {
        Self::DynAsmError(x)
    }
}

impl From<std::io::Error> for TopError {
    fn from(x: std::io::Error) -> Self {
        Self::IoError(x)
    }
}

fn demo(asm: &mut DynAsm) -> Result<(), TopError> {
    let eax: Register = Register::EAX;
    let edx: Register = Register::EDX;

    // Clear result register
    asm.gen_load(eax, 0x0)?;

    // Define pseudo call and return. Return value is place in a register instead of the stack
    fn pseudo_call(asm: &mut DynAsm, function: Sym) -> Result<(), TopError> {
        // forward declare return label
        let ret = asm.new_sym();
        // Load return register
        asm.gen_load_symbol(Register::EBX, ret)?;
        // Jump to the function
        asm.gen_jump(function)?;
        // Resolve retunr label to be just after the jump
        asm.set_sym_here(ret)?;
        Ok(())
    }

    fn pseudo_ret(asm: &mut DynAsm) -> Result<(), TopError> {
        // Jump to the return register
        asm.gen(asm::j(Register::EBX))?;
        Ok(())
    }

    // Forward declare push function
    let push = asm.new_sym();

    // Push some bytes
    asm.gen_load(edx, 0xB)?;
    pseudo_call(asm, push)?;
    asm.gen_load(edx, 0xA)?;
    pseudo_call(asm, push)?;
    asm.gen_load(edx, 0xD)?;
    pseudo_call(asm, push)?;
    asm.gen_load(edx, 0xC)?;
    pseudo_call(asm, push)?;
    asm.gen_load(edx, 0x0)?;
    pseudo_call(asm, push)?;
    asm.gen_load(edx, 0xD)?;
    pseudo_call(asm, push)?;
    asm.gen_load(edx, 0xE)?;
    pseudo_call(asm, push)?;

    // Done jump to the end
    let end = asm.new_sym();
    asm.gen_jump(end)?;

    // Function that will push a byte in the result
    // EAX = EAX << 4 | EDX
    asm.set_sym_here(push)?;
    asm.gen_load(Register::R4, 4)?;
    asm.gen(asm::shl(eax, eax, Register::R4))?;
    asm.gen(asm::or(eax, eax, edx))?;
    pseudo_ret(asm)?;

    // The end is here
    asm.set_sym_here(end)?;

    Ok(())
}

fn main() -> Result<(), TopError> {
    // Gen some code, at location 0x480000, this is where our kernel will place the payload
    let mut asm = DynAsm::new(0x48_0000);

    // Add x86 to AIS transition header
    asm.gen_header();

    // Demo
    demo(&mut asm)?;

    // Append footer and we are done. This is just a return, so it will return from the payload back into the kernel
    asm.gen_footer();

    // Show dynamic assembled instructions
    asm.dump();

    // Write payload to out.bin, the kernel will included this as the payload
    let mut output = File::create("out.bin")?;
    output.by_ref().write_all(asm.memory())?;
    output.flush()?;

    // Show generated disassembly in regular x86 instructions.
    let output = Command::new("objdump")
        .args(["-D", "-bbinary", "-mi386", "-Mintel", "out.bin"])
        .output()?;
    println!("{}", std::str::from_utf8(&output.stdout).unwrap());

    Ok(())
}
