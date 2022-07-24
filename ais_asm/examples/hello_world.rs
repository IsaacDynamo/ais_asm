extern crate ais_asm;

use ais_asm::ais::{Const, Register, Size};
use ais_asm::asm;
use ais_asm::dynasm::{DynAsm, DynAsmError};

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

fn hello_world(asm: &mut DynAsm) -> Result<(), TopError> {
    let eax: Register = Register::EAX;
    let ecx: Register = Register::ECX;
    let edx: Register = Register::EDX;

    let start = asm.new_sym();
    asm.gen_jump(start)?;

    let putc = asm.new_sym_here();

    asm.gen_load(edx, 0x3F8 + 5)?;
    asm.gen(asm::ior(Size::Bits8L, edx, eax))?;

    asm.gen(asm::shri(eax, eax, Const::Number(5)))?;

    let ready = asm.new_sym();
    asm.gen_cond_jump(eax, ready, putc)?;
    asm.set_sym_here(ready)?;

    asm.gen_load(edx, 0x3F8)?;
    asm.gen(asm::iow(Size::Bits8L, edx, ecx))?;

    asm.gen_ret()?;

    asm.set_sym_here(start)?;

    for b in "Hello World!\n".as_bytes() {
        asm.gen_load(ecx, (*b).into())?;
        asm.gen_call(putc)?;
    }

    Ok(())
}

fn main() -> Result<(), TopError> {
    // Gen some code, at location 0x480000, this is where our kernel will place the payload
    let mut asm = DynAsm::new(0x48_0000);

    // Add x86 to AIS transition header
    asm.gen_header();

    // Hello world example
    hello_world(&mut asm)?;

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
