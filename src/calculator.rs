
#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Number(f64),
    Plus,
    Minus,
    Multiply,
    Divide,
    LParen,
    RParen,
}

pub fn evaluate(input: &str) -> Result<f64, String> {
    let tokens = tokenize(input)?;
    if tokens.is_empty() {
        return Ok(0.0);
    }
    let mut parser = Parser::new(tokens);
    parser.parse_expression()
}

fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&c) = chars.peek() {
        match c {
            '0'..='9' | '.' => {
                let mut num_str = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_digit(10) || c == '.' {
                        num_str.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                let num = num_str.parse::<f64>().map_err(|_| "Invalid number")?;
                tokens.push(Token::Number(num));
            }
            '+' => { tokens.push(Token::Plus); chars.next(); }
            '-' => { tokens.push(Token::Minus); chars.next(); }
            '*' => { tokens.push(Token::Multiply); chars.next(); }
            '/' => { tokens.push(Token::Divide); chars.next(); }
            '(' => { tokens.push(Token::LParen); chars.next(); }
            ')' => { tokens.push(Token::RParen); chars.next(); }
            ws if ws.is_whitespace() => { chars.next(); }
            _ => return Err(format!("Invalid character: {}", c)),
        }
    }
    Ok(tokens)
}

struct Parser {
    tokens: std::vec::IntoIter<Token>,
    current: Option<Token>,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        let mut iter = tokens.into_iter();
        let current = iter.next();
        Self { tokens: iter, current }
    }

    fn advance(&mut self) {
        self.current = self.tokens.next();
    }

    fn parse_expression(&mut self) -> Result<f64, String> {
        let mut left = self.parse_term()?;

        while let Some(token) = &self.current {
            match token {
                Token::Plus => {
                    self.advance();
                    let right = self.parse_term()?;
                    left += right;
                }
                Token::Minus => {
                    self.advance();
                    let right = self.parse_term()?;
                    left -= right;
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_term(&mut self) -> Result<f64, String> {
        let mut left = self.parse_factor()?;

        while let Some(token) = &self.current {
            match token {
                Token::Multiply => {
                    self.advance();
                    let right = self.parse_factor()?;
                    left *= right;
                }
                Token::Divide => {
                    self.advance();
                    let right = self.parse_factor()?;
                    if right == 0.0 {
                        return Err("Division by zero".to_string());
                    }
                    left /= right;
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_factor(&mut self) -> Result<f64, String> {
         if let Some(token) = self.current.clone() { 
             match token {
                 Token::Number(n) => {
                     self.advance();
                     Ok(n)
                 }
                 Token::LParen => {
                     self.advance();
                     let val = self.parse_expression()?;
                     match self.current {
                         Some(Token::RParen) => {
                             self.advance();
                             Ok(val)
                         }
                         _ => Err("Expected ')'".to_string())
                     }
                 }
                 Token::Minus => {
                     self.advance();
                     let val = self.parse_factor()?;
                     Ok(-val)
                 }
                 _ => Err("Unexpected token".to_string())
             }
         } else {
             Err("Unexpected end of input".to_string())
         }
    }
}
