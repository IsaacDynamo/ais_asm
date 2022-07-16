use crate::ais::{AisError, Const, Instruction, Register, Size};
use crate::asm;

#[derive(Debug)]
pub enum DynAsmError {
    AisError(AisError),
    InvalidSym,
    SymbolRedefined,
    ResolveUnstable,
}

impl From<AisError> for DynAsmError {
    fn from(x: AisError) -> Self {
        Self::AisError(x)
    }
}

#[derive(Copy, Clone)]
pub struct Sym(usize);

enum Symbol {
    Unresolved(Vec<SymRef>),
    Resolved(u32),
}

#[derive(Debug)]
enum SymRefKind {
    HighImm,
    LowImm,
}

fn imm_high(addr: u32) -> u16 {
    ((addr >> 16) & 0xFFFF).try_into().unwrap()
}

fn imm_low(addr: u32) -> u16 {
    (addr & 0xFFFF).try_into().unwrap()
}

#[derive(Debug)]
struct SymRef {
    kind: SymRefKind,
    offset: u32,
}

pub struct DynAsm {
    base: u32,
    memory: Vec<u8>,
    symbols: Vec<Symbol>,
}

const HEADER: &[u8] = &[
    0xE8, 0x00, 0x00, 0x00, 0x00, //     call 1f
    0x58, // 1:  pop eax
    0x83, 0xC0, 0x06, //     add eax, 6
    0x0F,
    0x3F, //     jmpai eax
          // <- jmpai should jump to here, this is where the AI wrapper instruction start.
];

const FOOTER: &[u8] = &[
    0xC3, // ret
];

impl DynAsm {
    pub fn new(base: u32) -> Self {
        Self {
            base,
            memory: Vec::new(),
            symbols: Vec::new(),
        }
    }

    fn offset(&self) -> u32 {
        self.memory.len().try_into().unwrap()
    }

    fn addr(&self) -> u32 {
        self.offset() + self.base
    }

    fn symbol(&mut self, sym: Sym) -> Result<&mut Symbol, DynAsmError> {
        self.symbols.get_mut(sym.0).ok_or(DynAsmError::InvalidSym)
    }

    fn sym_ref_resolve(&mut self, sym_ref: SymRef, addr: u32) -> Result<(), DynAsmError> {
        println!("fixup: {:?} = {:X}", sym_ref, addr);

        // Decode
        let start = sym_ref.offset.try_into().unwrap();
        let end = self.memory.len();
        let bytes = self.memory.get_mut(start..end).unwrap();
        let (mut instr, len) = Instruction::decode(bytes)?;

        // Fixup
        match sym_ref.kind {
            SymRefKind::LowImm => {
                instr.imm = Some(imm_low(addr));
            }
            SymRefKind::HighImm => {
                instr.imm = Some(imm_high(addr));
            }
        }

        // Encode
        let new_bytes = instr.encode()?;
        if new_bytes.len() != len {
            return Err(DynAsmError::ResolveUnstable);
        }

        let old_bytes = &mut bytes[0..len];
        old_bytes.copy_from_slice(&new_bytes);

        Ok(())
    }

    fn sym_resolve(&mut self, sym: Sym, addr: u32) -> Result<(), DynAsmError> {
        let symbol = self.symbol(sym)?;

        let sym_refs = match symbol {
            Symbol::Unresolved(refs) => {
                let refs = core::mem::take(refs);
                *symbol = Symbol::Resolved(addr);
                refs
            }
            Symbol::Resolved(_) => return Err(DynAsmError::SymbolRedefined),
        };

        for sym_ref in sym_refs {
            self.sym_ref_resolve(sym_ref, addr)?;
        }

        Ok(())
    }

    fn sym_ref(&mut self, sym: Sym, kind: SymRefKind) -> Result<u32, DynAsmError> {
        let sym_ref = SymRef {
            offset: self.offset(),
            kind,
        };

        match self.symbol(sym)? {
            Symbol::Unresolved(refs) => {
                refs.push(sym_ref);
                Ok(0)
            }
            Symbol::Resolved(addr) => Ok(*addr),
        }
    }

    fn sym_ref_imm_high(&mut self, sym: Sym) -> Result<u16, DynAsmError> {
        self.sym_ref(sym, SymRefKind::HighImm).map(imm_high)
    }

    fn sym_ref_imm_low(&mut self, sym: Sym) -> Result<u16, DynAsmError> {
        self.sym_ref(sym, SymRefKind::LowImm).map(imm_low)
    }

    pub fn new_sym(&mut self) -> Sym {
        let entry = Symbol::Unresolved(Vec::new());
        self.symbols.push(entry);
        Sym(self.symbols.len() - 1)
    }

    pub fn new_sym_here(&mut self) -> Sym {
        let sym = self.new_sym();
        self.sym_resolve(sym, self.addr()).unwrap();
        sym
    }

    pub fn sym_addr(&mut self, sym: Sym) -> Result<Option<u32>, DynAsmError> {
        Ok(match self.symbol(sym)? {
            Symbol::Unresolved(_) => None,
            Symbol::Resolved(addr) => Some(*addr),
        })
    }

    pub fn set_sym_here(&mut self, sym: Sym) -> Result<(), DynAsmError> {
        self.sym_resolve(sym, self.addr())
    }

    pub fn gen(&mut self, instruction: Instruction) -> Result<(), DynAsmError> {
        let instr = instruction.encode()?;
        self.memory.extend_from_slice(instr.as_slice());
        Ok(())
    }

    pub fn gen_load(&mut self, dst: Register, imm: u32) -> Result<(), DynAsmError> {
        let low_zero = imm & 0xFFFF == 0;
        let high_zero = imm & 0xFFFF0000 == 0;

        match (high_zero, low_zero) {
            (false, false) => {
                self.gen(asm::xori(dst, Register::R0, imm as u16))?;
                self.gen(asm::xoriu(dst, dst, (imm >> 16) as u16))?;
            }
            (false, true) => self.gen(asm::xoriu(dst, Register::R0, (imm >> 16) as u16))?,
            (true, _) => self.gen(asm::xori(dst, Register::R0, imm as u16))?,
        }

        Ok(())
    }

    pub fn gen_load_symbol(&mut self, dst: Register, sym: Sym) -> Result<(), DynAsmError> {
        let low = self.sym_ref_imm_low(sym)?;
        self.gen(asm::xori(dst, Register::R0, low))?;

        let high = self.sym_ref_imm_high(sym)?;
        self.gen(asm::xoriu(dst, dst, high))
    }

    pub fn gen_jump(&mut self, sym: Sym) -> Result<(), DynAsmError> {
        self.gen_load_symbol(Register::R4, sym)?;
        self.gen(asm::j(Register::R4))?;
        Ok(())
    }

    pub fn gen_cond_jump(&mut self, cond: Register, t: Sym, f: Sym) -> Result<(), DynAsmError> {
        let r0 = Register::R0;
        let r4 = Register::R4;
        let r5 = Register::R5;

        // AND condition with one to make sure its 0 or 1
        self.gen(asm::andi(r5, cond, Const::Number(1)))?;

        // Map 0 to 0x0000_0000 and 1 to 0xFFFF_FFFF
        self.gen(asm::sub(r4, r0, r5))?;

        // AND in true branch sym address
        let low = self.sym_ref_imm_low(t)?;
        self.gen(asm::xandil(r4, r4, low))?;
        let high = self.sym_ref_imm_high(t)?;
        self.gen(asm::xandiu(r4, r4, high))?;

        // Map 0 to 0xFFFF_FFFF and 1 to 0x0000_0000
        self.gen(asm::subi(r5, r5, Const::Number(1)))?;

        // AND in false branch sym address
        let low = self.sym_ref_imm_low(f)?;
        self.gen(asm::xandil(r5, r5, low))?;
        let high = self.sym_ref_imm_high(f)?;
        self.gen(asm::xandiu(r5, r5, high))?;

        // Merge jump locations
        self.gen(asm::or(r4, r4, r5))?;

        // Jump
        self.gen(asm::j(r4))?;

        Ok(())
    }

    pub fn gen_call(&mut self, sym: Sym) -> Result<(), DynAsmError> {
        let r4 = Register::R4;
        self.gen_load_symbol(r4, sym)?;
        self.gen(asm::puship(Size::Bits32))?;
        self.gen(asm::j(r4))?;
        Ok(())
    }

    pub fn gen_ret(&mut self) -> Result<(), DynAsmError> {
        let r4 = Register::R4;
        self.gen(asm::popsp(Size::Bits32, r4))?;
        self.gen(asm::addi(r4, r4, Const::Number(6)))?;
        self.gen(asm::j(r4))?;
        Ok(())
    }

    pub fn gen_header(&mut self) {
        self.memory.extend_from_slice(HEADER);
    }

    pub fn gen_footer(&mut self) {
        self.memory.extend_from_slice(FOOTER);
    }

    pub fn memory(&self) -> &Vec<u8> {
        &self.memory
    }

    pub fn dump(&self) {
        let mut bytes = &self.memory[HEADER.len()..self.memory.len() - FOOTER.len()];
        loop {
            if bytes.is_empty() {
                break;
            }

            match Instruction::decode(bytes) {
                Ok((i, size)) => {
                    println!("{:?}", i);
                    bytes = &bytes[size..];
                }
                Err(e) => {
                    println!("{:?}", e);
                    break;
                }
            }
        }
    }
}
