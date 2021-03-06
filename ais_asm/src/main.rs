mod ais;
mod dynasm;

use crate::ais::{
    AisError, Const, DpCntl, Function, Instruction, Offset, Opcode, Register, Size, SubOpXalu,
};
use crate::dynasm::{DynAsm, DynAsmError, Sym};

use std::fs::File;
use std::io::Write;
use std::process::Command;

#[derive(Debug)]
enum TopError {
    AisError(AisError),
    DynAsmError(DynAsmError),
    IoError(std::io::Error),
}

impl From<AisError> for TopError {
    fn from(x: AisError) -> Self {
        Self::AisError(x)
    }
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
    let eax: Register = "EAX".try_into()?;
    let edx: Register = "EDX".try_into()?;

    // Clear result register
    asm.gen_load(eax, 0x0)?;

    // Define pseudo call and return. Return value is place in a register instead of the stack
    fn pseudo_call(asm: &mut DynAsm, function: Sym) -> Result<(), TopError> {
        // forward declare return label
        let ret = asm.new_sym();
        // Load return register
        asm.gen_load_symbol("EBX".try_into()?, ret)?;
        // Jump to the function
        asm.gen_jump(function)?;
        // Resolve retunr label to be just after the jump
        asm.set_sym_here(ret)?;
        Ok(())
    }

    fn pseudo_ret(asm: &mut DynAsm) -> Result<(), TopError> {
        // Jump to the return register
        asm.gen(Instruction::xj("EBX".try_into()?))?;
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
    asm.gen_load("R4".try_into()?, 4)?;
    asm.gen(Instruction::xalur(
        SubOpXalu::SHL,
        DpCntl::Word,
        eax,
        eax,
        "R4".try_into()?,
    ))?;
    asm.gen(Instruction::xalur(
        SubOpXalu::OR,
        DpCntl::Word,
        eax,
        eax,
        edx,
    ))?;
    pseudo_ret(asm)?;

    // The end is here
    asm.set_sym_here(end)?;

    Ok(())
}

fn tests(asm: &mut DynAsm) -> Result<(), TopError> {
    let eax: Register = "EAX".try_into()?;
    let edx: Register = "EDX".try_into()?;

    // asm.gen_load(eax, 33)?;
    // asm.gen_load(edx, 0x3F8)?;
    // asm.gen(Instruction::xiow(ais::Size::Bits8, edx, eax))?;

    // asm.gen_load(eax, 33)?;
    // asm.gen_load(edx, 0x3F8 + 5)?;
    // asm.gen(Instruction::xior(ais::Size::Bits8, edx, eax))?;

    // push & pop
    // asm.gen_load(edx, 0xF00BAA)?;
    // asm.gen(Instruction::xpush(ais::Size::Bits32, edx))?;
    // asm.gen(Instruction::xpop(ais::Size::Bits32, eax))?;

    // pop ret addr
    //asm.gen(Instruction::xpop(ais::Size::Bits32, eax))?;
    //asm.gen(Instruction::xpush(ais::Size::Bits32, eax))?;

    // push ip
    // asm.gen(Instruction::xpuship(ais::Size::Bits32))?;
    // asm.gen(Instruction::xpop(ais::Size::Bits32, eax))?;

    asm.gen(Instruction::xlead(
        eax,
        0.try_into()?,
        ais::Offset::MDOS,
        ais::AddrSize::Bits32,
        ais::Size::Bits32,
    ))?;

    Ok(())
}

fn dump_cp2_regs(asm: &mut DynAsm) -> Result<(), TopError> {
    let eax: Register = "EAX".try_into()?;
    let edx: Register = "EDX".try_into()?;

    asm.gen_load(edx, 0x50_0000 - 4)?;
    for i in 0..32 {
        asm.gen(Instruction::cfc2(eax, i.try_into()?))?;

        let mut push = Instruction::xpush(Size::Bits32, eax);
        push.rt = Some(edx);
        push.offset = Some(Offset::Number(4));
        asm.gen(push)?;
    }

    Ok(())
}

fn dump_regs(asm: &mut DynAsm) -> Result<(), TopError> {
    let eax: Register = "EAX".try_into()?;
    let edx: Register = "EDX".try_into()?;

    asm.gen_load(edx, 0x50_0000 - 4)?;
    for i in 0..32 {
        let mut push = Instruction::xpush(Size::Bits32, i.try_into()?);
        push.rt = Some(edx);
        push.offset = Some(Offset::Number(4));
        asm.gen(push)?;
    }

    Ok(())
}

fn dump_offset(asm: &mut DynAsm) -> Result<(), TopError> {
    let eax: Register = "EAX".try_into()?;
    let edx: Register = "EDX".try_into()?;
    let r0: Register = "R0".try_into()?;

    asm.gen_load(edx, 0x50_0000 - 4)?;
    for i in 0..32 {
        let mut instr = Instruction::xlead(
            eax,
            r0,
            Offset::Number(0),
            ais::AddrSize::Bits32,
            Size::Bits32,
        );

        instr.mask = i << 21;
        asm.gen(instr)?;

        let mut push = Instruction::xpush(Size::Bits32, eax);
        push.rt = Some(edx);
        push.offset = Some(Offset::Number(4));
        asm.gen(push)?;
    }

    Ok(())
}

fn dump_constant(asm: &mut DynAsm) -> Result<(), TopError> {
    let eax: Register = "EAX".try_into()?;
    let edx: Register = "EDX".try_into()?;
    let r0: Register = "R0".try_into()?;

    asm.gen_load(edx, 0x50_0000 - 4)?;
    for i in 0..32 {
        let mut instr =
            Instruction::xaluir(SubOpXalu::ADD, DpCntl::Word, eax, r0, ais::Const::Raw(i));

        asm.gen(instr)?;

        let mut push = Instruction::xpush(Size::Bits32, eax);
        push.rt = Some(edx);
        push.offset = Some(Offset::Number(4));
        asm.gen(push)?;
    }

    Ok(())
}

fn test_eflags(asm: &mut DynAsm) -> Result<(), TopError> {
    let eax: Register = "EAX".try_into()?;
    let edx: Register = "EDX".try_into()?;
    let r4: Register = "R4".try_into()?;
    let r5: Register = "R5".try_into()?;

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

        let mut push = Instruction::xpush(Size::Bits32, r4);
        push.rt = Some(edx);
        push.offset = Some(Offset::Number(4));
        asm.gen(push)?;

        asm.gen_load(r5, b)?;

        let mut push = Instruction::xpush(Size::Bits32, r5);
        push.rt = Some(edx);
        push.offset = Some(Offset::Number(4));
        asm.gen(push)?;

        asm.gen(Instruction::xalur(
            SubOpXalu::ADD,
            DpCntl::Word,
            eax,
            r4,
            r5,
        ))?;
        asm.gen(Instruction::xalur(
            SubOpXalu::ADD,
            DpCntl::Word,
            eax,
            r4,
            r5,
        ))?;
        asm.gen(Instruction::xalur(
            SubOpXalu::ADD,
            DpCntl::Word,
            eax,
            r4,
            r5,
        ))?;
        asm.gen(Instruction::xalur(
            SubOpXalu::ADD,
            DpCntl::Word,
            eax,
            r4,
            r5,
        ))?;
        asm.gen(Instruction::xalur(
            SubOpXalu::ADD,
            DpCntl::Word,
            eax,
            r4,
            r5,
        ))?;

        asm.gen(Instruction::cfc2(eax, 31.try_into()?))?;

        let mut push = Instruction::xpush(Size::Bits32, eax);
        push.rt = Some(edx);
        push.offset = Some(Offset::Number(4));
        asm.gen(push)?;
    }

    Ok(())
}

fn test_xj(asm: &mut DynAsm) -> Result<(), TopError> {
    let eax: Register = "EAX".try_into()?;
    let edx: Register = "EDX".try_into()?;
    let ecx: Register = "ECX".try_into()?;
    let r0: Register = "R0".try_into()?;
    let r4: Register = "R4".try_into()?;
    let r5: Register = "R5".try_into()?;
    let r6: Register = "R6".try_into()?;
    let r7: Register = "R7".try_into()?;

    asm.gen_load(edx, 0x50_0000 - 4)?;

    let comb = [
        (0, 0),                     // 0x46  // ZF
        (1, 0),                     // 0x02
        (0x8000_0000, 0),           // 0x86   SF
        (0xFFFF_FFFF, 0x8000_0001), // 0x97  SF, CF
        (0xFFFF_FFFF, 1),           // 0x57   ZF, CF
    ];

    //for j in 0..8 {
    for (a, b) in comb {
        asm.gen_load(eax, 0)?;
        // asm.gen_load(r6, 0xFFFF_FFFF)?;
        // asm.gen_load(r7, 1)?;
        asm.gen_load(r6, a)?;
        asm.gen_load(r7, b)?;

        for i in 0..16 {
            asm.gen_load(ecx, 1 << i)?;

            let jmp = asm.new_sym();
            asm.gen_load_symbol(r4, jmp)?;

            asm.gen(Instruction::xalur(SubOpXalu::ADD, DpCntl::Word, r5, r6, r7))?;

            let mut xj = Instruction::xj(r4);
            xj.opcode = Opcode::XJ;
            xj.mask = 6 << 11 | i << 2;
            asm.gen(xj)?;
            asm.gen(Instruction::xalur(
                SubOpXalu::OR,
                DpCntl::Word,
                eax,
                eax,
                ecx,
            ))?;
            asm.set_sym_here(jmp)?;
        }

        let mut push = Instruction::xpush(Size::Bits32, eax);
        push.rt = Some(edx);
        push.offset = Some(Offset::Number(4));
        asm.gen(push)?;

        // asm.gen(Instruction::xalur(SubOpXalu::ADD, DpCntl::Word, r5, r6, r7))?;
        // asm.gen(Instruction::xalur(SubOpXalu::ADD, DpCntl::Word, r5, r6, r7))?;
        // asm.gen(Instruction::xalur(SubOpXalu::ADD, DpCntl::Word, r5, r6, r7))?;
        // asm.gen(Instruction::xalur(SubOpXalu::ADD, DpCntl::Word, r5, r6, r7))?;
        // asm.gen(Instruction::xalur(SubOpXalu::ADD, DpCntl::Word, r5, r6, r7))?;

        // asm.gen(Instruction::cfc2(eax, 31.try_into()?))?;

        // let mut push = Instruction::xpush(Size::Bits32, eax);
        // push.rt = Some(edx);
        // push.offset = Some(Offset::Number(4));
        // asm.gen(push)?;
    }

    Ok(())
}

fn test_cond_jump(asm: &mut DynAsm) -> Result<(), TopError> {
    let eax: Register = "EAX".try_into()?;
    let ecx: Register = "ECX".try_into()?;

    asm.gen_load(eax, 0)?;
    asm.gen_load(ecx, 0b111111)?;

    let done = asm.new_sym();
    let body = asm.new_sym();
    let looop = asm.new_sym_here();

    asm.gen_cond_jump(ecx, body, done)?;
    asm.set_sym_here(body)?;

    asm.gen(Instruction::xaluir(
        SubOpXalu::ADD,
        DpCntl::Word,
        eax,
        eax,
        Const::Number(1),
    ))?;

    asm.gen(Instruction::xaluir(
        SubOpXalu::SHR,
        DpCntl::Word,
        ecx,
        ecx,
        Const::Number(1),
    ))?;

    asm.gen_jump(looop)?;
    asm.set_sym_here(done)?;

    Ok(())
}

fn test_call_ret(asm: &mut DynAsm) -> Result<(), TopError> {
    let eax: Register = "EAX".try_into()?;
    let ecx: Register = "ECX".try_into()?;

    let start = asm.new_sym();
    asm.gen_jump(start)?;

    let add = asm.new_sym_here();
    asm.gen(Instruction::xalur(
        SubOpXalu::ADD,
        DpCntl::Word,
        eax,
        eax,
        ecx,
    ))?;
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
    let eax: Register = "EAX".try_into()?;
    let ecx: Register = "ECX".try_into()?;
    let edx: Register = "EDX".try_into()?;

    let start = asm.new_sym();
    asm.gen_jump(start)?;

    let putc = asm.new_sym_here();

    asm.gen_load(edx, 0x3F8 + 5)?;
    asm.gen(Instruction::xior(ais::Size::Bits8L, edx, eax))?;

    asm.gen(Instruction::xaluir(
        SubOpXalu::SHR,
        DpCntl::Word,
        eax,
        eax,
        Const::Number(5),
    ))?;

    let ready = asm.new_sym();
    asm.gen_cond_jump(eax, ready, putc)?;
    asm.set_sym_here(ready)?;

    asm.gen_load(edx, 0x3F8)?;
    asm.gen(Instruction::xiow(ais::Size::Bits8L, edx, ecx))?;

    asm.gen_ret()?;

    asm.set_sym_here(start)?;

    for b in "Hello World!\n".as_bytes() {
        asm.gen_load(ecx, (*b).into())?;
        asm.gen_call(putc)?;
    }

    Ok(())
}

fn main() -> Result<(), TopError> {
    let entry = Instruction::decode(&[0x62, 0x80, 0x19, 0x08, 0xE0, 0x83]);
    let exit = Instruction::decode(&[0x62, 0x80, 0x47, 0x00, 0x10, 0x18]);
    println!("entry: {:?}", entry);
    println!("exit:  {:?}", exit);

    let eflags_load = Instruction::decode(&[0x62, 0x80, 0xC0, 0xFF, 0x07, 0xA0]);
    let eflags_store = Instruction::decode(&[0x62, 0x80, 0x19, 0xF8, 0xE0, 0x80]);
    println!("eflags_load:  {:?}", eflags_load);
    println!("eflags_store: {:?}", eflags_store);

    // Gen some code, at location 0x480000, this is where our kernel will place the payload
    let mut asm = DynAsm::new(0x48_0000);

    // Add x86 to AIS transition header
    asm.gen_header();

    //test_eflags(&mut asm)?;
    //test_xj(&mut asm)?;
    //dump_regs(&mut asm)?;
    //dump_offset(&mut asm)?;
    //dump_constant(&mut asm)?;
    //test_cond_jump(&mut asm)?;
    //test_call_ret(&mut asm)?;
    test_hello_world(&mut asm)?;

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
