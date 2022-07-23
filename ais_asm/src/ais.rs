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

use num_derive::FromPrimitive;
use std::convert::TryFrom;

#[derive(Debug)]
pub enum AisError {
    DecodeSize,
    DecodeHeader,
    Decode(Field),
    Missing(Field),
    Unsupported(Field),
}

#[derive(Debug, Copy, Clone)]
pub enum Field {
    Opcode,
    Const,
    Offset,
    Immediate,
    RS,
    RT,
    RD,
    Function,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Register(pub u8);

impl Register {
    pub const R0: Self = Self(0);

    pub const R4: Self = Self(4);
    pub const R5: Self = Self(5);
    pub const R6: Self = Self(6);
    pub const R7: Self = Self(7);

    pub const CS: Self = Self(9);
    pub const SS: Self = Self(10);
    pub const DS: Self = Self(11);
    pub const FS: Self = Self(12);
    pub const GS: Self = Self(13);

    pub const EAX: Self = Self(16);
    pub const ECX: Self = Self(17);
    pub const EDX: Self = Self(18);
    pub const EBX: Self = Self(19);
    pub const ESP: Self = Self(20);
    pub const EBP: Self = Self(21);
    pub const ESI: Self = Self(22);
    pub const EDI: Self = Self(23);
}

impl TryFrom<u8> for Register {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            x if x <= 31 => Ok(Self(x)),
            _ => Err(()),
        }
    }
}

impl TryInto<u8> for Register {
    type Error = ();

    fn try_into(self) -> Result<u8, Self::Error> {
        match self {
            Register(x) if x <= 31 => Ok(x),
            _ => Err(()),
        }
    }
}

#[test]
fn register_from_into_identity() {
    for i in 0..32u8 {
        let reg = Register::try_from(i).unwrap();
        let j: u8 = reg.try_into().unwrap();
        assert_eq!(j, i)
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Const {
    Number(i8),
    // There are some other special case, skip for now
    Raw(u8),
}

impl TryFrom<u8> for Const {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            0b00000 => Self::Number(0),
            x if x < 32 => Self::Raw(x),
            _ => return Err(()),
        })
    }
}

impl TryInto<u8> for Const {
    type Error = ();

    fn try_into(self) -> Result<u8, Self::Error> {
        Ok(match self {
            Const::Number(0) => 0b00000,
            Const::Number(1) => 0b00001,

            Const::Number(5) => 0b01111,

            Const::Number(6) => 0b10010,

            Const::Raw(x) if x <= 0b11111 => x,
            Const::Raw(_) => return Err(()),
            _ => todo!(),
        })
    }
}

#[test]
fn const_from_into_identity() {
    for i in 0..32u8 {
        let reg = Const::try_from(i).unwrap();
        let j: u8 = reg.try_into().unwrap();
        assert_eq!(j, i)
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
    Raw(u8),
}

impl TryFrom<u8> for Offset {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            0b00000 => Self::Number(0),
            0b00001 => Self::Number(1),
            0b00010 => Self::Number(2),
            0b00011 => Self::Number(4),
            0b00100 => Self::Number(8),
            0b00101 => Self::Number(16),
            0b00110 => Self::Number(24),
            0b00111 => Self::Number(32),
            0b01000 => Self::Number(10),
            0b01001 => Self::Number(-1),
            0b01010 => Self::Number(-2),
            0b01011 => Self::Number(-4),
            0b01100 => Self::Number(-8),

            0b01111 => Self::Number(5),
            0b10000 => Self::OS,
            0b10001 => Self::PDOS,

            0b11000 => Self::MOS,
            0b11001 => Self::MGS,
            0b11010 => Self::MDOS,

            0b11100 => Self::DF,
            0b11101 => Self::DFOS,

            0b11111 => Self::DISP,

            x if x <= 0b11111 => Self::Raw(x),
            _ => return Err(()),
        })
    }
}

impl TryInto<u8> for Offset {
    type Error = ();

    fn try_into(self) -> Result<u8, Self::Error> {
        let bits = match self {
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

            Offset::Raw(x) if x <= 0b11111 => x,

            _ => return Err(()),
        };

        Ok(bits)
    }
}

#[test]
fn offset_from_into_identity() {
    for i in 0..32u8 {
        let reg = Offset::try_from(i).unwrap();
        let j: u8 = reg.try_into().unwrap();
        assert_eq!(j, i)
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
    Raw(u8),
}

impl TryFrom<u8> for SubOp {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            x if x < 4 => Self::Raw(x),
            _ => return Err(()),
        })
    }
}

impl TryInto<u8> for SubOp {
    type Error = ();

    fn try_into(self) -> Result<u8, Self::Error> {
        Ok(match self {
            Self::Raw(x) if x < 4 => x,
            _ => return Err(()),
        })
    }
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

#[derive(Debug, Copy, Clone)]
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
    pub leftovers: u32, // Bits that are not represented by other fields.
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
            leftovers: 0,
        }
    }

    pub fn is_i_type(&self) -> bool {
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

    pub fn is_xalu_type(&self) -> bool {
        matches!(self.opcode, Opcode::XALU | Opcode::XALUR)
    }

    pub fn is_xalui_type(&self) -> bool {
        matches!(self.opcode, Opcode::XALUI | Opcode::XALUIR)
    }

    pub fn is_xls_type(&self) -> bool {
        matches!(
            self.opcode,
            Opcode::XIOR | Opcode::XIOW | Opcode::XPUSH | Opcode::XPOP | Opcode::XPUSHIP
        )
    }

    pub fn encode(&self) -> Result<Vec<u8>, AisError> {
        crate::encode::encode(self)
    }

    pub fn decode(bytes: &[u8]) -> Result<(Instruction, usize), AisError> {
        crate::decode::decode(bytes)
    }
}
