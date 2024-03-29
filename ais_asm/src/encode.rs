use crate::ais::{AisError, Field, Function, Instruction, Opcode, Register};
use crate::{bit, bits};

fn encode_opcode(instr: &Instruction) -> Result<u32, AisError> {
    Ok((instr.opcode as u32) << 26)
}

fn encode_rs(instr: &Instruction) -> Result<u32, AisError> {
    encode_register(&instr.rs, AisError::Missing(Field::RS)).map(|x| x << 21)
}

fn encode_rt(instr: &Instruction) -> Result<u32, AisError> {
    encode_register(&instr.rt, AisError::Missing(Field::RT)).map(|x| x << 16)
}

fn encode_rd(instr: &Instruction) -> Result<u32, AisError> {
    encode_register(&instr.rd, AisError::Missing(Field::RD)).map(|x| x << 11)
}

fn encode_register(register: &Option<Register>, err: AisError) -> Result<u32, AisError> {
    register.as_ref().ok_or(err).map(|x| x.0.into())
}

fn encode_imm(instr: &Instruction) -> Result<u32, AisError> {
    instr
        .imm
        .ok_or(AisError::Missing(Field::Immediate))
        .map(|x| x.into())
}

fn encode_const(instr: &Instruction) -> Result<u32, AisError> {
    let c = instr.constant.ok_or(AisError::Missing(Field::Const))?;

    let x: u8 = c
        .try_into()
        .ok()
        .ok_or(AisError::Unsupported(Field::Const))?;
    let x: u32 = x.into();

    Ok(x << 16)
}

fn encode_offset(instr: &Instruction) -> Result<u32, AisError> {
    let offset = instr.offset.ok_or(AisError::Missing(Field::Offset))?;

    let x: u8 = offset
        .try_into()
        .ok()
        .ok_or(AisError::Unsupported(Field::Offset))?;
    let x: u32 = x.into();

    Ok(x << 21)
}

fn encode_function(instr: &Instruction) -> Result<u32, AisError> {
    let function = instr.function.ok_or(AisError::Missing(Field::Function))?;

    let bits = match function {
        Function::Xalu(sub_op, dp_cntl) => (sub_op as u32) | (dp_cntl as u32) << 5,
        Function::Xio(sub_op, addr_size, size, sel) => {
            let subop_bits = (sub_op as u32) << 9;
            subop_bits
                | (addr_size as u32 & 2) << 7
                | ((size as u32) & 0x6) << 5
                | (sel as u32) << 2
                | (size as u32 & 1) << 1
                | addr_size as u32 & 1
        }
        Function::Xj(size, mode) => (size as u32) << 6 | mode as u32,
        Function::Xls(sub_op, addr_size, size, sel) => {
            let subop_bits: u8 = sub_op
                .try_into()
                .ok()
                .ok_or(AisError::Unsupported(Field::Function))?;
            let subop_bits = (subop_bits as u32) << 9;
            subop_bits
                | (addr_size as u32 & 2) << 7
                | ((size as u32) & 0x6) << 5
                | (sel as u32) << 2
                | (size as u32 & 1) << 1
                | addr_size as u32 & 1
        }
        Function::Xlea(addr_size, size) => {
            let addr_size = addr_size as u32;
            let size = size as u32;

            bit(addr_size, 1) << 8
                | bits(size, 2, 1) << 6
                | bit(size, 3) << 2
                | bit(size, 0) << 1
                | bit(addr_size, 0)
        }
        Function::Raw(x) => x.into(),
        Function::Xmisc(sub_func, raw) => {
            let subfunc_bits = sub_func as u32;

            subfunc_bits << 6 | raw as u32
        }
    };

    Ok(bits)
}

pub fn encode32(instr: &Instruction) -> Result<u32, AisError> {
    let word = if instr.is_i_type() {
        let op = encode_opcode(instr)?;
        let rs = encode_rs(instr)?;
        let rt = encode_rt(instr)?;
        let imm = encode_imm(instr)?;

        op | rs | rt | imm
    } else if instr.is_xalu_type() {
        let op = encode_opcode(instr)?;
        let rs = encode_rs(instr)?;
        let rt = encode_rt(instr)?;
        let rd = encode_rd(instr)?;
        let function = encode_function(instr)?;

        op | rs | rt | rd | function
    } else if instr.is_xalui_type() {
        let op = encode_opcode(instr)?;
        let rs = encode_rs(instr)?;
        let c = encode_const(instr)?;
        let rd = encode_rd(instr)?;
        let function = encode_function(instr)?;

        op | rs | c | rd | function
    } else if instr.opcode == Opcode::XJ {
        let op = encode_opcode(instr)?;
        let rt = encode_rt(instr)?;
        let function = encode_function(instr)?;

        op | rt | function
    } else if instr.opcode == Opcode::XMISC {
        let op = encode_opcode(instr)?;
        let rt = encode_rt(instr)?;
        let rd = encode_rd(instr)?;
        let function = encode_function(instr)?;

        op | rt | rd | function
    } else if instr.is_xls_type() || instr.opcode == Opcode::XLEAD {
        let op = encode_opcode(instr)?;
        let offset = encode_offset(instr)?;
        let base = encode_rt(instr)?;
        let rs = encode_register(&instr.rs, AisError::Missing(Field::RS))? << 11;
        let function = encode_function(instr)?;

        op | offset | base | rs | function
    } else {
        return Err(AisError::Unsupported(Field::Opcode));
    };

    Ok(word | instr.leftovers)
}

pub fn encode(instr: &Instruction) -> Result<Vec<u8>, AisError> {
    let word = encode32(instr)?;

    let mut data = Vec::new();
    data.extend_from_slice(&[0x62, 0x80]);
    data.extend_from_slice(&word.to_le_bytes());
    Ok(data)
}
