use std::fmt;
use std::ops::Index;

use crate::codegen::{self, Effects, Instruction, Register64};
use crate::parser::Token;
use crate::target_abi::{self, LinkerInfo, ABI};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Label(pub usize);
impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, ".label{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    scope: Vec<(Label, Label)>,
    next_label: Label,
    steps: Vec<Step>,
}
impl State {
    pub fn new() -> Self {
        Self {
            scope: Vec::new(),
            next_label: Label(0),
            steps: Vec::new(),
        }
    }

    fn get_label(&mut self) -> Label {
        let result = self.next_label;
        self.next_label = Label(self.next_label.0 + 1);
        result
    }

    pub fn append(&mut self, token: Token) {
        match token {
            Token::Next => self.steps.push(Step::Next(1)),
            Token::Prev => self.steps.push(Step::Prev(1)),
            Token::Increment => self.steps.push(Step::Add(1)),
            Token::Decrement => self.steps.push(Step::Add(255)),
            Token::Output => self.steps.push(Step::Output),
            Token::Input => self.steps.push(Step::Input),
            Token::JumpForwards => {
                let source_label = self.get_label();
                let target_label = self.get_label();
                self.scope.push((source_label, target_label));
                self.steps.push(Step::JumpToIf(false, target_label));
                self.steps.push(Step::Label(source_label));
            },
            Token::JumpBackwards => {
                let (source_label, target_label) = self.scope.pop().unwrap();
                self.steps.push(Step::JumpToIf(true, source_label));
                self.steps.push(Step::Label(target_label));
            },
        }
    }

    /// Simple peephole instruction combinator
    fn combine(a: Step, b: Step) -> Vec<Step> {
        if let Step::Add(v0) = a {
            if let Step::Add(v1) = b {
                vec![Step::Add(v0.wrapping_add(v1))]
            } else {
                vec![a, b]
            }
        } else if let Step::Next(v0) = a {
            if let Step::Next(v1) = b {
                vec![Step::Next(v0.wrapping_add(v1))]
            } else if let Step::Prev(v1) = b {
                if v0 == v1 {
                    vec![]
                } else if v0 > v1 {
                    vec![Step::Next(v0 - v1)]
                } else {
                    vec![Step::Prev(v1 - v0)]
                }
            } else {
                vec![a, b]
            }
        } else if let Step::Prev(v0) = a {
            if let Step::Prev(v1) = b {
                vec![Step::Prev(v0.checked_add(v1).unwrap())]
            } else if let Step::Next(v1) = b {
                if v0 == v1 {
                    vec![]
                } else if v0 < v1 {
                    vec![Step::Next(v0 - v1)]
                } else {
                    vec![Step::Prev(v1 - v0)]
                }
            } else {
                vec![a, b]
            }
        } else {
            vec![a, b]
        }
    }

    /// Simple peephole optimization pass
    fn optimize_peephole_combine(&mut self) {
        let mut index: usize = 0;
        while index + 1 < self.steps.len() {
            let a = self.steps.remove(index);
            let b = self.steps.remove(index);
            let c = Self::combine(a, b);
            for (i, v) in c.iter().copied().enumerate() {
                self.steps.insert(index + i, v);
            }
            if vec![a, b] == c {
                index += 1;
            }
        }
    }

    /// Runs programs until some input is required.
    /// This also fully reduces programs with no input.
    fn optimize_startup(&mut self) {
        let mut intp = StepInterpreter {
            steps: &self.steps,
            state: StepInterpreterState {
                index: 0,
                tape: Tape::new(),
                pointer: 0,
                output: Vec::new(),
            },
        };
        while !intp.done() {
            if !intp.step() {
                break;
            }
        }
        if intp.done() {
            // Whole execution complete, the program takes no input,
            // so just print the correct output and exit
            let end_state = intp.state;
            let mut new_steps = Vec::new();

            // Print initial output
            for v in end_state.output {
                // Output value
                new_steps.push(Step::Add(v));
                new_steps.push(Step::Output);
                // Zero cell
                let label_zero = self.get_label();
                new_steps.push(Step::Label(label_zero));
                new_steps.push(Step::Add(1));
                new_steps.push(Step::JumpToIf(true, label_zero));
            }
            self.steps = new_steps;
        } else {
            intp.state.tape.trim();
            let end_state = intp.state;
            let mut new_steps = Vec::new();

            // Print initial output
            for v in end_state.output {
                // Output value
                new_steps.push(Step::Add(v));
                new_steps.push(Step::Output);
                // Zero cell
                let label_zero = self.get_label();
                new_steps.push(Step::Label(label_zero));
                new_steps.push(Step::Add(1));
                new_steps.push(Step::JumpToIf(true, label_zero));
            }

            // Insert tape contents
            let tape_len = end_state.tape.0.len();
            for v in end_state.tape.0 {
                new_steps.push(Step::Add(v));
                new_steps.push(Step::Next(1));
            }

            // Adjust tape pointer
            if tape_len > end_state.pointer {
                new_steps.push(Step::Prev((tape_len - end_state.pointer) as u64));
            } else if tape_len < end_state.pointer {
                new_steps.push(Step::Next((end_state.pointer - tape_len) as u64));
            }

            // Jump to proper position in code to continue
            if end_state.index != 0 {
                let label_zero = self.get_label();
                self.steps.insert(end_state.index, Step::Label(label_zero));
                self.steps.insert(0, Step::JumpTo(label_zero));
            }

            new_steps.extend(self.steps.iter());
            self.steps = new_steps;
        }
    }

    /// Run optimizations
    pub fn optimize(&mut self) {
        self.optimize_peephole_combine();
        self.optimize_startup();
    }

    pub fn to_assembly(&self, abi: ABI) -> String {
        let mut abi_ops = abi.operations();

        let ptr_reg = Register64::rbx;
        let steps: Vec<Instruction> = self
            .steps
            .iter()
            .flat_map(|x| x.to_assembly(ptr_reg, &mut *abi_ops))
            .collect();
        let startup: Vec<Instruction> = abi_ops.startup();
        let exit: Vec<Instruction> = abi_ops.exit();

        let body = codegen::optimize(
            startup
                .iter()
                .chain(steps.iter())
                .chain(exit.iter())
                .cloned()
                .collect(),
        );
        let (body, data) = codegen::separate_data(body);

        let header = vec![
            Instruction::BlackBox("sub rsp, $arraylen".to_owned(), Effects::VOLATILE),
            Instruction::BlackBox("mov rcx, $arraylen".to_owned(), Effects::VOLATILE),
            Instruction::BlackBox("mov rdi, rsp".to_owned(), Effects::VOLATILE),
            Instruction::BlackBox("xor al, al".to_owned(), Effects::VOLATILE),
            Instruction::BlackBox("rep stosb".to_owned(), Effects::VOLATILE),
            Instruction::BlackBox("mov $pointer, rsp".to_owned(), Effects::VOLATILE),
            Instruction::BlackBox("sub rsp, 8".to_owned(), Effects::VOLATILE),
        ];

        let s = format!(
            "{}\nsection .text\n$entrypoint:\n{}\n{}\nsection .data\n{}\n",
            abi.operations().linker_info().to_assembly(),
            header
                .iter()
                .map(Instruction::to_source)
                .collect::<Vec<_>>()
                .join("\n"),
            body.iter()
                .map(Instruction::to_source)
                .collect::<Vec<_>>()
                .join("\n"),
            if data.is_empty() {
                String::new()
            } else {
                data.iter()
                    .map(Instruction::to_source)
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        );
        s.replace("$entrypoint", &abi_ops.linker_info().entrypoint)
            .replace("$pointer", &format!("{}", ptr_reg))
            .replace("$arraylen", "30000")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Step {
    /// Move to right
    Next(u64),
    /// Move to left
    Prev(u64),
    /// Add to current cell (or subtract by overflowing)
    Add(u8),
    /// Unconditional jump to label
    JumpTo(Label),
    /// if bool == true, then jump on nonzero
    JumpToIf(bool, Label),
    /// Label meta-instruction
    Label(Label),
    /// Call to write function
    Output,
    /// Call to read function
    Input,
}
impl Step {
    fn to_assembly(self, pointer: Register64, abi_ops: &mut dyn target_abi::Operations) -> Vec<Instruction> {
        match self {
            Self::Next(count) => vec![Instruction::AddImm(pointer, count)],
            Self::Prev(count) => vec![Instruction::SubImm(pointer, count)],
            Self::Add(n) => vec![Instruction::AddPtr8Imm(pointer, n)],
            Self::JumpTo(label) => vec![Instruction::Jump(format!("{}", label))],
            Self::JumpToIf(condition, label) => vec![
                Instruction::IsZeroPtr8(pointer),
                if condition {
                    Instruction::JumpNonZero(format!("{}", label))
                } else {
                    Instruction::JumpZero(format!("{}", label))
                },
            ],
            Self::Label(label) => vec![Instruction::Label(format!("{}", label))],
            Self::Output => abi_ops.write_bytes(pointer, 1),
            Self::Input => abi_ops.read_byte(pointer),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StepInterpreterState {
    /// Step index
    index: usize,
    /// Tape
    tape: Tape,
    /// Tape pointer (index)
    pointer: usize,
    /// Output buffer
    output: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StepInterpreter<'a> {
    /// Instructions
    steps: &'a [Step],
    /// Current state
    state: StepInterpreterState,
}
impl<'a> StepInterpreter<'a> {
    #[must_use]
    #[inline]
    pub fn done(&self) -> bool {
        self.state.index == self.steps.len()
    }

    pub fn jump_to(&mut self, label: Label) {
        for (i, s) in self.steps.iter().enumerate() {
            if s == &Step::Label(label) {
                self.state.index = i;
                return;
            }
        }
        unreachable!("Missing label");
    }

    /// Returns true if next step can be ran without input
    #[must_use]
    pub fn step(&mut self) -> bool {
        use Step::*;
        debug_assert!(!self.done());
        match self.steps[self.state.index] {
            Next(n) => self.state.pointer = self.state.pointer.checked_add(n as usize).unwrap(),
            Prev(n) => self.state.pointer = self.state.pointer.checked_sub(n as usize).unwrap(),
            Add(n) => self.state.tape.add(self.state.pointer, n),
            JumpTo(label) => self.jump_to(label),
            JumpToIf(cond, label) => {
                if cond == (self.state.tape[self.state.pointer] != 0) {
                    self.jump_to(label);
                }
            },
            Label(_) => {},
            Output => self.state.output.push(self.state.tape[self.state.pointer]),
            Input => return false,
        }
        self.state.index += 1;
        true
    }
}

#[derive(Debug, Clone)]
struct Tape(Vec<u8>);
impl Tape {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn add(&mut self, index: usize, add: u8) {
        while self.0.len() <= index {
            self.0.push(0);
        }
        self.0[index] = self.0[index].wrapping_add(add);
    }

    pub fn trim(&mut self) {
        let mut len = self.0.len();
        while len > 0 && self.0[len - 1] == 0 {
            self.0.pop();
            len -= 1;
        }
    }
}
impl Index<usize> for Tape {
    type Output = u8;

    fn index(&self, i: usize) -> &Self::Output {
        self.0.get(i).unwrap_or(&0)
    }
}

impl PartialEq for Tape {
    fn eq(&self, other: &Self) -> bool {
        for i in 0..(self.0.len().max(other.0.len())) {
            if self.0.get(i).unwrap_or(&0) != other.0.get(i).unwrap_or(&0) {
                return false;
            }
        }
        true
    }
}
impl Eq for Tape {}

pub fn compile_tokens(tokens: Vec<Token>, abi: ABI) -> (String, LinkerInfo) {
    let mut state = State::new();
    for token in tokens {
        state.append(token);
    }
    state.optimize();
    let linker_info = abi.operations().linker_info();
    (state.to_assembly(abi), linker_info)
}
