use std::fmt;

type AssemblyString = String;

fn format_data(data: &[u8]) -> String {
    let mut result = String::new();
    let mut in_string = false;
    for byte in data {
        let c = *byte as char;
        if c.is_ascii_graphic() || c == ' ' {
            if !in_string {
                result.push('"');
                in_string = true;
            }
            result.push(c);
        } else {
            if in_string {
                result.push('"');
                result.push(',');
                in_string = false;
            }
            result.push_str(&format!("{:#02x}", byte));
            result.push(',');
        }
    }
    result = result.trim_end_matches(',').to_owned();
    if in_string {
        result.push('"');
    }
    result
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Register64 {
    rax,
    rbx,
    rcx,
    rdx,
    rsi,
    rdi,
    rsp,
    r10,
    r11,
    r12,
}
impl fmt::Display for Register64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// What effects does instruction cause
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Effects {
    /// Affects flags (Zero flag considered here)
    pub flags: bool,
    /// Affects registers
    pub registers: bool,
    /// Conditional branching
    pub control_flow: bool,
    /// Affects stack
    pub stack: bool,
    /// File IO
    pub io: bool,
}
impl Effects {
    /// Volatile operation, should not be moved or eliminated
    pub const VOLATILE: Self = Self {
        flags: true,
        registers: true,
        control_flow: true,
        stack: true,
        io: true,
    };

    /// Register-only operation
    pub const REG: Self = Self {
        flags: false,
        registers: true,
        control_flow: false,
        stack: false,
        io: false,
    };

    /// Flag operation
    pub const FLAG: Self = Self {
        flags: true,
        registers: false,
        control_flow: false,
        stack: false,
        io: false,
    };

    /// Register + Flag operation
    pub const ARITHMETIC: Self = Self {
        flags: true,
        registers: true,
        control_flow: false,
        stack: false,
        io: false,
    };

    /// Jump
    pub const JUMP: Self = Self {
        flags: false,
        registers: false,
        control_flow: true,
        stack: false,
        io: false,
    };

    /// Label, considering origin
    pub const LABEL: Self = Self {
        flags: true,
        registers: true,
        control_flow: false,
        stack: false,
        io: false,
    };

    /// No-op
    pub const NOP: Self = Self {
        flags: false,
        registers: false,
        control_flow: false,
        stack: false,
        io: false,
    };
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Instruction {
    /// Black box, i.e. raw assembly that optimizer should pass through
    BlackBox(AssemblyString, Effects),
    /// Named black box, i.e. black box containing identifier for optimizer
    NamedBlackBox(String, AssemblyString, Effects),
    /// `mov rax, 2`
    MovImm(Register64, u64),
    /// `mov rax, label`
    MovImmVar(Register64, String),
    /// `mov rax, rbx`
    Mov(Register64, Register64),
    /// `mov byte [rax], 2`
    MovPtr8Imm(Register64, u8),
    /// `mov word [rax], 2`
    MovPtr16Imm(Register64, u16),
    /// `mov dword [rax], 2`
    MovPtr32Imm(Register64, u32),
    /// `mov quad [rax], 2`
    MovPtr64Imm(Register64, u64),
    /// `add rax, 2`
    AddImm(Register64, u64),
    /// `sub rax, 2`
    SubImm(Register64, u64),
    /// `add byte [rax], 2`
    AddPtr8Imm(Register64, u8),
    /// `add word [rax], 2`
    AddPtr16Imm(Register64, u16),
    /// `add dword [rax], 2`
    AddPtr32Imm(Register64, u32),
    /// `add quad [rax], 2`
    AddPtr64Imm(Register64, u64),
    /// `test eax, eax` (always followed by conditional jump)
    IsZero(Register64),
    /// `cmp byte [eax], 0` (always followed by conditional jump)
    IsZeroPtr8(Register64),
    /// `jz .label2`
    JumpZero(String),
    /// `jnz .label2`
    JumpNonZero(String),
    /// `jmp .label2`
    Jump(String),
    /// `.label2:` (Label(".label2"))
    Label(String),
    /// `name: db "abc", 10, 13` (in section .data)
    Data(String, Vec<u8>),
}
impl Instruction {
    pub fn to_source(&self) -> String {
        match self {
            Self::BlackBox(src, _) => src.clone(),
            Self::NamedBlackBox(_, src, _) => src.clone(),
            Self::MovImm(r, imm) => match imm {
                0 => format!("xor {}, {}", r, r),
                i => format!("mov {}, {}", r, i),
            },
            Self::MovImmVar(r, label) => format!("mov {}, {}", r, label),
            Self::Mov(r1, r2) => format!("mov {}, {}", r1, r2),
            Self::MovPtr8Imm(r, imm) => format!("mov byte [{}], {}", r, imm),
            Self::MovPtr16Imm(r, imm) => format!("mov word [{}], {}", r, imm),
            Self::MovPtr32Imm(r, imm) => format!("mov dword [{}], {}", r, imm),
            Self::MovPtr64Imm(r, imm) => format!("mov quad [{}], {}", r, imm),
            Self::AddImm(r, imm) => match imm {
                1 => format!("inc {}", r),
                i => format!("add {}, {}", r, i),
            },
            Self::SubImm(r, imm) => match imm {
                1 => format!("dec {}", r),
                i => format!("sub {}, {}", r, i),
            },
            Self::AddPtr8Imm(r, imm) => match imm {
                255 => format!("dec byte [{}]", r),
                1 => format!("inc byte [{}]", r),
                i => format!("add byte [{}], {}", r, i),
            },
            Self::AddPtr16Imm(r, imm) => format!("add word [{}], {}", r, imm),
            Self::AddPtr32Imm(r, imm) => format!("add dword [{}], {}", r, imm),
            Self::AddPtr64Imm(r, imm) => format!("add quad [{}], {}", r, imm),
            Self::IsZero(r) => format!("test {}, {}", r, r),
            Self::IsZeroPtr8(r) => format!("cmp byte [{}], 0", r),
            Self::JumpZero(n) => format!("jz {}", n),
            Self::JumpNonZero(n) => format!("jnz {}", n),
            Self::Jump(n) => format!("jmp {}", n),
            Self::Label(n) => format!("{}:", n),
            Self::Data(n, v) => format!("{}: db {}", n, format_data(v)),
        }
    }

    /// Whether this instruction affects the zero flag
    pub fn affects_zero_flag(&self) -> bool {
        self.effects().map_or(false, |e| e.flags)
    }

    /// Does this instruction use zero flag?
    pub fn reads_zf(&self) -> bool {
        match self {
            Self::BlackBox(_, _) => true,
            Self::NamedBlackBox(_, _, _) => true,
            Self::MovImm(_, _) => false,
            Self::MovImmVar(_, _) => false,
            Self::Mov(_, _) => false,
            Self::MovPtr8Imm(_, _) => false,
            Self::MovPtr16Imm(_, _) => false,
            Self::MovPtr32Imm(_, _) => false,
            Self::MovPtr64Imm(_, _) => false,
            Self::AddImm(_, 0) => false,
            Self::SubImm(_, 0) => false,
            Self::AddImm(_, _) => false,
            Self::SubImm(_, _) => false,
            Self::AddPtr8Imm(_, 0) => false,
            Self::AddPtr16Imm(_, 0) => false,
            Self::AddPtr32Imm(_, 0) => false,
            Self::AddPtr64Imm(_, 0) => false,
            Self::AddPtr8Imm(_, _) => false,
            Self::AddPtr16Imm(_, _) => false,
            Self::AddPtr32Imm(_, _) => false,
            Self::AddPtr64Imm(_, _) => false,
            Self::IsZero(_) => false,
            Self::IsZeroPtr8(_) => false,
            Self::JumpZero(_) => true,
            Self::JumpNonZero(_) => true,
            Self::Jump(_) => false,
            Self::Label(_) => false,
            Self::Data(_, _) => false,
        }
    }

    /// Returns none for static data, as it must not be executed
    pub fn effects(&self) -> Option<Effects> {
        Some(match self {
            Self::BlackBox(_, e) => *e,
            Self::NamedBlackBox(_, _, e) => *e,
            Self::MovImm(_, _) => Effects::REG,
            Self::MovImmVar(_, _) => Effects::REG,
            Self::Mov(_, _) => Effects::REG,
            Self::MovPtr8Imm(_, _) => Effects::REG,
            Self::MovPtr16Imm(_, _) => Effects::REG,
            Self::MovPtr32Imm(_, _) => Effects::REG,
            Self::MovPtr64Imm(_, _) => Effects::REG,
            Self::AddImm(_, 0) => Effects::FLAG,
            Self::SubImm(_, 0) => Effects::FLAG,
            Self::AddImm(_, _) => Effects::ARITHMETIC,
            Self::SubImm(_, _) => Effects::ARITHMETIC,
            Self::AddPtr8Imm(_, 0) => Effects::FLAG,
            Self::AddPtr16Imm(_, 0) => Effects::FLAG,
            Self::AddPtr32Imm(_, 0) => Effects::FLAG,
            Self::AddPtr64Imm(_, 0) => Effects::FLAG,
            Self::AddPtr8Imm(_, _) => Effects::ARITHMETIC,
            Self::AddPtr16Imm(_, _) => Effects::ARITHMETIC,
            Self::AddPtr32Imm(_, _) => Effects::ARITHMETIC,
            Self::AddPtr64Imm(_, _) => Effects::ARITHMETIC,
            Self::IsZero(_) => Effects::FLAG,
            Self::IsZeroPtr8(_) => Effects::FLAG,
            Self::JumpZero(_) => Effects::JUMP,
            Self::JumpNonZero(_) => Effects::JUMP,
            Self::Jump(_) => Effects::JUMP,
            Self::Label(_) => Effects::LABEL, // Jump can end here
            Self::Data(_, _) => {
                return None;
            },
        })
    }

    /// Combines two instructions into one if possible
    pub fn combine(self, other: Self) -> Vec<Self> {
        use Instruction::*;
        if let AddPtr8Imm(r0, v0) = self.clone() {
            if let AddPtr8Imm(r1, v1) = other.clone() {
                if r0 == r1 {
                    return vec![AddPtr8Imm(r0, v0.wrapping_add(v1))];
                }
            } else if let MovPtr8Imm(r1, v1) = other.clone() {
                if r0 == r1 {
                    return vec![MovPtr8Imm(r0, v1)];
                }
            }
        } else if let MovPtr8Imm(r0, v0) = self.clone() {
            if let AddPtr8Imm(r1, v1) = other.clone() {
                if r0 == r1 {
                    return vec![MovPtr8Imm(r0, v0.wrapping_add(v1))];
                }
            }
        } else if let AddImm(r0, v0) = self.clone() {
            if let AddImm(r1, v1) = other.clone() {
                if r0 == r1 {
                    return vec![AddImm(r0, v0.wrapping_add(v1))];
                }
            } else if let SubImm(r1, v1) = other.clone() {
                if r0 == r1 {
                    if v0 == v1 {
                        return Vec::new();
                    } else if v0 < v1 {
                        return vec![SubImm(r0, v1 - v0)];
                    } else {
                        return vec![AddImm(r0, v0 - v1)];
                    }
                }
            }
        } else if let SubImm(r0, v0) = self.clone() {
            if let AddImm(r1, _) = other.clone() {
                if r0 == r1 {
                    return other.combine(self);
                }
            } else if let SubImm(r1, v1) = other.clone() {
                if r0 == r1 {
                    return vec![SubImm(r0, v0.wrapping_add(v1))];
                }
            }
        } else if let JumpZero(target) = self.clone() {
            if let JumpZero(_) = other.clone() {
                return vec![JumpZero(target)];
            }
        } else if let JumpNonZero(target) = self.clone() {
            if let JumpNonZero(_) = other.clone() {
                return vec![JumpNonZero(target)];
            }
        }
        vec![self, other]
    }
}
impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_source())
    }
}
