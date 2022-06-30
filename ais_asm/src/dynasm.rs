use crate::ais::{AisError, Instruction, Opcode, Register};

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
                self.gen(Instruction::i_type(
                    Opcode::ORI,
                    dst.clone(),
                    0.try_into()?,
                    imm as u16,
                ))?;
                self.gen(Instruction::i_type(
                    Opcode::ORIU,
                    dst.clone(),
                    dst,
                    (imm >> 16) as u16,
                ))?;
            }
            (false, true) => self.gen(Instruction::i_type(
                Opcode::ORIU,
                dst,
                0.try_into()?,
                (imm >> 16) as u16,
            ))?,
            (true, _) => self.gen(Instruction::i_type(
                Opcode::ORI,
                dst,
                0.try_into()?,
                imm as u16,
            ))?,
        }

        Ok(())
    }

    pub fn gen_load_symbol(&mut self, dst: Register, sym: Sym) -> Result<(), DynAsmError> {
        let low = self.sym_ref_imm_low(sym)?;
        self.gen(Instruction::i_type(
            Opcode::ORI,
            dst.clone(),
            0.try_into()?,
            low,
        ))?;

        let high = self.sym_ref_imm_high(sym)?;
        self.gen(Instruction::i_type(Opcode::ORIU, dst.clone(), dst, high))
    }

    pub fn gen_jump(&mut self, sym: Sym) -> Result<(), DynAsmError> {
        self.gen_load_symbol("R4".try_into()?, sym)?;
        self.gen(Instruction::xj("R4".try_into()?))?;
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
