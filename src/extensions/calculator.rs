use crate::config::AppConfig;
use super::api::{ExtensionMetadata, FlareExtension, ExtensionResult};

pub struct Calculator;

impl FlareExtension for Calculator {
    fn metadata(&self, config: &AppConfig) -> ExtensionMetadata {
        metadata(config)
    }

    fn should_handle(&self, query: &str, config: &AppConfig) -> bool {
        should_handle(query, config)
    }

    fn process(&self, query: &str, config: &AppConfig, _registry: &crate::extensions::ExtensionRegistry) -> ExtensionResult {
        if let Some((q, r)) = evaluate_from_query(query, config) {
            ExtensionResult::Single { query: q, result: r }
        } else {
            ExtensionResult::None
        }
    }
}

pub fn metadata(config: &AppConfig) -> ExtensionMetadata {
    ExtensionMetadata {
        name: "Calculator".to_string(),
        description: "Evaluate mathematical expressions".to_string(),
        trigger: trigger(config).to_string(),
    }
}

pub fn trigger(config: &AppConfig) -> &str {
    &config.features.calculator_search_trigger
}

pub fn should_handle(query: &str, config: &AppConfig) -> bool {
    query.starts_with(trigger(config))
}

pub fn evaluate_from_query(query: &str, config: &AppConfig) -> Option<(String, String)> {
    let trigger = trigger(config);
    let raw_query = query.strip_prefix(trigger).unwrap_or("").trim();

    if raw_query.is_empty() {
        return Some((String::new(), String::new()));
    }

    match evaluate(raw_query) {
        Ok(result) => Some((raw_query.to_string(), result.to_string())),
        Err(_) => Some((raw_query.to_string(), "Error".to_string())),
    }
}

pub fn evaluate(input: &str) -> Result<String, String> {
    let input = input.trim();
    if input.is_empty() {
        return Ok(String::new());
    }

    if let Some(result) = try_special_functions(input) {
        return result;
    }

    let meval_expr = convert_to_meval_syntax(input);
    
    match meval::eval_str(&meval_expr) {
        Ok(result) => {
            if result.fract() == 0.0 && result.abs() < 1e15 {
                Ok(format!("{}", result as i64))
            } else {
                Ok(format!("{}", result))
            }
        }
        Err(e) => Err(format!("Evaluation error: {}", e)),
    }
}

fn convert_to_meval_syntax(input: &str) -> String {
    input
        .replace("ln(", "log(")
        .replace("ABS(", "abs(")
        .replace("SIN(", "sin(")
        .replace("COS(", "cos(")
        .replace("TAN(", "tan(")
        .replace("SQRT(", "sqrt(")
        .replace("EXP(", "exp(")
        .replace("LOG(", "log10(")
}

fn try_special_functions(input: &str) -> Option<Result<String, String>> {
    let input_lower = input.to_lowercase();
    
    if input_lower.starts_with("integrate(") {
        return Some(handle_integrate(input));
    }
    
    if input_lower.starts_with("limit(") {
        return Some(handle_limit(input));
    }
    
    if input_lower.starts_with("diff(") {
        return Some(handle_diff(input));
    }
    
    None
}

fn parse_function_args(input: &str) -> Result<Vec<String>, String> {
    let start = input.find('(').ok_or("Missing opening parenthesis")?;
    let end = input.rfind(')').ok_or("Missing closing parenthesis")?;
    
    if end <= start {
        return Err("Invalid function syntax".to_string());
    }
    
    let args_str = &input[start + 1..end];
    
    let mut args = Vec::new();
    let mut current_arg = String::new();
    let mut paren_depth = 0;
    
    for c in args_str.chars() {
        match c {
            '(' => {
                paren_depth += 1;
                current_arg.push(c);
            }
            ')' => {
                paren_depth -= 1;
                current_arg.push(c);
            }
            ',' if paren_depth == 0 => {
                args.push(current_arg.trim().to_string());
                current_arg = String::new();
            }
            _ => current_arg.push(c),
        }
    }
    
    if !current_arg.trim().is_empty() {
        args.push(current_arg.trim().to_string());
    }
    
    Ok(args)
}

fn handle_integrate(input: &str) -> Result<String, String> {
    let args = parse_function_args(input)?;
    
    if args.len() == 4 {
        let expr = &args[0];
        let var = &args[1];
        let a: f64 = meval::eval_str(&convert_to_meval_syntax(&args[2]))
            .map_err(|e| format!("Invalid lower bound: {}", e))?;
        let b: f64 = meval::eval_str(&convert_to_meval_syntax(&args[3]))
            .map_err(|e| format!("Invalid upper bound: {}", e))?;
        
        let expr_for_eval = expr.clone();
        let var_for_eval = var.clone();
        
        let integrand = move |x: f64| -> f64 {
            let substituted = substitute_variable(&expr_for_eval, &var_for_eval, x);
            let meval_expr = convert_to_meval_syntax(&substituted);
            meval::eval_str(&meval_expr).unwrap_or(f64::NAN)
        };
        
        let result = quadrature::integrate(integrand, a, b, 1e-10);
        
        if result.integral.is_nan() {
            return Err("Integration failed: result is NaN".to_string());
        }
        
        if result.integral.fract() == 0.0 && result.integral.abs() < 1e15 {
            Ok(format!("{}", result.integral as i64))
        } else {
            Ok(format!("{:.10}", result.integral).trim_end_matches('0').trim_end_matches('.').to_string())
        }
    } else if args.len() == 2 {
        Err("Indefinite integrals are not supported. Use integrate(expr, var, a, b) for definite integrals.".to_string())
    } else {
        Err("integrate requires 4 arguments: integrate(expr, var, lower, upper)".to_string())
    }
}

fn handle_limit(input: &str) -> Result<String, String> {
    let args = parse_function_args(input)?;
    
    if args.len() != 3 {
        return Err("limit requires 3 arguments: limit(expr, var, value)".to_string());
    }
    
    let expr = &args[0];
    let var = &args[1];
    let value: f64 = meval::eval_str(&convert_to_meval_syntax(&args[2]))
        .map_err(|e| format!("Invalid limit value: {}", e))?;
    
    let substituted = substitute_variable(expr, var, value);
    let meval_expr = convert_to_meval_syntax(&substituted);
    
    match meval::eval_str(&meval_expr) {
        Ok(result) => {
            if result.is_nan() || result.is_infinite() {
                let h = 1e-10;
                let left = {
                    let sub = substitute_variable(expr, var, value - h);
                    meval::eval_str(&convert_to_meval_syntax(&sub)).unwrap_or(f64::NAN)
                };
                let right = {
                    let sub = substitute_variable(expr, var, value + h);
                    meval::eval_str(&convert_to_meval_syntax(&sub)).unwrap_or(f64::NAN)
                };
                
                if (left - right).abs() < 1e-6 {
                    let avg = (left + right) / 2.0;
                    if avg.fract() == 0.0 && avg.abs() < 1e15 {
                        Ok(format!("{}", avg as i64))
                    } else {
                        Ok(format!("{}", avg))
                    }
                } else if result.is_infinite() {
                    Ok(if result > 0.0 { "∞".to_string() } else { "-∞".to_string() })
                } else {
                    Err("Limit does not exist".to_string())
                }
            } else if result.fract() == 0.0 && result.abs() < 1e15 {
                Ok(format!("{}", result as i64))
            } else {
                Ok(format!("{}", result))
            }
        }
        Err(e) => Err(format!("Limit evaluation error: {}", e)),
    }
}

fn handle_diff(input: &str) -> Result<String, String> {
    let args = parse_function_args(input)?;
    
    if args.len() < 2 || args.len() > 3 {
        return Err("diff requires 2-3 arguments: diff(expr, var) or diff(expr, var, at_value)".to_string());
    }
    
    let expr = &args[0];
    let var = &args[1];
    
    if args.len() == 3 {
        let at_value: f64 = meval::eval_str(&convert_to_meval_syntax(&args[2]))
            .map_err(|e| format!("Invalid evaluation point: {}", e))?;
        
        let h = 1e-8;
        let f_plus = {
            let sub = substitute_variable(expr, var, at_value + h);
            meval::eval_str(&convert_to_meval_syntax(&sub)).unwrap_or(f64::NAN)
        };
        let f_minus = {
            let sub = substitute_variable(expr, var, at_value - h);
            meval::eval_str(&convert_to_meval_syntax(&sub)).unwrap_or(f64::NAN)
        };
        
        let derivative = (f_plus - f_minus) / (2.0 * h);
        
        if derivative.is_nan() {
            return Err("Differentiation failed".to_string());
        }
        
        if derivative.fract().abs() < 1e-10 && derivative.abs() < 1e15 {
            Ok(format!("{}", derivative.round() as i64))
        } else {
            Ok(format!("{:.10}", derivative).trim_end_matches('0').trim_end_matches('.').to_string())
        }
    } else {
        Err("Symbolic differentiation is not supported. Use diff(expr, var, at_value) for numerical derivative at a point.".to_string())
    }
}

fn substitute_variable(expr: &str, var: &str, value: f64) -> String {
    let mut result = String::new();
    let mut chars = expr.chars().peekable();
    
    while let Some(c) = chars.next() {
        if c.is_alphabetic() || c == '_' {
            let mut ident = String::new();
            ident.push(c);
            while let Some(&next_c) = chars.peek() {
                if next_c.is_alphanumeric() || next_c == '_' {
                    ident.push(next_c);
                    chars.next();
                } else {
                    break;
                }
            }
            if ident == var {
                if value < 0.0 {
                    result.push_str(&format!("({})", value));
                } else {
                    result.push_str(&format!("{}", value));
                }
            } else {
                result.push_str(&ident);
            }
        } else {
            result.push(c);
        }
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_abs_lowercase() {
        let result = evaluate("abs(-5)").unwrap();
        assert_eq!(result, "5");
    }

    #[test]
    fn test_abs_uppercase() {
        let result = evaluate("ABS(-5)").unwrap();
        assert_eq!(result, "5");
    }

    #[test]
    fn test_abs_positive() {
        let result = evaluate("abs(3.14)").unwrap();
        assert_eq!(result, "3.14");
    }

    #[test]
    fn test_limit_simple() {
        let result = evaluate("limit(x + 2, x, 3)").unwrap();
        assert_eq!(result, "5");
    }

    #[test]
    fn test_limit_polynomial() {
        let result = evaluate("limit(x^2, x, 4)").unwrap();
        assert_eq!(result, "16");
    }

    #[test]
    fn test_integrate_definite_constant() {
        let result = evaluate("integrate(2, x, 0, 3)").unwrap();
        let val: f64 = result.parse().unwrap();
        assert!((val - 6.0).abs() < 0.01);
    }

    #[test]
    fn test_integrate_definite_linear() {
        let result = evaluate("integrate(x, x, 0, 2)").unwrap();
        let val: f64 = result.parse().unwrap();
        assert!((val - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_integrate_quadratic() {
        let result = evaluate("integrate(x^2, x, 0, 3)").unwrap();
        let val: f64 = result.parse().unwrap();
        assert!((val - 9.0).abs() < 0.01);
    }

    #[test]
    fn test_basic_arithmetic() {
        assert_eq!(evaluate("2 + 3").unwrap(), "5");
        assert_eq!(evaluate("10 - 4").unwrap(), "6");
        assert_eq!(evaluate("3 * 4").unwrap(), "12");
        assert_eq!(evaluate("15 / 3").unwrap(), "5");
    }

    #[test]
    fn test_sqrt() {
        let result = evaluate("sqrt(16)").unwrap();
        assert_eq!(result, "4");
    }

    #[test]
    fn test_sin_cos() {
        let result = evaluate("sin(0)").unwrap();
        assert_eq!(result, "0");
        
        let result = evaluate("cos(0)").unwrap();
        assert_eq!(result, "1");
    }

    #[test]
    fn test_power() {
        assert_eq!(evaluate("2^3").unwrap(), "8");
        assert_eq!(evaluate("3^2").unwrap(), "9");
    }

    #[test]
    fn test_diff_at_point() {
        let result = evaluate("diff(x^2, x, 3)").unwrap();
        let val: f64 = result.parse().unwrap();
        assert!((val - 6.0).abs() < 0.01);
    }

    #[test]
    fn test_exp() {
        let result = evaluate("exp(0)").unwrap();
        assert_eq!(result, "1");
    }

    #[test]
    fn test_complex_expression() {
        let result = evaluate("2 * sin(0) + cos(0)").unwrap();
        assert_eq!(result, "1");
    }
}
