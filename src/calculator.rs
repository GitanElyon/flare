use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(f64),
    Identifier(String),
    Plus,
    Minus,
    Multiply,
    Divide,
    Power,
    LParen,
    RParen,
    Comma,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(f64),
    Variable(String),
    Binary(Box<Expr>, Op, Box<Expr>),
    Call(String, Vec<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
}

pub fn evaluate(input: &str) -> Result<String, String> {
    let tokens = tokenize(input)?;
    if tokens.is_empty() {
        return Ok("".to_string());
    }
    let mut parser = Parser::new(tokens);
    let ast = parser.parse_expression()?;
    
    // 1. Evaluate symbolic functions (diff, integrate, etc)
    let expanded = eval_functions(ast);
    // 2. Simplify (constant folding, identities)
    let simplified = simplify(expanded);
    
    Ok(format!("{}", simplified))
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Number(n) => write!(f, "{}", n),
            Expr::Variable(s) => write!(f, "{}", s),
            Expr::Binary(lhs, op, rhs) => {
                let op_str = match op {
                    Op::Add => "+",
                    Op::Sub => "-",
                    Op::Mul => "*",
                    Op::Div => "/",
                    Op::Pow => "^",
                };
                // Minimal parens logic could go here, for now just parens safely
                write!(f, "({} {} {})", lhs, op_str, rhs)
            }
            Expr::Call(name, args) => {
                write!(f, "{}(", name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
        }
    }
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
            'a'..='z' | 'A'..='Z' | '_' => {
                let mut ident = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        ident.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(Token::Identifier(ident));
            }
            '+' => { tokens.push(Token::Plus); chars.next(); }
            '-' => { tokens.push(Token::Minus); chars.next(); }
            '*' => { tokens.push(Token::Multiply); chars.next(); }
            '/' => { tokens.push(Token::Divide); chars.next(); }
            '^' => { tokens.push(Token::Power); chars.next(); }
            '(' => { tokens.push(Token::LParen); chars.next(); }
            ')' => { tokens.push(Token::RParen); chars.next(); }
            ',' => { tokens.push(Token::Comma); chars.next(); }
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

    fn parse_expression(&mut self) -> Result<Expr, String> {
        self.parse_add_sub()
    }

    fn parse_add_sub(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_mul_div()?;
        while let Some(token) = &self.current {
            match token {
                Token::Plus => { self.advance(); left = Expr::Binary(Box::new(left), Op::Add, Box::new(self.parse_mul_div()?)); }
                Token::Minus => { self.advance(); left = Expr::Binary(Box::new(left), Op::Sub, Box::new(self.parse_mul_div()?)); }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_mul_div(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_power()?;
        while let Some(token) = &self.current {
            match token {
                Token::Multiply => { self.advance(); left = Expr::Binary(Box::new(left), Op::Mul, Box::new(self.parse_power()?)); }
                Token::Divide => { self.advance(); left = Expr::Binary(Box::new(left), Op::Div, Box::new(self.parse_power()?)); }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_power(&mut self) -> Result<Expr, String> {
        let left = self.parse_primary()?;
        if let Some(Token::Power) = self.current {
             self.advance();
             let right = self.parse_power()?; // Right associative
             return Ok(Expr::Binary(Box::new(left), Op::Pow, Box::new(right)));
        }
        Ok(left)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        if let Some(token) = self.current.clone() {
            match token {
                Token::Number(n) => { self.advance(); Ok(Expr::Number(n)) }
                Token::Identifier(name) => {
                    self.advance();
                    if let Some(Token::LParen) = self.current {
                        self.advance();
                        let args = self.parse_args()?;
                        if let Some(Token::RParen) = self.current {
                            self.advance();
                            Ok(Expr::Call(name, args))
                        } else {
                            Err("Expected ')'".to_string())
                        }
                    } else {
                        Ok(Expr::Variable(name))
                    }
                }
                Token::LParen => {
                    self.advance();
                    let val = self.parse_expression()?;
                    if let Some(Token::RParen) = self.current {
                        self.advance();
                        Ok(val)
                    } else {
                        Err("Expected ')'".to_string())
                    }
                }
                Token::Minus => {
                    self.advance();
                    let val = self.parse_power()?;
                    // Represent unary minus as 0 - x
                    Ok(Expr::Binary(Box::new(Expr::Number(0.0)), Op::Sub, Box::new(val)))
                }
                _ => Err("Unexpected token".to_string())
            }
        } else {
            Err("Unexpected end of input".to_string())
        }
    }

    fn parse_args(&mut self) -> Result<Vec<Expr>, String> {
        let mut args = Vec::new();
        if let Some(Token::RParen) = self.current {
            return Ok(args);
        }
        
        args.push(self.parse_expression()?);
        while let Some(Token::Comma) = self.current {
            self.advance();
            args.push(self.parse_expression()?);
        }
        Ok(args)
    }
}

// --- Calculus & Functions ---

fn eval_functions(expr: Expr) -> Expr {
    match expr {
        Expr::Call(name, args) => {
            let processed_args: Vec<Expr> = args.into_iter().map(eval_functions).collect();
            match name.as_str() {
                "diff" => {
                    if processed_args.len() == 2 {
                         if let Expr::Variable(v) = &processed_args[1] {
                             return differentiate(processed_args[0].clone(), v);
                         }
                    }
                    Expr::Call(name, processed_args)
                }
                "integrate" => {
                    if processed_args.len() == 2 {
                        if let Expr::Variable(v) = &processed_args[1] {
                            return integrate(processed_args[0].clone(), v);
                        }
                    } else if processed_args.len() == 4 {
                         if let Expr::Variable(v) = &processed_args[1] {
                             // Definite integral
                             let antideriv = integrate(processed_args[0].clone(), v);
                             // F(b) - F(a)
                             let fb = substitute(antideriv.clone(), v, &processed_args[3]);
                             let fa = substitute(antideriv, v, &processed_args[2]);
                             return Expr::Binary(Box::new(fb), Op::Sub, Box::new(fa));
                         }
                    }
                    Expr::Call(name, processed_args)
                }
                "limit" => {
                     // limit(expr, var, to)
                     if processed_args.len() == 3 {
                         if let Expr::Variable(v) = &processed_args[1] {
                             let val = &processed_args[2];

                             return substitute(processed_args[0].clone(), v, val);
                         }
                     }
                     Expr::Call(name, processed_args)
                }
                "log" => {
                    if processed_args.len() == 2 {
                        // log(x, base) = ln(x) / ln(base)
                        let num = Expr::Call("ln".to_string(), vec![processed_args[0].clone()]);
                        let den = Expr::Call("ln".to_string(), vec![processed_args[1].clone()]);
                        return Expr::Binary(Box::new(num), Op::Div, Box::new(den));
                    } else if processed_args.len() == 1 {
                        if let Expr::Number(n) = processed_args[0] {
                            return Expr::Number(n.log10());
                        }
                    }
                    Expr::Call(name, processed_args)
                }
                // Handle basic math funcs that can be evaluated numerically if inputs are numbers
                "sqrt" | "ln" | "sin" | "cos" | "tan" | "abs" => {
                     if processed_args.len() == 1 {
                         if let Expr::Number(n) = processed_args[0] {
                             let res = match name.as_str() {
                                 "sqrt" => n.sqrt(),
                                 "ln" => n.ln(),
                                 "sin" => n.sin(),
                                 "cos" => n.cos(),
                                 "tan" => n.tan(),
                                 "abs" => n.abs(),
                                 _ => n 
                             };
                             return Expr::Number(res);
                         }
                     }
                     Expr::Call(name, processed_args)
                }
                _ => Expr::Call(name, processed_args)
            }
        }
        Expr::Binary(lhs, op, rhs) => Expr::Binary(Box::new(eval_functions(*lhs)), op, Box::new(eval_functions(*rhs))),
        _ => expr,
    }
}

fn differentiate(expr: Expr, var: &str) -> Expr {
    match expr {
        Expr::Number(_) => Expr::Number(0.0),
        Expr::Variable(name) => {
            if name == var { Expr::Number(1.0) } else { Expr::Number(0.0) }
        }
        Expr::Binary(lhs, op, rhs) => {
            match op {
                Op::Add => Expr::Binary(Box::new(differentiate(*lhs, var)), Op::Add, Box::new(differentiate(*rhs, var))),
                Op::Sub => Expr::Binary(Box::new(differentiate(*lhs, var)), Op::Sub, Box::new(differentiate(*rhs, var))),
                Op::Mul => {
                    // d(u*v) = u'v + uv'
                    let u = *lhs;
                    let v = *rhs;
                    let du = differentiate(u.clone(), var);
                    let dv = differentiate(v.clone(), var);
                    Expr::Binary(
                        Box::new(Expr::Binary(Box::new(du), Op::Mul, Box::new(v))),
                        Op::Add,
                        Box::new(Expr::Binary(Box::new(u), Op::Mul, Box::new(dv)))
                    )
                }
                Op::Div => {
                    // d(u/v) = (u'v - uv') / v^2
                    let u = *lhs;
                    let v = *rhs;
                    let du = differentiate(u.clone(), var);
                    let dv = differentiate(v.clone(), var);
                    let numerator = Expr::Binary(
                        Box::new(Expr::Binary(Box::new(du), Op::Mul, Box::new(v.clone()))),
                        Op::Sub,
                        Box::new(Expr::Binary(Box::new(u), Op::Mul, Box::new(dv)))
                    );
                    let denominator = Expr::Binary(Box::new(v), Op::Pow, Box::new(Expr::Number(2.0)));
                    Expr::Binary(Box::new(numerator), Op::Div, Box::new(denominator))
                }
                Op::Pow => {
                    // Power rule generalized: d(u^v) = u^v * (v'*ln(u) + v*u'/u)
                    // Simplified for u^n where n is number: n*u^(n-1) * u'
                    let u = *lhs;
                    let v = *rhs;
                    if let Expr::Number(n) = v {
                         let du = differentiate(u.clone(), var);
                         // n * u^(n-1) * du
                         let term1 = Expr::Binary(Box::new(Expr::Number(n)), Op::Mul, Box::new(Expr::Binary(Box::new(u), Op::Pow, Box::new(Expr::Number(n - 1.0)))));
                         Expr::Binary(Box::new(term1), Op::Mul, Box::new(du))
                    } else {
                        // Symbolic generalized power rule is complex, fallback
                         Expr::Call("diff".to_string(), vec![Expr::Binary(Box::new(u), Op::Pow, Box::new(v)), Expr::Variable(var.to_string())])
                    }
                }
            }
        }
        _ => Expr::Number(0.0) // Fallback
    }
}

fn integrate(expr: Expr, var: &str) -> Expr {
    match expr {
        Expr::Number(n) => Expr::Binary(Box::new(Expr::Number(n)), Op::Mul, Box::new(Expr::Variable(var.to_string()))),
        Expr::Variable(name) => {
            if name == var {
                // x -> x^2/2
                Expr::Binary(
                    Box::new(Expr::Binary(Box::new(Expr::Variable(name)), Op::Pow, Box::new(Expr::Number(2.0)))),
                    Op::Div,
                    Box::new(Expr::Number(2.0))
                )
            } else {
                // c -> c*x
                 Expr::Binary(Box::new(Expr::Variable(name)), Op::Mul, Box::new(Expr::Variable(var.to_string())))
            }
        }
        Expr::Binary(lhs, op, rhs) => {
            match op {
                Op::Add => Expr::Binary(Box::new(integrate(*lhs, var)), Op::Add, Box::new(integrate(*rhs, var))),
                Op::Sub => Expr::Binary(Box::new(integrate(*lhs, var)), Op::Sub, Box::new(integrate(*rhs, var))),
                Op::Mul => {
                     // Check for constant multiple: c * f(x)
                     if let Expr::Number(n) = *lhs {
                         return Expr::Binary(Box::new(Expr::Number(n)), Op::Mul, Box::new(integrate(*rhs, var)));
                     }
                      if let Expr::Number(n) = *rhs {
                         return Expr::Binary(Box::new(integrate(*lhs, var)), Op::Mul, Box::new(Expr::Number(n)));
                     }
                     Expr::Call("integrate".to_string(), vec![Expr::Binary(lhs, Op::Mul, rhs), Expr::Variable(var.to_string())])
                }
                Op::Pow => {
                    // x^n -> x^(n+1)/(n+1)
                    let u = *lhs;
                    let v = *rhs;
                    match (u, v) {
                        (Expr::Variable(ref name), Expr::Number(n)) if name == var => {
                            if n == -1.0 {
                                Expr::Call("ln".to_string(), vec![Expr::Variable(name.clone())])
                            } else {
                                Expr::Binary(
                                    Box::new(Expr::Binary(Box::new(Expr::Variable(name.clone())), Op::Pow, Box::new(Expr::Number(n + 1.0)))),
                                    Op::Div,
                                    Box::new(Expr::Number(n + 1.0))
                                )
                            }
                        }
                        (u, v) => Expr::Call("integrate".to_string(), vec![Expr::Binary(Box::new(u), Op::Pow, Box::new(v)), Expr::Variable(var.to_string())])
                    }
                }
                _ => Expr::Call("integrate".to_string(), vec![Expr::Binary(lhs, op, rhs), Expr::Variable(var.to_string())])
            }
        }
        _ => Expr::Call("integrate".to_string(), vec![expr, Expr::Variable(var.to_string())])
    }
}

fn substitute(expr: Expr, var: &str, val: &Expr) -> Expr {
    match expr {
        Expr::Variable(name) => {
            if name == var { val.clone() } else { Expr::Variable(name) }
        }
        Expr::Binary(lhs, op, rhs) => Expr::Binary(Box::new(substitute(*lhs, var, val)), op, Box::new(substitute(*rhs, var, val))),
        Expr::Call(name, args) => Expr::Call(name, args.into_iter().map(|arg| substitute(arg, var, val)).collect()),
         _ => expr
    }
}

// --- Simplification ---

fn simplify(expr: Expr) -> Expr {
    match expr {
        Expr::Binary(lhs, op, rhs) => {
            let s_lhs = simplify(*lhs);
            let s_rhs = simplify(*rhs);
            
            match (s_lhs.clone(), op, s_rhs.clone()) {
                // Constant folding
                (Expr::Number(a), _, Expr::Number(b)) => {
                    match op {
                        Op::Add => Expr::Number(a + b),
                        Op::Sub => Expr::Number(a - b),
                        Op::Mul => Expr::Number(a * b),
                        Op::Div => if b != 0.0 { Expr::Number(a / b) } else { Expr::Binary(Box::new(s_lhs), op, Box::new(s_rhs)) },
                        Op::Pow => Expr::Number(a.powf(b)),
                    }
                }
                // Mul Identity
                (_, Op::Mul, Expr::Number(n)) if n == 1.0 => s_lhs,
                (Expr::Number(n), Op::Mul, _) if n == 1.0 => s_rhs,
                (_, Op::Mul, Expr::Number(n)) if n == 0.0 => Expr::Number(0.0),
                (Expr::Number(n), Op::Mul, _) if n == 0.0 => Expr::Number(0.0),
                
                // Add Identity
                (_, Op::Add, Expr::Number(n)) if n == 0.0 => s_lhs,
                (Expr::Number(n), Op::Add, _) if n == 0.0 => s_rhs,
                
                // Sub
                (_, Op::Sub, Expr::Number(n)) if n == 0.0 => s_lhs,
                
                // Pow
                (_, Op::Pow, Expr::Number(n)) if n == 1.0 => s_lhs,
                (_, Op::Pow, Expr::Number(n)) if n == 0.0 => Expr::Number(1.0),

                // Div
                (_, Op::Div, Expr::Number(n)) if n == 1.0 => s_lhs,
                
                _ => Expr::Binary(Box::new(s_lhs), op, Box::new(s_rhs))
            }
        }
        Expr::Call(name, args) => Expr::Call(name, args.into_iter().map(simplify).collect()),
        _ => expr
    }
}
