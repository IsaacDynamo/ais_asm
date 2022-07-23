use num::FromPrimitive;
use num_derive::FromPrimitive;
use std::{convert::TryFrom, ops::Range};

use crate::ais::{
    AisError, Const, DpCntl, Field, Function, Instruction, Offset, Opcode, Register, Size, SubOp,
    SubOpXalu,
};

fn bit(word: u32, bit: u32) -> u32 {
    (word >> bit) & 1
}

fn bits(word: u32, high: u32, low: u32) -> u32 {
    let mask = (1 << (high - low + 1)) - 1;
    (word >> low) & mask
}

fn decode_xalu_function(word: u32) -> Result<Function, AisError> {
    let sub_op_bits = bits(word, 4, 0);
    let dp_cntl_bits = bits(word, 7, 5);
    let sub_op = FromPrimitive::from_u32(sub_op_bits).ok_or(AisError::DecodeIssue)?;
    let dp_cntl = FromPrimitive::from_u32(dp_cntl_bits).ok_or(AisError::DecodeIssue)?;
    Ok(Function::Xalu(sub_op, dp_cntl))
}

fn decode_xio_function(word: u32) -> Result<Function, AisError> {
    let sub_op_bits = bits(word, 10, 9);
    let addr_size_bits = bit(word, 8) << 1 | bit(word, 0);
    let size_bits = bits(word, 7, 6) << 1 | bit(word, 1);
    let sel_bits = bits(word, 5, 2);

    let sub_op = FromPrimitive::from_u32(sub_op_bits).ok_or(AisError::DecodeIssue)?;
    let addr_size = FromPrimitive::from_u32(addr_size_bits).ok_or(AisError::DecodeIssue)?;
    let size = FromPrimitive::from_u32(size_bits).ok_or(AisError::DecodeIssue)?;
    let sel = FromPrimitive::from_u32(sel_bits).ok_or(AisError::DecodeIssue)?;

    Ok(Function::Xio(sub_op, addr_size, size, sel))
}

fn decode_xj_function(word: u32) -> Result<Function, AisError> {
    let size = FromPrimitive::from_u32(bits(word, 7, 6)).ok_or(AisError::DecodeIssue)?;
    let mode = FromPrimitive::from_u32(bits(word, 1, 0)).ok_or(AisError::DecodeIssue)?;
    Ok(Function::Xj(size, mode))
}

fn decode_function(opcode: Opcode, word: u32) -> Result<Function, AisError> {
    match opcode {
        Opcode::XIOR | Opcode::XIOW => decode_xio_function(word),
        Opcode::XLEAD | Opcode::XLEAI => {
            let addr_size_bits = bit(word, 8) << 1 | bit(word, 0);
            let size_bits = bit(word, 2) << 3 | bits(word, 7, 6) << 1 | bit(word, 1);

            let addr_size = FromPrimitive::from_u32(addr_size_bits).ok_or(AisError::DecodeIssue)?;
            let size = FromPrimitive::from_u32(size_bits).ok_or(AisError::DecodeIssue)?;

            Ok(Function::Xlea(addr_size, size))
        }
        Opcode::XPUSH | Opcode::XPOP | Opcode::XPUSHIP => {
            let sub_op_bits = bits(word, 10, 9);
            let addr_size_bits = bit(word, 8) << 1 | bit(word, 0);
            let size_bits = bits(word, 7, 6) << 1 | bit(word, 1);
            let sel_bits = bits(word, 5, 2);

            //let sub_op = FromPrimitive::from_u32(sub_op_bits).ok_or(AisError::DecodeIssue)?;
            let addr_size = FromPrimitive::from_u32(addr_size_bits).ok_or(AisError::DecodeIssue)?;
            let size = FromPrimitive::from_u32(size_bits).ok_or(AisError::DecodeIssue)?;
            let sel = FromPrimitive::from_u32(sel_bits).ok_or(AisError::DecodeIssue)?;

            Ok(Function::Xls(
                SubOp::Raw(sub_op_bits.try_into().unwrap()),
                addr_size,
                size,
                sel,
            ))
        }
        _ => todo!(),
    }
}

fn decode_opcode(word: u32) -> Result<Opcode, AisError> {
    let opcode_bits = (word >> 26) & 0x3F;
    FromPrimitive::from_u32(opcode_bits).ok_or(AisError::UnknownOpcode(opcode_bits))
}

pub fn decode(bytes: &[u8]) -> Result<(Instruction, usize), AisError> {
    if bytes.len() < 6 {
        return Err(AisError::DecodeError(bytes.into()));
    }

    let header = &bytes[0..2];
    if header != [0x62, 0x80] {
        return Err(AisError::DecodeError(bytes.into()));
    }

    let word = u32::from_le_bytes(bytes[2..6].try_into().unwrap());

    let opcode = decode_opcode(word)?;
    let mut instr = Instruction::new(opcode);

    let rt_bits: u8 = bits(word, 20, 16).try_into().unwrap();
    let function_bits: u16 = bits(word, 10, 0).try_into().unwrap();
    let imm_bits = bits(word, 15, 0).try_into().unwrap();

    fn reg(bits: u32, field: Field) -> Result<Register, AisError> {
        let x: u8 = bits.try_into().map_err(|_| AisError::Decode(field))?;
        x.try_into().map_err(|_| AisError::Decode(field))
    }

    fn rs(word: u32) -> Result<Option<Register>, AisError> {
        let bits = bits(word, 25, 21);
        reg(bits, Field::RS).map(Some)
    }

    fn rt(word: u32) -> Result<Option<Register>, AisError> {
        let bits = bits(word, 20, 16);
        reg(bits, Field::RT).map(Some)
    }

    fn rd(word: u32) -> Result<Option<Register>, AisError> {
        let bits = bits(word, 15, 11);
        reg(bits, Field::RD).map(Some)
    }

    if instr.is_i_type() {
        instr.rs = rs(word)?;
        instr.rt = rt(word)?;
        instr.imm = Some(imm_bits);
    } else if instr.is_xalu_type() {
        instr.function = Some(decode_xalu_function(word)?);
        instr.rs = rs(word)?;
        instr.rt = rt(word)?;
        instr.rd = rd(word)?;
    } else if instr.is_xalui_type() {
        instr.function = Some(decode_xalu_function(word)?);
        instr.rs = rs(word)?;
        instr.rd = rd(word)?;
        instr.constant = bits(word, 20, 16)
            .try_into()
            .ok()
            .and_then(|x: u8| x.try_into().ok())
            .ok_or(AisError::Decode(Field::Const))
            .map(Some)?;
    } else if instr.opcode == Opcode::XJ {
        instr.rt = rt(word)?;
        instr.function = Some(decode_xj_function(word)?);
    } else if instr.opcode == Opcode::XMISC {
        instr.rs = rs(word)?;
        instr.rt = rt(word)?;
        instr.rd = rd(word)?;

        let subfunc = FromPrimitive::from_u32(bits(word, 10, 6)).ok_or(AisError::DecodeIssue)?;
        let other_bits = bits(word, 5, 0).try_into().unwrap();

        instr.function = Some(Function::Xmisc(subfunc, other_bits));
    } else if instr.is_xls_type() || instr.opcode == Opcode::XLEAD {
        // The XLS type has an other order of fields
        let xls_rs_bits = bits(word, 15, 11).try_into().unwrap();

        instr.rs = Some(reg(xls_rs_bits, Field::RS)?);
        instr.rt = rt(word)?;
        instr.offset = bits(word, 25, 21)
            .try_into()
            .ok()
            .and_then(|x: u8| x.try_into().ok())
            .ok_or(AisError::Decode(Field::Offset))
            .map(Some)?;
        instr.function = Some(decode_function(instr.opcode, word)?);
    } else {
        return Err(AisError::DecodeError(bytes.into()));
    }

    // let code = instr.encode32()?;
    // instr.mask = word & !code;

    // assert!(word == code | instr.mask, "{:?} {:08X} {:08X} {:08X}", instr, word, code, instr.mask);

    Ok((instr, 6))
}
