#![allow(clippy::needless_pass_by_value)]

use std::collections::{HashMap, HashSet};

use super::instruction::{Effects, Instruction, Register64};

/// Removes redundant movs
pub fn optimize_redundant_movs(ops: Vec<Instruction>) -> Vec<Instruction> {
    use Instruction::*;
    let mut last_known: HashMap<Register64, u64> = HashMap::new();
    let mut result = Vec::new();
    for op in ops {
        let mut include_this = true; // Will Set to false to remove item
        if let MovImm(r, imm) = op {
            if last_known.get(&r) == Some(&imm) {
                include_this = false;
            }
        } else if let Mov(r1, r2) = op {
            if let Some(v) = last_known.get(&r1) {
                if Some(v) == last_known.get(&r2) {
                    include_this = false;
                }
            }
        }
        if include_this {
            result.push(op.clone());
        }

        // Update last_kwown table
        match op {
            BlackBox(_, _) | NamedBlackBox(_, _, _) => {
                last_known.clear();
            },
            Mov(r, r2) => {
                if let Some(v) = last_known.clone().get(&r2) {
                    last_known.insert(r, *v);
                } else {
                    last_known.remove(&r);
                }
            },
            MovImm(r, imm) => {
                last_known.insert(r, imm);
            },
            AddImm(r, _) | SubImm(r, _) => {
                // before jump target labels.

                last_known.remove(&r);
            },
            Label(_) => {
                last_known.clear();
            },
            _ => {},
        }
    }
    result
}

/// Combines adjancent instructions
pub fn optimize_adjacent(ops: Vec<Instruction>) -> Vec<Instruction> {
    ops.into_iter()
        .fold(Vec::new(), |a: Vec<Instruction>, b: Instruction| {
            let mut result = a.clone();
            if let Some(last) = result.pop() {
                result.extend(last.combine(b));
                result
            } else {
                vec![b]
            }
        })
}

/// Combines adjancent immediate memory moves
pub fn optimize_adjancent_mem_movs(ops: Vec<Instruction>) -> Vec<Instruction> {
    use Instruction::*;
    let mut result = Vec::new();
    let mut index: usize = 0;
    while index < ops.len() {
        if let MovPtr8Imm(r0, imm) = ops[index] {
            let mut imms = vec![imm];
            while index + imms.len() < ops.len() {
                if let MovPtr8Imm(r1, imm) = ops[index + imms.len()] {
                    if r0 != r1 {
                        break;
                    }
                    imms.push(imm);
                } else {
                    break;
                }
            }

            if imms.len() > 1 {
                imms.truncate(8);
                while !imms.len().is_power_of_two() {
                    imms.pop();
                }
                let bytes = imms.len();
                let mut orred: u64 = 0;
                // Reversed as x86 is little-endian
                for imm in imms.into_iter().rev() {
                    orred = (orred << 8) | u64::from(imm);
                }
                result.push(match bytes {
                    2 => MovPtr16Imm(r0, orred as u16),
                    4 => MovPtr32Imm(r0, orred as u32),
                    8 => MovPtr64Imm(r0, orred),
                    _ => unreachable!(),
                });
                result.push(AddImm(r0, bytes as u64));
                index += bytes;
                continue;
            }
        }

        result.push(ops[index].clone());
        index += 1;
    }
    result
}

/// If code begins with setting the first cell to value, use mov instead of add
pub fn optimize_start_cells(mut ops: Vec<Instruction>) -> Vec<Instruction> {
    use Instruction::*;
    let mut index = 0;
    while index < ops.len() {
        if let AddPtr8Imm(r0, imm) = ops[index].clone() {
            if imm == 0 {
                ops.remove(0);
                continue;
            } else {
                ops[index] = MovPtr8Imm(r0, imm);
            }
        } else if let AddImm(_, _) = ops[index] {
        } else {
            break;
        }

        index += 1;
    }
    ops
}

/// Zeroing loop: `[+]` or `[-]`
pub fn optimize_zero_loop(ops: Vec<Instruction>) -> Vec<Instruction> {
    use Instruction::*;
    let mut result = Vec::new();
    let mut index: usize = 0;
    while index < ops.len() {
        if index + 2 < ops.len() {
            if let JumpNonZero(label) = ops[index + 2].clone() {
                if let AddPtr8Imm(r, 1) | AddPtr8Imm(r, 255) = ops[index + 1] {
                    if ops[index] == Label(label) {
                        result.push(MovPtr8Imm(r, 0));
                        index += 3;
                        continue;
                    }
                }
            }
        }

        result.push(ops[index].clone());
        index += 1;
    }
    result
}

/// Constant output cycle used by the startup optimizer etc
pub fn optimize_constant_output(ops: Vec<Instruction>) -> Vec<Instruction> {
    use Instruction::*;

    let mut name_label: usize = 0;
    macro_rules! get_label {
        () => {{
            let label = format!("constant_output{}", name_label);
            name_label += 1;
            label
        }};
    }

    let mut result = Vec::new();
    let mut index: usize = 0;
    let mut current_bytes = Vec::new();
    let mut const_strings = Vec::new();
    let mut write_fn: Option<Instruction> = None;
    while index < ops.len() {
        if index + 4 < ops.len() {
            if let MovPtr8Imm(r0, imm) = ops[index] {
                if MovImm(Register64::rdi, 1) == ops[index + 1]
                    && Mov(Register64::rsi, r0) == ops[index + 2]
                    && MovImm(Register64::rdx, 1) == ops[index + 3]
                {
                    if let NamedBlackBox(name, f, eff) = ops[index + 4].clone() {
                        if name == "write" {
                            let bb = BlackBox(f, eff);
                            if let Some(wf) = write_fn.clone() {
                                debug_assert_eq!(wf, bb);
                            } else {
                                write_fn = Some(bb);
                            }
                            current_bytes.push(imm);
                            index += 5;
                            continue;
                        }
                    }
                }
            }
        }

        if !current_bytes.is_empty() {
            let name = get_label!();

            result.push(MovImm(Register64::rdi, 1));
            result.push(MovImmVar(Register64::rsi, name.clone()));
            result.push(MovImm(Register64::rdx, current_bytes.len() as u64));
            result.push(write_fn.clone().unwrap());

            const_strings.push(Data(name, current_bytes.clone()));
            current_bytes.clear();
        }

        result.push(ops[index].clone());
        index += 1;
    }
    result.extend(const_strings);
    result
}

/// Removes redundant cmp instructions where zero flag can was set by the previous instruction
pub fn optimize_zero_flags(ops: Vec<Instruction>) -> Vec<Instruction> {
    use Instruction::*;
    let mut result = Vec::new();
    let mut index: usize = 0;
    while index < ops.len() {
        if let IsZeroPtr8(r0) = ops[index] {
            let mut i: usize = 1;
            while i < index {
                if ops[index - i].affects_zero_flag() {
                    break;
                }
                i += 1;
            }
            if i >= 1 {
                if IsZeroPtr8(r0) == ops[index - i] {
                    index += 1;
                    continue;
                } else if let AddPtr8Imm(r1, _) = ops[index - i] {
                    if r0 == r1 {
                        index += 1;
                        continue;
                    } else if let NamedBlackBox(name, _, _) = &ops[index - i] {
                        if name == "read" {
                            index += 1;
                            continue;
                        }
                    }
                }
            }
        }

        result.push(ops[index].clone());
        index += 1;
    }
    result
}

/// Removes redundant instructions just before exit is called
pub fn optimize_exit(mut ops: Vec<Instruction>) -> Vec<Instruction> {
    use Instruction::*;
    let mut index: usize = 0;
    'outer: while index < ops.len() {
        // Test if this instruction can be removed
        if let Label(_) = ops[index] {
            index += 1;
            continue;
        } else if let Some(eff) = ops[index].effects() {
            if eff.control_flow || eff.io {
                index += 1;
                continue;
            }
        } else {
            index += 1;
            continue;
        }

        let mut offset: usize = 1;
        // Scan forward until exit or dependency,
        while index + offset < ops.len() {
            if let NamedBlackBox(name, _, _) = &ops[index + offset] {
                if name == "exit" {
                    // Preserve one instruction before exit, as that sets the exit code
                    if offset > 1 {
                        ops.remove(index);
                        continue 'outer;
                    } else {
                        debug_assert_eq!(MovImm(Register64::rdi, 0), ops[index + offset - 1]);
                    }
                }
            }

            // If side effects are found, this instruction cannot be removed
            if let Some(eff) = ops[index + offset].effects() {
                if eff.control_flow || eff.io {
                    break;
                }
            }

            offset += 1;
        }
        index += 1;
    }
    ops
}

/// Removes dead code, i.e. unconditional jumps over sections
pub fn optimize_remove_dead_code(ops: Vec<Instruction>) -> Vec<Instruction> {
    use Instruction::*;
    let mut result = Vec::new();
    let mut index: usize = 0;
    while index < ops.len() {
        if let Jump(l0) = &ops[index] {
            let mut ok = true;
            let mut i: usize = 1;
            while index + i < ops.len() {
                if let Label(l1) = &ops[index + i] {
                    ok = l0 == l1;
                    break;
                }
                i += 1;
            }

            if ok && i > 1 {
                index += i;
                continue;
            }
        }

        result.push(ops[index].clone());
        index += 1;
    }
    result
}

/// Removes unused labels
pub fn optimize_remove_unused_labels(ops: Vec<Instruction>) -> Vec<Instruction> {
    use Instruction::*;

    let mut used_labels = HashSet::new();
    for op in &ops {
        if let Jump(l) | JumpZero(l) | JumpNonZero(l) = op {
            used_labels.insert(l.clone());
        }
    }

    let mut result = Vec::new();
    let mut index: usize = 0;
    while index < ops.len() {
        if let Label(l) = &ops[index] {
            if !used_labels.contains(l) {
                index += 1;
                continue;
            }
        }

        result.push(ops[index].clone());
        index += 1;
    }
    result
}

/// Removes instructions that cause no effects
pub fn optimize_remove_nops(mut ops: Vec<Instruction>) -> Vec<Instruction> {
    let mut index: usize = 0;
    while index < ops.len() {
        if let Some(efs) = ops[index].effects() {
            let mut required = true;
            if efs == Effects::NOP {
                required = false;
            } else if efs.flags && !(efs.registers || efs.control_flow) {
                // Test if the flags are overwritten before next instruction that uses them.
                // Note that the compiler currently makes almost no assumptions about events
                // before jump target labels.
                required = false; // Switch default for the loop
                let mut i: usize = 1;
                while index + i < ops.len() {
                    if ops[index + i].reads_zf() {
                        required = true;
                        break;
                    } else if let Some(e) = ops[index + i].effects() {
                        if e.flags {
                            // Next effect shadows flag changes
                            break;
                        }
                    }
                    i += 1;
                }
            }
            if !required {
                ops.remove(index);
                continue;
            }
        }
        index += 1;
    }
    ops
}

/// Removes jumps that are never followed
pub fn optimize_dead_jumps(mut ops: Vec<Instruction>) -> Vec<Instruction> {
    use Instruction::*;
    let mut index: usize = 0;
    'outer: while index < ops.len() {
        if let JumpZero(_) = ops[index].clone() {
            let mut neg_offset: usize = 1;
            while index > neg_offset {
                // If flags are set and there are no jumps between these, then this jump is required
                if ops[index - neg_offset].effects().map_or(false, |e| e.flags) {
                    break;
                } else if let JumpZero(_) = ops[index - neg_offset] {
                    ops.remove(index);
                    continue 'outer;
                }
                neg_offset += 1;
            }
        } else if let JumpNonZero(_) = ops[index].clone() {
            let mut neg_offset: usize = 1;
            while index > neg_offset {
                // If flags are set and there are no jumps between these, then this jump is required
                if ops[index - neg_offset].effects().map_or(false, |e| e.flags) {
                    break;
                } else if let JumpNonZero(_) = ops[index - neg_offset] {
                    ops.remove(index);
                    continue 'outer;
                }
                neg_offset += 1;
            }
        }
        index += 1;
    }
    ops
}

pub fn label_index(ops: &[Instruction], label: &str) -> usize {
    let t = Instruction::Label(label.to_owned());
    for (i, op) in ops.iter().cloned().enumerate() {
        if op == t {
            return i;
        }
    }
    unreachable!("Label doesn't exist");
}

/// Jumps directly over check if negation of condition is check after jump
pub fn optimize_jump_skip_recheck(mut ops: Vec<Instruction>) -> Vec<Instruction> {
    use Instruction::*;

    let mut next_label: usize = 0;
    macro_rules! get_label {
        () => {{
            let label = format!(".jump_skip_recheck{}", next_label);
            next_label += 1;
            label
        }};
    }

    let mut index: usize = 1;
    while index < ops.len() {
        if let IsZeroPtr8(r) = ops[index - 1].clone() {
            if let JumpZero(label) = ops[index].clone() {
                let li = label_index(&ops, &label);
                if li + 2 < ops.len() && IsZeroPtr8(r) == ops[li + 1] {
                    if let JumpNonZero(_) = ops[li + 2].clone() {
                        let new_label = get_label!();
                        ops[index] = JumpZero(new_label.clone());
                        ops.insert(li + 3, Label(new_label));
                        if li < index {
                            debug_assert!(li + 3 < index);
                            index += 1;
                        }
                    }
                }
            } else if let JumpNonZero(label) = ops[index].clone() {
                let li = label_index(&ops, &label);
                if li + 2 < ops.len() && IsZeroPtr8(r) == ops[li + 1] {
                    if let JumpZero(_) = ops[li + 2].clone() {
                        let new_label = get_label!();
                        ops[index] = JumpNonZero(new_label.clone());
                        ops.insert(li + 3, Label(new_label));
                        if li < index {
                            debug_assert!(li + 3 < index);
                            index += 1;
                        }
                    }
                }
            }
        }
        index += 1;
    }
    ops
}

/// Separates instructions and data
pub fn separate_data(mut ops: Vec<Instruction>) -> (Vec<Instruction>, Vec<Instruction>) {
    use Instruction::*;
    let mut data: Vec<Instruction> = Vec::new();
    let mut index: usize = 0;
    while index < ops.len() {
        if let Data(_, _) = ops[index] {
            data.push(ops.remove(index));
            continue;
        }
        index += 1;
    }
    data.sort();
    (ops, data)
}

/// Moves data instructions to the end of data buffer
pub fn move_data_to_end(ops: Vec<Instruction>) -> Vec<Instruction> {
    let (mut ops, data) = separate_data(ops);
    ops.extend(data.into_iter());
    ops
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pass {
    /// Name of the pass
    name: String,
    /// Actual function
    function: fn(Vec<Instruction>) -> Vec<Instruction>,
    /// List of passes to be executed immediately after this
    cleanup: Vec<PassId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PassId(usize);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Optimizer {
    /// Passes
    passes: Vec<Pass>,
}
impl Optimizer {
    pub fn new() -> Self {
        Self { passes: Vec::new() }
    }

    pub fn add_pass(&mut self, pass: Pass) -> PassId {
        if self.passes.iter().any(|p| p.name == pass.name) {
            panic!("Pass named {} already exists", pass.name);
        }
        self.passes.push(pass);
        PassId(self.passes.len() - 1)
    }

    pub fn get_id(&self, name: &str) -> PassId {
        self.passes
            .iter()
            .enumerate()
            .find_map(|(i, p)| if p.name == name { Some(PassId(i)) } else { None })
            .unwrap_or_else(|| panic!("Pass {} not defined yet", name))
    }

    pub fn get(&self, id: PassId) -> Pass {
        self.passes[id.0].clone()
    }
}

/// Removes redundant movs
pub fn optimize(mut ops: Vec<Instruction>) -> Vec<Instruction> {
    let mut optimizer = Optimizer::new();

    macro_rules! pass {
        ($optimizer:ident; $name:ident; $($cleanup:ident),*) => {
            $optimizer.add_pass(Pass {
                name: stringify!($name).to_owned(),
                function: $name,
                cleanup: vec![$(optimizer.get_id(stringify!($cleanup)),)*],
            })
        };
        ($optimizer:ident; $name:ident) => {pass!($optimizer; $name;)};
    };

    pass!(optimizer; optimize_remove_unused_labels);
    pass!(optimizer; optimize_start_cells; optimize_remove_unused_labels);
    pass!(optimizer; optimize_zero_loop);
    pass!(optimizer; optimize_zero_flags; optimize_remove_unused_labels);
    pass!(optimizer; optimize_remove_nops; optimize_remove_unused_labels);
    pass!(optimizer; optimize_adjacent; optimize_remove_nops);
    pass!(optimizer; optimize_adjancent_mem_movs; optimize_remove_nops, optimize_zero_loop, optimize_adjacent);
    pass!(optimizer; optimize_constant_output);
    pass!(optimizer; optimize_dead_jumps; optimize_remove_unused_labels, optimize_remove_nops);
    pass!(optimizer; optimize_jump_skip_recheck; optimize_remove_unused_labels, optimize_dead_jumps);
    pass!(optimizer; optimize_remove_dead_code; optimize_remove_unused_labels, optimize_remove_nops);
    pass!(optimizer; optimize_exit; optimize_remove_unused_labels, optimize_dead_jumps, optimize_zero_flags, optimize_remove_nops);

    let mut queue: Vec<_> = optimizer.passes.iter().cloned().rev().collect();
    while let Some(pass) = queue.pop() {
        log::trace!("Optimization: {}", pass.name);
        ops = (pass.function)(ops);
        ops = move_data_to_end(ops);
        for pass_id in pass.cleanup {
            let p = optimizer.get(pass_id);
            if queue.last() != Some(&p) {
                queue.push(p);
            }
        }
    }
    ops
}

// TODO: Future optimizations:
// `[<]` and `[>]` to scan loops
// `,[>,]` to read until EOF
// `[.>]` to print null-terminated string, i.e. scan and print

// dec rbx
// inc byte [rbx]
// inc rbx
// dec byte [rbx]
// TO
// inc byte [rbx - 1]
// inc byte [rbx]
