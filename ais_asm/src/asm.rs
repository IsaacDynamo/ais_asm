use crate::ais::{
    AddrSize, Const, DpCntl, Function, Instruction, Offset, Opcode, Register, Sel, Size, SubFunc,
    SubOp, SubOpXalu, SubOpXio, XjMode, XjSize,
};

fn i_type(opcode: Opcode, dst: Register, src: Register, imm: u16) -> Instruction {
    let mut ret = Instruction::new(opcode);
    ret.rs = Some(src);
    ret.rt = Some(dst);
    ret.imm = Some(imm);
    ret
}

fn xalur(
    subop: SubOpXalu,
    dpcntl: DpCntl,
    dst: Register,
    src: Register,
    extra: Register,
) -> Instruction {
    let mut ret = Instruction::new(Opcode::XALUR);
    ret.rs = Some(src);
    ret.rd = Some(dst);
    ret.rt = Some(extra);
    ret.function = Some(Function::Xalu(subop, dpcntl));
    ret
}

fn xaluir(
    subop: SubOpXalu,
    dpcntl: DpCntl,
    dst: Register,
    src: Register,
    constant: Const,
) -> Instruction {
    let mut ret = Instruction::new(Opcode::XALUIR);
    ret.rs = Some(src);
    ret.rd = Some(dst);
    ret.constant = Some(constant);
    ret.function = Some(Function::Xalu(subop, dpcntl));
    ret
}

fn xls_type(opcode: Opcode, rs: Register, base: Register, offset: Offset) -> Instruction {
    let mut ret = Instruction::new(opcode);
    ret.rs = Some(rs);
    ret.rt = Some(base);
    ret.offset = Some(offset);
    ret
}

pub fn iow(size: Size, port: Register, value: Register) -> Instruction {
    let mut instr = xls_type(Opcode::XIOW, value, port, Offset::Number(0));
    instr.function = Some(Function::Xio(
        SubOpXio::Norm,
        AddrSize::Bits16,
        size,
        Sel::FLAT,
    ));
    instr
}

pub fn ior(size: Size, port: Register, value: Register) -> Instruction {
    let mut instr = xls_type(Opcode::XIOR, value, port, Offset::Number(0));
    instr.function = Some(Function::Xio(
        SubOpXio::Norm,
        AddrSize::Bits16,
        size,
        Sel::FLAT,
    ));
    instr
}

pub fn push(size: Size, reg: Register, base: Register, offset: Offset) -> Instruction {
    let mut instr = xls_type(Opcode::XPUSH, reg, base, offset);
    instr.function = Some(Function::Xls(
        SubOp::Raw(0),
        AddrSize::Bits32,
        size,
        Sel::SS,
    ));
    instr
}

pub fn pushsp(size: Size, reg: Register) -> Instruction {
    push(size, reg, Register::ESP, Offset::Number(-4))
}

pub fn pop(size: Size, reg: Register, base: Register, offset: Offset) -> Instruction {
    let mut instr = xls_type(Opcode::XPOP, reg, base, offset);
    instr.function = Some(Function::Xls(
        SubOp::Raw(0),
        AddrSize::Bits32,
        size,
        Sel::SS,
    ));
    instr
}

pub fn popsp(size: Size, reg: Register) -> Instruction {
    pop(size, reg, Register::ESP, Offset::Number(4))
}

pub fn puship(size: Size) -> Instruction {
    let mut instr = xls_type(
        Opcode::XPUSHIP,
        Register::R0,
        Register::ESP,
        Offset::Number(-4),
    );
    instr.function = Some(Function::Xls(
        SubOp::Raw(0),
        AddrSize::Bits32,
        size,
        Sel::SS,
    ));
    instr
}

pub fn lead(
    dst: Register,
    base: Register,
    offset: Offset,
    addr_size: AddrSize,
    size: Size,
) -> Instruction {
    let mut instr = xls_type(Opcode::XLEAD, dst, base, offset);
    instr.function = Some(Function::Xlea(addr_size, size));
    instr
}

pub fn leai(
    dst: Register,
    base: Register,
    index: Register,
    addr_size: AddrSize,
    size: Size,
) -> Instruction {
    let mut instr = Instruction::new(Opcode::XLEAI);
    instr.rs = Some(dst);
    instr.rt = Some(base);
    instr.rd = Some(index);
    instr.function = Some(Function::Xlea(addr_size, size));
    instr
}

pub fn cfc2(dst: Register, src: Register) -> Instruction {
    let mut instr = Instruction::new(Opcode::XMISC);
    instr.rt = Some(dst);
    instr.rd = Some(src);
    instr.function = Some(Function::Xmisc(SubFunc::CFC2, 0));
    instr
}

pub fn j(base: Register) -> Instruction {
    let mut ret = Instruction::new(Opcode::XJ);
    ret.rt = Some(base);
    ret.function = Some(Function::Xj(XjSize::Bits32, XjMode::AIS));
    ret.leftovers = 0b0001 << 2; // Use leftovers to set undocumented bit field
    ret
}

pub fn xandil(dst: Register, src: Register, imm: u16) -> Instruction {
    i_type(Opcode::ANDIL, dst, src, imm)
}

pub fn xandiu(dst: Register, src: Register, imm: u16) -> Instruction {
    i_type(Opcode::ANDIU, dst, src, imm)
}

pub fn xori(dst: Register, src: Register, imm: u16) -> Instruction {
    i_type(Opcode::ORI, dst, src, imm)
}

pub fn xoriu(dst: Register, src: Register, imm: u16) -> Instruction {
    i_type(Opcode::ORIU, dst, src, imm)
}

pub fn and(dst: Register, src: Register, extra: Register) -> Instruction {
    xalur(SubOpXalu::AND, DpCntl::Word, dst, src, extra)
}

pub fn andi(dst: Register, src: Register, constant: Const) -> Instruction {
    xaluir(SubOpXalu::AND, DpCntl::Word, dst, src, constant)
}

pub fn or(dst: Register, src: Register, extra: Register) -> Instruction {
    xalur(SubOpXalu::OR, DpCntl::Word, dst, src, extra)
}

pub fn ori(dst: Register, src: Register, constant: Const) -> Instruction {
    xaluir(SubOpXalu::OR, DpCntl::Word, dst, src, constant)
}

pub fn sub(dst: Register, src: Register, extra: Register) -> Instruction {
    xalur(SubOpXalu::SUB, DpCntl::Word, dst, src, extra)
}

pub fn subi(dst: Register, src: Register, constant: Const) -> Instruction {
    xaluir(SubOpXalu::SUB, DpCntl::Word, dst, src, constant)
}

pub fn add(dst: Register, src: Register, extra: Register) -> Instruction {
    xalur(SubOpXalu::ADD, DpCntl::Word, dst, src, extra)
}

pub fn addi(dst: Register, src: Register, constant: Const) -> Instruction {
    xaluir(SubOpXalu::ADD, DpCntl::Word, dst, src, constant)
}

pub fn shl(dst: Register, src: Register, extra: Register) -> Instruction {
    xalur(SubOpXalu::SHL, DpCntl::Word, dst, src, extra)
}

pub fn shli(dst: Register, src: Register, constant: Const) -> Instruction {
    xaluir(SubOpXalu::SHL, DpCntl::Word, dst, src, constant)
}

pub fn shr(dst: Register, src: Register, extra: Register) -> Instruction {
    xalur(SubOpXalu::SHR, DpCntl::Word, dst, src, extra)
}

pub fn shri(dst: Register, src: Register, constant: Const) -> Instruction {
    xaluir(SubOpXalu::SHR, DpCntl::Word, dst, src, constant)
}
