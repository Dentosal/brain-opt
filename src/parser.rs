use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Token {
    Next,
    Prev,
    Increment,
    Decrement,
    Output,
    Input,
    JumpForwards,
    JumpBackwards,
}
impl Token {
    pub fn parse(c: char) -> Option<Self> {
        Some(match c {
            '>' => Self::Next,
            '<' => Self::Prev,
            '+' => Self::Increment,
            '-' => Self::Decrement,
            '.' => Self::Output,
            ',' => Self::Input,
            '[' => Self::JumpForwards,
            ']' => Self::JumpBackwards,
            _ => return None,
        })
    }
}
impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            Self::Next => '>',
            Self::Prev => '<',
            Self::Increment => '+',
            Self::Decrement => '-',
            Self::Output => '.',
            Self::Input => ',',
            Self::JumpForwards => '[',
            Self::JumpBackwards => ']',
        })
    }
}

pub fn parse(s: &str) -> Vec<Token> {
    let mut result = Vec::new();
    for c in s.chars() {
        if let Some(token) = Token::parse(c) {
            result.push(token);
        }
    }

    // check bracket balance
    let mut level: usize = 0;
    for r in result.iter().copied() {
        if r == Token::JumpForwards {
            level += 1;
        } else if r == Token::JumpBackwards {
            if level == 0 {
                panic!("Unbalanced ']'");
            }
            level -= 1;
        }
    }
    if level != 0 {
        panic!("Unbalanced '['");
    }

    result
}

#[cfg(test)]
mod tests {
    use super::{parse, Token};

    #[test]
    fn test_parse() {
        assert_eq!(parse("[->+<?]"), vec![
            Token::JumpForwards,
            Token::Decrement,
            Token::Next,
            Token::Increment,
            Token::Prev,
            Token::JumpBackwards,
        ]);
    }
}
