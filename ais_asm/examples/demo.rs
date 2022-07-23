
extern crate ais_asm;

use ais_asm::ais::{
    Const, Offset, Opcode, Register, Size, AddrSize
};
use ais_asm::dynasm::{DynAsm, DynAsmError, Sym};
use ais_asm::asm;
use ais_asm::decode::decode;

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

fn tests(asm: &mut DynAsm) -> Result<(), TopError> {
    let eax: Register = Register::EAX;

    // asm.gen_load(eax, 33)?;
    // asm.gen_load(edx, 0x3F8)?;
    // asm.gen(asm::xiow(ais::Size::Bits8, edx, eax))?;

    // asm.gen_load(eax, 33)?;
    // asm.gen_load(edx, 0x3F8 + 5)?;
    // asm.gen(asm::xior(ais::Size::Bits8, edx, eax))?;

    // push & pop
    // asm.gen_load(edx, 0xF00BAA)?;
    // asm.gen(asm::xpush(ais::Size::Bits32, edx))?;
    // asm.gen(asm::xpop(ais::Size::Bits32, eax))?;

    // pop ret addr
    //asm.gen(asm::xpop(ais::Size::Bits32, eax))?;
    //asm.gen(asm::xpush(ais::Size::Bits32, eax))?;

    // push ip
    // asm.gen(asm::xpuship(ais::Size::Bits32))?;
    // asm.gen(asm::xpop(ais::Size::Bits32, eax))?;

    asm.gen(asm::lead(
        eax,
        Register::R0,
        Offset::MDOS,
        AddrSize::Bits32,
        Size::Bits32,
    ))?;

    Ok(())
}

fn dump_cp2_regs(asm: &mut DynAsm) -> Result<(), TopError> {
    let eax: Register = Register::EAX;
    let edx: Register = Register::EDX;

    asm.gen_load(edx, 0x50_0000 - 4)?;
    for i in 0..32 {
        asm.gen(asm::cfc2(eax, Register(i)))?;

        asm.gen(asm::push(Size::Bits32, eax, edx, Offset::Number(4)))?;
    }

    Ok(())
}

fn dump_regs(asm: &mut DynAsm) -> Result<(), TopError> {
    let edx: Register = Register::EDX;

    asm.gen_load(edx, 0x50_0000 - 4)?;
    for i in 0..32 {
        asm.gen(asm::push(Size::Bits32, Register(i), edx, Offset::Number(4)))?;
    }

    Ok(())
}

fn dump_offset(asm: &mut DynAsm) -> Result<(), TopError> {
    let eax: Register = Register::EAX;
    let edx: Register = Register::EDX;
    let r0: Register = Register::R0;

    asm.gen_load(edx, 0x50_0000 - 4)?;
    for i in 0..32 {
        let mut instr = asm::lead(
            eax,
            r0,
            Offset::Number(0),
            AddrSize::Bits32,
            Size::Bits32,
        );

        instr.leftovers = i << 21;
        asm.gen(instr)?;
        asm.gen(asm::push(Size::Bits32, eax, edx, Offset::Number(4)))?;
    }

    Ok(())
}

fn dump_constant(asm: &mut DynAsm) -> Result<(), TopError> {
    let eax: Register = Register::EAX;
    let edx: Register = Register::EDX;
    let r0: Register = Register::R0;

    asm.gen_load(edx, 0x50_0000 - 4)?;
    for i in 0..32 {
        asm.gen(asm::addi(eax, r0, Const::Raw(i)))?;
        asm.gen(asm::push(Size::Bits32, eax, edx, Offset::Number(4)))?;
    }

    Ok(())
}

fn test_eflags(asm: &mut DynAsm) -> Result<(), TopError> {
    let eax: Register = Register::EAX;
    let edx: Register = Register::EDX;
    let r4: Register = Register::R4;
    let r5: Register = Register::R5;

    asm.gen_load(edx, 0x50_0000 - 4)?;

    let comb = [
        (0, 0),           // 0x46  // ZF, PF
        (1, 0),           // 0x02
        (0x8000_0000, 0), // 0x86   SF, PF
        (0xFFFF_FFFF, 0), // 0x86   SF, PF
        (0xFFFF_FFFF, 1), // 0x57   ZF, AF, PF, CF
    ];

    for (a, b) in comb {
        asm.gen_load(r4, a)?;
        asm.gen(asm::push(Size::Bits32, r4, edx, Offset::Number(4)))?;

        asm.gen_load(r5, b)?;
        asm.gen(asm::push(Size::Bits32, r5, edx, Offset::Number(4)))?;

        asm.gen(asm::add(eax, r4, r5))?;
        asm.gen(asm::add(eax, r4, r5))?;
        asm.gen(asm::add(eax, r4, r5))?;
        asm.gen(asm::add(eax, r4, r5))?;
        asm.gen(asm::add(eax, r4, r5))?;

        asm.gen(asm::cfc2(eax, Register(31)))?;
        asm.gen(asm::push(Size::Bits32, eax, edx, Offset::Number(4)))?;
    }

    Ok(())
}

fn test_xj(asm: &mut DynAsm) -> Result<(), TopError> {
    let eax: Register = Register::EAX;
    let edx: Register = Register::EDX;
    let ecx: Register = Register::ECX;
    let r4: Register = Register::R4;
    let r5: Register = Register::R5;
    let r6: Register = Register::R6;
    let r7: Register = Register::R7;

    asm.gen_load(edx, 0x50_0000 - 4)?;

    let comb = [
        (0, 0),                     // 0x46 ZF
        (1, 0),                     // 0x02
        (0x8000_0000, 0),           // 0x86 SF
        (0xFFFF_FFFF, 0x8000_0001), // 0x97 SF, CF
        (0xFFFF_FFFF, 1),           // 0x57 ZF, CF
    ];

    for (a, b) in comb {
        asm.gen_load(eax, 0)?;

        asm.gen_load(r6, a)?;
        asm.gen_load(r7, b)?;

        for i in 0..16 {
            asm.gen_load(ecx, 1 << i)?;

            let jmp = asm.new_sym();
            asm.gen_load_symbol(r4, jmp)?;

            asm.gen(asm::add(r5, r6, r7))?;

            let mut xj = asm::j(r4);
            xj.opcode = Opcode::XJ;
            xj.leftovers = 6 << 11 | i << 2;
            asm.gen(xj)?;
            asm.gen(asm::or(eax, eax, ecx))?;
            asm.set_sym_here(jmp)?;
        }

        asm.gen(asm::push(Size::Bits32, eax, edx, Offset::Number(4)))?;
    }

    Ok(())
}

fn test_cond_jump(asm: &mut DynAsm) -> Result<(), TopError> {
    let eax: Register = Register::EAX;
    let ecx: Register = Register::ECX;

    asm.gen_load(eax, 0)?;
    asm.gen_load(ecx, 0b111111)?;

    let done = asm.new_sym();
    let body = asm.new_sym();
    let looop = asm.new_sym_here();

    asm.gen_cond_jump(ecx, body, done)?;
    asm.set_sym_here(body)?;

    asm.gen(asm::addi(eax, eax, Const::Number(1)))?;

    asm.gen(asm::shri(ecx, ecx, Const::Number(1)))?;

    asm.gen_jump(looop)?;
    asm.set_sym_here(done)?;

    Ok(())
}

fn test_call_ret(asm: &mut DynAsm) -> Result<(), TopError> {
    let eax: Register = Register::EAX;
    let ecx: Register = Register::ECX;

    let start = asm.new_sym();
    asm.gen_jump(start)?;

    let add = asm.new_sym_here();
    asm.gen(asm::add(eax, eax, ecx))?;
    asm.gen_ret()?;

    let inc = asm.new_sym_here();
    asm.gen_load(ecx, 1)?;
    asm.gen_call(add)?;
    asm.gen_ret()?;

    asm.set_sym_here(start)?;
    asm.gen_load(eax, 0)?;
    asm.gen_load(ecx, 41)?;
    asm.gen_call(add)?;
    asm.gen_call(inc)?;

    Ok(())
}

fn test_hello_world(asm: &mut DynAsm) -> Result<(), TopError> {
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

fn test_timestamp(asm: &mut DynAsm) -> Result<(), TopError> {
    let eax: Register = Register::EAX;
    let ecx: Register = Register::ECX;
    let edx: Register = Register::EDX;

    asm.gen_load(edx, 0x50_0000 - 4)?;
    for _ in 0..16 {
        asm.gen(asm::cfc2(eax, Register(19)))?;
        asm.gen(asm::cfc2(ecx, Register(19)))?;
        asm.gen(asm::push(Size::Bits32, eax, edx, Offset::Number(4)))?;
        asm.gen(asm::push(Size::Bits32, ecx, edx, Offset::Number(4)))?;
    }

    Ok(())
}

fn dump_manual_constants() {
    let entry = decode(&[0x62, 0x80, 0x19, 0x08, 0xE0, 0x83]);
    let exit = decode(&[0x62, 0x80, 0x47, 0x00, 0x10, 0x18]);
    println!("entry: {:?}", entry);
    println!("exit:  {:?}", exit);

    let eflags_load = decode(&[0x62, 0x80, 0xC0, 0xFF, 0x07, 0xA0]);
    let eflags_store = decode(&[0x62, 0x80, 0x19, 0xF8, 0xE0, 0x80]);
    println!("eflags_load:  {:?}", eflags_load);
    println!("eflags_store: {:?}", eflags_store);
}

fn main() -> Result<(), TopError> {

    dump_manual_constants();

    // Gen some code, at location 0x480000, this is where our kernel will place the payload
    let mut asm = DynAsm::new(0x48_0000);

    // Add x86 to AIS transition header
    asm.gen_header();

    demo(&mut asm)?;
    tests(&mut asm)?;
    dump_cp2_regs(&mut asm)?;
    test_eflags(&mut asm)?;
    test_xj(&mut asm)?;
    dump_regs(&mut asm)?;
    dump_offset(&mut asm)?;
    dump_constant(&mut asm)?;
    test_cond_jump(&mut asm)?;
    test_call_ret(&mut asm)?;
    test_hello_world(&mut asm)?;
    test_timestamp(&mut asm)?;

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
