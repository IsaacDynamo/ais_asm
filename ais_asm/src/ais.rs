/* NOTES

Internal state / registers


TSR         ?Translator Status Register?
TSR.OS      Operand Size
TSR.AS      Address Size
TSR.SAS     Stack Address Size
TSR.SEL?
TSR.DPcntl
TSR.REPN
TSR.LK      Lock, the x86 instruction had a valid LOCK prefix

IMMED
DISP
COUNT

LO/HI/MD         Registers used by multiply and divide


IIR.tttn

CP2 registers
TSC_U       Time stamp counter Upper 32bits
TSC_L       Time stamp counter Lower 32bits
CR0
EFLAGS

NSIP        ... Instruction Pointer, mentioned to be a CP2 register


MTCNT       Load COUNT (?Move To CouNT?)
CTC2    	Store to CP2
CFC2        Move control from CP2


XALU:       CTC2/MFLOU/MFLOI


"Instructions that AND and OR a value with the EFLAGS register"
"Selected x86 control register such as CR0 can be loaded or stored without any protection checking"

MT..        ?Move To?           C0/C1/C2/CNT
MF..        ?Move From?         C0/C1/C2/CNT/HI/LOU/LOI
DMF..       ?Double Move From?  C1
DMT..       ?Double Move To?    C1/C2/MD
CT..        ?Copy To?           C1/C2
CF..        ?Copy From?         C1/C2/PFL

Remaining XMISC:
XRET, XCNULL, XRFP, XHALT SBF/SBN, XTI, XTII, XMDB, XMDBI

*/

use num::FromPrimitive;
use num_derive::FromPrimitive;
use std::{convert::TryFrom, ops::Range};

#[derive(Debug)]
pub enum AisError {
    InvalidRegisterIndex(u8),
    InvalidRegisterName(String),

    Unsupported(Instruction),

    MissingImmediate(Instruction),
    MissingRs(Instruction),
    MissingRt(Instruction),
    MissingRd(Instruction),
    MissingConstant(Instruction),
    MissingOffset(Instruction),
    MissingFunction(Instruction),
    UnsupportedOffset(Offset),

    DecodeError(Vec<u8>),
    DecodeIssue,

    UnknownOpcode(u32),
    UnknownOffset(u8),
    UnknownConst(u8),
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Register(u8);

impl TryFrom<u8> for Register {
    type Error = AisError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            x if x > 31 => Err(AisError::InvalidRegisterIndex(x)),
            x => Ok(Self(x)),
        }
    }
}

impl TryFrom<&str> for Register {
    type Error = AisError;

    fn try_from(x: &str) -> Result<Self, Self::Error> {
        match x {
            "R0" => Ok(Self(0)),

            "R4" => Ok(Self(4)),
            "R5" => Ok(Self(5)),
            "R6" => Ok(Self(6)),
            "R7" => Ok(Self(7)),

            "ES" => Ok(Self(8)),
            "CS" => Ok(Self(9)),
            "SS" => Ok(Self(10)),
            "DS" => Ok(Self(11)),
            "FS" => Ok(Self(12)),
            "GS" => Ok(Self(13)),

            "EAX" => Ok(Self(16)),
            "ECX" => Ok(Self(17)),
            "EDX" => Ok(Self(18)),
            "EBX" => Ok(Self(19)),
            "ESP" => Ok(Self(20)),
            "EBP" => Ok(Self(21)),
            "ESI" => Ok(Self(22)),
            "EDI" => Ok(Self(23)),
            _ => Err(AisError::InvalidRegisterName(x.to_string())),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Const {
    Number(i8),
    // There are some other special case, skip for now
    Raw(u8),
}


impl TryFrom<u8> for Const {
    type Error = AisError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0b00000 => Ok(Self::Number(0)),
            x if x < 32 => Ok(Self::Raw(x)),
            _ => Err(AisError::UnknownConst(value)),
        }
    }

}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Copy, Clone)]
pub enum Offset {
    Number(i8),
    OS,
    PDOS,
    MOS,
    MGS,
    MDOS,
    DF,
    DFOS,
    DISP,
}

impl TryFrom<u8> for Offset {
    type Error = AisError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0b00000 => Ok(Self::Number(0)),
            0b00001 => Ok(Self::Number(1)),
            0b00010 => Ok(Self::Number(2)),
            0b00011 => Ok(Self::Number(4)),
            0b00100 => Ok(Self::Number(8)),
            0b00101 => Ok(Self::Number(16)),
            0b00110 => Ok(Self::Number(24)),
            0b00111 => Ok(Self::Number(32)),
            0b01000 => Ok(Self::Number(10)),
            0b01001 => Ok(Self::Number(-1)),
            0b01010 => Ok(Self::Number(-2)),
            0b01011 => Ok(Self::Number(-4)),
            0b01100 => Ok(Self::Number(-8)),

            0b01111 => Ok(Self::Number(5)),
            0b10000 => Ok(Self::OS),
            0b10001 => Ok(Self::PDOS),

            0b11000 => Ok(Self::MOS),
            0b11001 => Ok(Self::MGS),
            0b11010 => Ok(Self::MDOS),

            0b11100 => Ok(Self::DF),
            0b11101 => Ok(Self::DFOS),

            0b11111 => Ok(Self::DISP),

            _ => Err(AisError::UnknownOffset(value)),
        }
    }
}

#[derive(Debug, Copy, Clone, FromPrimitive)]
pub enum Size {
    Bits16 = 0b000,
    Bits8L = 0b001,
    Bits32 = 0b010,
    Bits8H = 0b011, // Bits16H
    AS = 0b100,     // Address Size
    Bits64 = 0b101,
    OS = 0b110,  // Operand Size
    IND = 0b111, // GS  Gate Size

    SAS = 0b1000,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Copy, Clone, FromPrimitive)]
pub enum Sel {
    ES = 0b0000,
    CS = 0b0001,
    SS = 0b0010,
    DS = 0b0011,
    FS = 0b0100,
    GS = 0b0101,
    GDT = 0b0110,
    LDT = 0b0111,
    IDT = 0b1000,
    TSS = 0b1001,
    FLAT = 0b1010,
    T0 = 0b1011,
    ISEL = 0b1111,
}

#[derive(Debug, Copy, Clone)]
pub enum SubOp {
    Xio(SubOpXio),
    Raw(u8),
}

#[derive(Debug, Copy, Clone, FromPrimitive)]
pub enum SubOpXio {
    Norm = 0,
}

#[derive(Debug, Copy, Clone, FromPrimitive)]
pub enum AddrSize {
    AS = 0b00,  // Address Size
    SAS = 0b01, // Stack Address Size
    Bits16 = 0b10,
    Bits32 = 0b11,
}

#[derive(Debug, Copy, Clone)]
pub enum Function {
    Xio(SubOpXio, AddrSize, Size, Sel),
    Xls(SubOp, AddrSize, Size, Sel),
    Xalu(SubOpXalu, DpCntl),
    Xj(XjSize, XjMode),
    Xlea(AddrSize, Size),
    Xmisc(SubFunc, u8),
    Raw(u16),
}

#[derive(Debug, Copy, Clone, FromPrimitive)]
pub enum SubFunc {
    CFC2 = 0o37,
}

#[derive(Debug, Copy, Clone, FromPrimitive)]
pub enum XjSize {
    Bits16 = 0b00,
    Bits32 = 0b01,
    AS = 0b10, // Address Size
    OS = 0b11, // Operand Size
}

#[derive(Debug, Copy, Clone, FromPrimitive)]
pub enum XjMode {
    AIS = 0b00,
    X86 = 0b11,
}

#[derive(Debug, Copy, Clone, FromPrimitive)]
pub enum DpCntl {
    Word = 0b000,
    Short = 0b001,
    LL = 0b010,
    HL = 0b011,
    LH = 0b100,
    HH = 0b101,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Copy, Clone, PartialEq, FromPrimitive)]
pub enum Opcode {
    XJ = 0o06,
    XJ7 = 0o00, // TODO remove

    // I type
    ORIU = 0o10,
    ADDI = 0o11,
    ANDIU = 0o12,
    ANDIL = 0o13,
    ANDI = 0o14,
    ORI = 0o15,
    XORI = 0o16,
    XORIU = 0o17,

    // XALU type
    XALU = 0o40,
    XALUI = 0o41,
    XALUR = 0o42,
    XALUIR = 0o43,

    XMISC = 0o50,
    XLEAI = 0o53,
    XLEAD = 0o54,

    XL = 0o60,
    XL2 = 0o61,
    XL3 = 0o62,
    XLBI = 0o63,
    XLDESC = 0o64,
    XIOR = 0o65,
    XPOPBR = 0o66,
    XPOP = 0o67,

    XS = 0o70,
    XS2 = 0o71,
    XPUSHI = 0o72,
    XSI = 0o73,
    XPUSHIP = 0o74,
    XIOW = 0o75,
    XSU = 0o76,
    XPUSH = 0o77,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Copy, Clone, FromPrimitive)]
pub enum SubOpXalu {
    SHL = 0o00,
    SHR = 0o02,
    SAR = 0o03,
    ROL = 0o04,
    ROR = 0o05,
    RCL = 0o06,
    RCR = 0o07,
    INC = 0o10,
    CMPS = 0o11,
    DEC = 0o12,
    IMUL = 0o14,
    MUL = 0o15,
    IDIV = 0o16,
    ADD = 0o20,
    ADC = 0o21,
    SUB = 0o22,
    SBB = 0o23,
    AND = 0o24,
    OR = 0o25,
    XOR = 0o26,
    NOR = 0o27,
    CTC2 = 0o31,
    SETCC = 0o35,
    MFLOU = 0o36,
    MFLOI = 0o37,
}

#[derive(Debug, Clone)]
pub struct Instruction {
    // Formats
    //        31:26    25:21    20:16   15:11    10:0
    // I:     Opcode | RS     | RT    | Immediate
    // XALU:  Opcode | RS     | RT    | RD     | Function
    // XALUI: Opcode | RS     | Const | RD     | Function
    // XMISC: Opcode | RS     | RT    | RD     | Function
    // XLS:   Opcode | RS     | Base  | Offset | Function  - Wrong!
    // XLS:   Opcode | Offset | Base  | RS     | Function  - Correct
    pub opcode: Opcode,
    pub rs: Option<Register>,
    pub rt: Option<Register>, // Base
    pub rd: Option<Register>,
    pub imm: Option<u16>,
    pub constant: Option<Const>,
    pub offset: Option<Offset>,
    pub function: Option<Function>,
    pub mask: u32,
}

impl Instruction {
    pub fn new(opcode: Opcode) -> Self {
        Self {
            opcode,
            rs: None,
            rt: None, // base
            rd: None,
            imm: None,
            constant: None,
            offset: None,
            function: None,
            mask: 0
        }
    }

    pub fn i_type(opcode: Opcode, dst: Register, src: Register, imm: u16) -> Self {
        let mut ret = Self::new(opcode);
        ret.rs = Some(src);
        ret.rt = Some(dst);
        ret.imm = Some(imm);
        ret
    }

    pub fn xalur(
        subop: SubOpXalu,
        dpcntl: DpCntl,
        dst: Register,
        src: Register,
        extra: Register,
    ) -> Self {
        let mut ret = Self::new(Opcode::XALUR);
        ret.rs = Some(src);
        ret.rd = Some(dst);
        ret.rt = Some(extra);
        ret.function = Some(Function::Xalu(subop, dpcntl));
        ret
    }

    pub fn xaluir(
        subop: SubOpXalu,
        dpcntl: DpCntl,
        dst: Register,
        src: Register,
        constant: Const,
    ) -> Self {
        let mut ret = Self::new(Opcode::XALUIR);
        ret.rs = Some(src);
        ret.rd = Some(dst);
        ret.constant = Some(constant);
        ret.function = Some(Function::Xalu(subop, dpcntl));
        ret
    }

    pub fn xiow(size: Size, port: Register, value: Register) -> Self {
        let mut instr = Instruction::xls_type(Opcode::XIOW, value, port, Offset::Number(0));
        instr.function = Some(Function::Xio(
            SubOpXio::Norm,
            AddrSize::Bits16,
            size,
            Sel::FLAT,
        ));
        instr
    }

    pub fn xior(size: Size, port: Register, value: Register) -> Self {
        let mut instr = Instruction::xls_type(Opcode::XIOR, value, port, Offset::Number(0));
        instr.function = Some(Function::Xio(
            SubOpXio::Norm,
            AddrSize::Bits16,
            size,
            Sel::FLAT,
        ));
        instr
    }

    pub fn xpush(size: Size, reg: Register) -> Self {
        let mut instr = Instruction::xls_type(
            Opcode::XPUSH,
            reg,
            "ESP".try_into().unwrap(),
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

    pub fn xpop(size: Size, reg: Register) -> Self {
        let mut instr = Instruction::xls_type(
            Opcode::XPOP,
            reg,
            "ESP".try_into().unwrap(),
            Offset::Number(4),
        );
        instr.function = Some(Function::Xls(
            SubOp::Raw(0),
            AddrSize::Bits32,
            size,
            Sel::SS,
        ));
        instr
    }

    pub fn xpuship(size: Size) -> Self {
        let mut instr = Instruction::xls_type(
            Opcode::XPUSHIP,
            0.try_into().unwrap(),
            "ESP".try_into().unwrap(),
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

    pub fn xlead(
        dst: Register,
        base: Register,
        offset: Offset,
        addr_size: AddrSize,
        size: Size,
    ) -> Self {
        let mut instr = Instruction::xls_type(Opcode::XLEAD, dst, base, offset);
        instr.function = Some(Function::Xlea(addr_size, size));
        instr
    }

    pub fn xleai(
        dst: Register,
        base: Register,
        index: Register,
        addr_size: AddrSize,
        size: Size,
    ) -> Self {
        let mut instr = Self::new(Opcode::XLEAI);
        instr.rs = Some(dst);
        instr.rt = Some(base);
        instr.rd = Some(index);
        instr.function = Some(Function::Xlea(addr_size, size));
        instr
    }

    pub fn cfc2(dst: Register, src: Register) -> Self {
        let mut instr = Self::new(Opcode::XMISC);
        instr.rt = Some(dst);
        instr.rd = Some(src);
        instr.function = Some(Function::Xmisc(SubFunc::CFC2, 0));
        instr
    }

    pub fn xj(base: Register) -> Self {
        let mut ret = Self::new(Opcode::XJ);
        ret.rt = Some(base);
        ret.function = Some(Function::Xj(XjSize::Bits32, XjMode::AIS));
        ret.mask = 0b0001 << 2;
        ret
    }

    pub fn xls_type(opcode: Opcode, rs: Register, base: Register, offset: Offset) -> Self {
        let mut ret = Self::new(opcode);
        ret.rs = Some(rs);
        ret.rt = Some(base);
        ret.offset = Some(offset);
        ret
    }

    fn is_i_type(&self) -> bool {
        matches!(
            self.opcode,
            Opcode::ORIU
                | Opcode::ADDI
                | Opcode::ANDIU
                | Opcode::ANDIL
                | Opcode::ANDI
                | Opcode::ORI
                | Opcode::XORI
                | Opcode::XORIU
        )
    }

    fn is_xalu_type(&self) -> bool {
        matches!(self.opcode, Opcode::XALU | Opcode::XALUR)
    }

    fn is_xalui_type(&self) -> bool {
        matches!(self.opcode, Opcode::XALUI | Opcode::XALUIR)
    }

    fn is_xls_type(&self) -> bool {
        matches!(
            self.opcode,
            Opcode::XIOR | Opcode::XIOW | Opcode::XPUSH | Opcode::XPOP | Opcode::XPUSHIP
        )
    }

    fn encode_opcode(&self) -> Result<u32, AisError> {
        Ok((self.opcode as u32) << 26)
    }

    fn encode_rs(&self) -> Result<u32, AisError> {
        Self::encode_register(&self.rs, || AisError::MissingRs(self.clone())).map(|x| x << 21)
    }

    fn encode_rt(&self) -> Result<u32, AisError> {
        Self::encode_register(&self.rt, || AisError::MissingRt(self.clone())).map(|x| x << 16)
    }

    fn encode_rd(&self) -> Result<u32, AisError> {
        Self::encode_register(&self.rd, || AisError::MissingRd(self.clone())).map(|x| x << 11)
    }

    fn encode_register<F: FnOnce() -> AisError>(
        register: &Option<Register>,
        func: F,
    ) -> Result<u32, AisError> {
        register.as_ref().ok_or_else(func).map(|x| x.0.into())
    }

    fn encode_imm(&self) -> Result<u32, AisError> {
        self.imm
            .ok_or_else(|| AisError::MissingImmediate(self.clone()))
            .map(|x| x.into())
    }

    fn encode_const(&self) -> Result<u32, AisError> {
        let c = self
            .constant
            .ok_or_else(|| AisError::MissingConstant(self.clone()))?;

        let bits = match c {
            Const::Number(0) => 0b00000,
            Const::Number(1) => 0b00001,

            Const::Number(5) => 0b01111,

            Const::Raw(x) => x.into(),
            _ => todo!(),
        };

        Ok(bits << 16)
    }

    fn encode_offset(&self) -> Result<u32, AisError> {
        let offset = self
            .offset
            .ok_or_else(|| AisError::MissingOffset(self.clone()))?;

        let offset_bits = match offset {
            Offset::Number(0) => 0b00000,
            Offset::Number(1) => 0b00001,
            Offset::Number(2) => 0b00010,
            Offset::Number(4) => 0b00011,
            Offset::Number(8) => 0b00100,
            Offset::Number(16) => 0b00101,
            Offset::Number(24) => 0b00110,
            Offset::Number(32) => 0b00111,
            Offset::Number(10) => 0b01000,
            Offset::Number(-1) => 0b01001,
            Offset::Number(-2) => 0b01010,
            Offset::Number(-4) => 0b01011,
            Offset::Number(-8) => 0b01100,

            Offset::Number(5) => 0b01111,

            Offset::OS => 0b10000,
            Offset::PDOS => 0b10001,

            Offset::MOS => 0b11000,
            Offset::MGS => 0b11001,
            Offset::MDOS => 0b11010,

            Offset::DF => 0b11100,
            Offset::DFOS => 0b11101,

            Offset::DISP => 0b11111,

            Offset::Number(_) => return Err(AisError::UnsupportedOffset(offset)),
        };

        Ok(offset_bits << 21)
    }

    fn encode_function(&self) -> Result<u32, AisError> {
        let function = self
            .function
            .ok_or_else(|| AisError::MissingFunction(self.clone()))?;

        let bits = match function {
            Function::Xalu(sub_op, dp_cntl) => (sub_op as u32) | (dp_cntl as u32) << 5,
            Function::Xio(sub_op, addr_size, size, sel) => {
                let subop_bits = (sub_op as u32) << 9; // self.encode_sub_op_xls(sub_op)?;
                subop_bits
                    | (addr_size as u32 & 2) << 7
                    | ((size as u32) & 0x6) << 5
                    | (sel as u32) << 2
                    | (size as u32 & 1) << 1
                    | addr_size as u32 & 1
            }
            Function::Xj(size, mode) => (size as u32) << 6 | mode as u32,
            Function::Xls(sub_op, addr_size, size, sel) => {
                let subop_bits = 0; //(sub_op as u32) << 9; // self.encode_sub_op_xls(sub_op)?;
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

                bits(addr_size, 1..1) << 8
                    | bits(size, 2..1) << 6
                    | bits(size, 3..3) << 2
                    | bits(size, 0..0) << 1
                    | bits(addr_size, 0..0)
            },
            Function::Raw(x) => {
                x.into()
            },
            Function::Xmisc(sub_func, raw) => {
                let subfunc_bits = sub_func as u32;

                subfunc_bits << 6 | raw as u32
            }
        };

        Ok(bits)
    }

    pub fn encode32(&self) -> Result<u32, AisError> {
        let instr = if self.is_i_type() {
            let op = self.encode_opcode()?;
            let rs = self.encode_rs()?;
            let rt = self.encode_rt()?;
            let imm = self.encode_imm()?;

            op | rs | rt | imm
        } else if self.is_xalu_type() {
            let op = self.encode_opcode()?;
            let rs = self.encode_rs()?;
            let rt = self.encode_rt()?;
            let rd = self.encode_rd()?;
            let function = self.encode_function()?;

            op | rs | rt | rd | function
        } else if self.is_xalui_type() {
            let op = self.encode_opcode()?;
            let rs = self.encode_rs()?;
            let c = self.encode_const()?;
            let rd = self.encode_rd()?;
            let function = self.encode_function()?;

            op | rs | c | rd | function
        } else if self.opcode == Opcode::XJ || self.opcode == Opcode::XJ7 {
            let op = self.encode_opcode()?;
            let rt = self.encode_rt()?;
            let function = self.encode_function()?;

            //assert!(function == 0b01_0001_00); // 32bit & stay in AIS mode

            op | rt | function
        } else if self.opcode == Opcode::XMISC {
            let op = self.encode_opcode()?;
            let rs = 0; //self.encode_rs()?;
            let rt = self.encode_rt()?;
            let rd = self.encode_rd()?;
            let function = self.encode_function()?;

            op | rs | rt | rd | function
        } else if self.is_xls_type() || self.opcode == Opcode::XLEAD {
            let op = self.encode_opcode()?;
            let rs = Self::encode_register(&self.rs, || AisError::MissingRs(self.clone()))? << 11;
            let base = self.encode_rt()?;
            let offset = self.encode_offset()?;
            let function = self.encode_function()?;

            //assert!(function == 0b00_1_00_1010_1_0);

            op | offset | base | rs | function
        } else {
            return Err(AisError::Unsupported(self.clone()));
        };

        Ok(instr | self.mask)
    }

    pub fn encode(&self) -> Result<Vec<u8>, AisError> {

        let instr = self.encode32()?;

        let mut data = Vec::new();
        data.extend_from_slice(&[0x62, 0x80]);
        data.extend_from_slice(&instr.to_le_bytes());
        Ok(data)
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

        let rs_bits: u8 = bits(word, 25..21).try_into().unwrap();
        let rt_bits: u8 = bits(word, 20..16).try_into().unwrap();
        let rd_bits: u8 = bits(word, 15..11).try_into().unwrap();
        let function_bits: u16 = bits(word, 10..0).try_into().unwrap();
        let imm_bits = bits(word, 15..0).try_into().unwrap();

        if instr.is_i_type() {
            instr.rs = Some(rs_bits.try_into()?);
            instr.rt = Some(rt_bits.try_into()?);
            instr.imm = Some(imm_bits);
        } else if instr.is_xalu_type() {
            instr.function = Some(decode_xalu_function(word)?);
            instr.rs = Some(rs_bits.try_into()?);
            instr.rt = Some(rt_bits.try_into()?);
            instr.rd = Some(rd_bits.try_into()?);
        } else if instr.is_xalui_type() {
            instr.function = Some(decode_xalu_function(word)?);
            instr.rs = Some(rs_bits.try_into()?);
            instr.rd = Some(rd_bits.try_into()?);
            instr.constant = Some(rt_bits.try_into()?);
        } else if instr.opcode == Opcode::XJ || instr.opcode == Opcode::XJ7 {
            instr.rt = Some(rt_bits.try_into()?);
            instr.function = Some(decode_xj_function(word)?);
        } else if instr.opcode == Opcode::XMISC {
            instr.rs = Some(rs_bits.try_into()?);
            instr.rt = Some(rt_bits.try_into()?);
            instr.rd = Some(rd_bits.try_into()?);

            let subfunc =  FromPrimitive::from_u32(bits(word, 10..6)).ok_or(AisError::DecodeIssue)?;
            let other_bits = bits(word, 5..0).try_into().unwrap();

            instr.function = Some(Function::Xmisc(subfunc, other_bits));
        } else if instr.is_xls_type() || instr.opcode == Opcode::XLEAD {
            // The XLS type has an other order of fields
            let xls_rs_bits = rd_bits;
            let xls_offset_bits = rs_bits;

            instr.rs = Some(xls_rs_bits.try_into()?);
            instr.rt = Some(rt_bits.try_into()?);
            instr.offset = Some(xls_offset_bits.try_into()?);
            instr.function = Some(decode_function(instr.opcode, word)?);
        } else {
            return Err(AisError::DecodeError(bytes.into()));
        }

        let code = instr.encode32()?;
        instr.mask = word & !code;

        assert!(word == code | instr.mask);

        Ok((instr, 6))
    }
}

fn decode_xalu_function(word: u32) -> Result<Function, AisError> {
    let sub_op_bits = bits(word, 4..0);
    let dp_cntl_bits = bits(word, 7..5);
    let sub_op = FromPrimitive::from_u32(sub_op_bits).ok_or(AisError::DecodeIssue)?;
    let dp_cntl = FromPrimitive::from_u32(dp_cntl_bits).ok_or(AisError::DecodeIssue)?;
    Ok(Function::Xalu(sub_op, dp_cntl))
}

fn bits(word: u32, bits: Range<u32>) -> u32 {
    let mask = (1 << (bits.start - bits.end + 1)) - 1;
    (word >> bits.end) & mask
}

fn decode_xio_function(word: u32) -> Result<Function, AisError> {
    let sub_op_bits = bits(word, 10..9);
    let addr_size_bits = bits(word, 8..8) << 1 | bits(word, 0..0);
    let size_bits = bits(word, 7..6) << 1 | bits(word, 1..1);
    let sel_bits = bits(word, 5..2);

    let sub_op = FromPrimitive::from_u32(sub_op_bits).ok_or(AisError::DecodeIssue)?;
    let addr_size = FromPrimitive::from_u32(addr_size_bits).ok_or(AisError::DecodeIssue)?;
    let size = FromPrimitive::from_u32(size_bits).ok_or(AisError::DecodeIssue)?;
    let sel = FromPrimitive::from_u32(sel_bits).ok_or(AisError::DecodeIssue)?;

    Ok(Function::Xio(sub_op, addr_size, size, sel))
}

fn decode_xj_function(word: u32) -> Result<Function, AisError> {
    let size = FromPrimitive::from_u32(bits(word, 7..6)).ok_or(AisError::DecodeIssue)?;
    let mode = FromPrimitive::from_u32(bits(word, 1..0)).ok_or(AisError::DecodeIssue)?;
    Ok(Function::Xj(size, mode))
}

fn decode_function(opcode: Opcode, word: u32) -> Result<Function, AisError> {
    match opcode {
        Opcode::XIOR | Opcode::XIOW => decode_xio_function(word),
        Opcode::XLEAD | Opcode::XLEAI => {
            let addr_size_bits = bits(word, 8..8) << 1 | bits(word, 0..0);
            let size_bits = bits(word, 2..2) << 3 | bits(word, 7..6) << 1 | bits(word, 1..1);

            let addr_size = FromPrimitive::from_u32(addr_size_bits).ok_or(AisError::DecodeIssue)?;
            let size = FromPrimitive::from_u32(size_bits).ok_or(AisError::DecodeIssue)?;

            Ok(Function::Xlea(addr_size, size))
        }
        Opcode::XPUSH | Opcode::XPOP | Opcode::XPUSHIP => {
            let sub_op_bits = bits(word, 10..9);
            let addr_size_bits = bits(word, 8..8) << 1 | bits(word, 0..0);
            let size_bits = bits(word, 7..6) << 1 | bits(word, 1..1);
            let sel_bits = bits(word, 5..2);

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
