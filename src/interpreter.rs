#![allow(clippy::new_without_default)]

use crate::parser::Token;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interpreter {
    cells: Vec<u8>,
    pointer: usize,
}
impl Interpreter {
    pub fn new() -> Self {
        Self {
            cells: vec![0],
            pointer: 0,
        }
    }

    #[must_use]
    fn step(&mut self, token: Token, io: &mut dyn IO) -> Mode {
        println!(
            "s: {:?} | {:?}",
            self.cells
                .iter()
                .enumerate()
                .map(|(i, v)| format!("{}{}", if i == self.pointer { "*" } else { "" }, v))
                .collect::<Vec<_>>()
                .join(", "),
            token
        );

        match token {
            Token::Next => {
                self.pointer += 1;
                if self.pointer == self.cells.len() {
                    self.cells.push(0);
                }
            },
            Token::Prev => {
                assert!(self.pointer != 0);
                self.pointer -= 1
            },
            Token::Increment | Token::Decrement => {
                self.cells[self.pointer] = if token == Token::Increment {
                    self.cells[self.pointer].wrapping_add(1)
                } else {
                    self.cells[self.pointer].wrapping_sub(1)
                };
            },
            Token::Output => io.write(self.cells[self.pointer]),
            Token::Input => self.cells[self.pointer] = io.read(),
            Token::JumpForwards => {
                if self.cells[self.pointer] == 0 {
                    return Mode::ScrollForwards;
                }
            },
            Token::JumpBackwards => {
                if self.cells[self.pointer] != 0 {
                    return Mode::ScrollBackwards;
                }
            },
        }
        Mode::Normal
    }

    /// Requires that tokens contains balanced brackets
    pub fn run(&mut self, tokens: &[Token], io: &mut dyn IO) {
        // let mut executor = Executor {
        //     interpreter: self.clone(),
        //     index: 0,
        //     tokens: &tokens,
        // };

        // while !executor.done() {
        //     executor.step(io);
        // }

        let mut index: usize = 0;
        while index < tokens.len() {
            println!(
                "t: {}",
                tokens.iter().map(|t| format!("{}", t)).collect::<String>()
            );

            println!("   {}^", " ".repeat(index));

            let mode = self.step(tokens[index], io);
            if mode == Mode::Normal {
                index += 1;
                continue;
            }

            if mode == Mode::ScrollForwards {
                let mut level = 1;
                while level > 0 {
                    index += 1;
                    if tokens[index] == Token::JumpForwards {
                        level += 1;
                    } else if tokens[index] == Token::JumpBackwards {
                        level -= 1;
                    }
                }
            } else {
                let mut level = 1;
                while level > 0 {
                    index -= 1;
                    if tokens[index] == Token::JumpBackwards {
                        level += 1;
                    } else if tokens[index] == Token::JumpForwards {
                        level -= 1;
                    }
                }
            }
        }
    }
}

// pub struct Executor<'a> {
//     interpreter: Interpreter,
//     index: usize,
//     tokens: &'a [Token],
// }
// impl<'a> Executor<'a> {
//     #[must_use]
//     #[inline]
//     pub fn done(&self) -> bool {
//         debug_assert!(self.index <= self.tokens.len());
//         self.index == self.tokens.len()
//     }

//     pub fn step(&mut self, io: &mut dyn IO) {
//         assert!(!self.done());

//         let mode = self.interpreter.step(self.tokens[self.index], io);
//         if mode == Mode::Normal {
//             self.index += 1;
//         } else if mode == Mode::ScrollForwards {
//             let mut level = 1;
//             while level > 0 {
//                 self.index += 1;
//                 if self.tokens[self.index] == Token::JumpForwards {
//                     level += 1;
//                 } else if self.tokens[self.index] == Token::JumpBackwards {
//                     level -= 1;
//                 }
//             }
//         } else {
//             let mut level = 1;
//             while level > 0 {
//                 self.index -= 1;
//                 if self.tokens[self.index] == Token::JumpBackwards {
//                     level += 1;
//                 } else if self.tokens[self.index] == Token::JumpForwards {
//                     level -= 1;
//                 }
//             }
//         }
//     }
// }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    ScrollForwards,
    ScrollBackwards,
}

pub trait IO {
    fn read(&mut self) -> u8;
    fn write(&mut self, value: u8);
}

/// All reads return zeros, writes stored
#[derive(Debug, Clone, PartialEq, Eq)]
struct ZeroIO {
    pub output: Vec<u8>,
}
impl ZeroIO {
    pub fn new() -> Self {
        Self { output: Vec::new() }
    }
}
impl IO for ZeroIO {
    fn read(&mut self) -> u8 {
        0
    }
    fn write(&mut self, value: u8) {
        self.output.push(value);
    }
}

#[cfg(test)]
mod tests {
    use super::{Interpreter, ZeroIO};
    use crate::parser::parse;

    #[test]
    fn test_simple() {
        let mut io = ZeroIO::new();
        Interpreter::new().run(&parse("+."), &mut io);
        assert_eq!(io.output, vec![1]);
    }

    #[test]
    fn test_add() {
        let mut io = ZeroIO::new();

        Interpreter::new().run(&parse("++ > +++ < [->+<] > ."), &mut io);
        assert_eq!(io.output, vec![5]);
    }

    #[test]
    fn test_hello_world() {
        let mut io = ZeroIO::new();

        Interpreter::new().run(
            &parse(
                r"++++++++[>++++[>++>+++>+++
                >+<<<<-]>+>+>->>+[<]<-]>>.>-
                --.+++++++..+++.>>.<-.<.+++.
                ------.--------.>>+.>++.",
            ),
            &mut io,
        );
        assert_eq!(io.output, b"Hello World!\n");
    }
}
