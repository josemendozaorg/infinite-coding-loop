use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 4 {
        eprintln!("Usage: calculator <num> <operator> <num>");
        std::process::exit(1);
    }

    let num1: f64 = match args[1].parse() {
        Ok(n) => n,
        Err(_) => {
            eprintln!("Error: First argument is not a number");
            std::process::exit(1);
        }
    };

    let operator = &args[2];

    let num2: f64 = match args[3].parse() {
        Ok(n) => n,
        Err(_) => {
            eprintln!("Error: Second argument is not a number");
            std::process::exit(1);
        }
    };

    let result = match operator.as_str() {
        "+" => num1 + num2,
        "-" => num1 - num2,
        "*" => num1 * num2,
        "/" => {
            if num2 == 0.0 {
                eprintln!("Error: Division by zero");
                std::process::exit(1);
            }
            num1 / num2
        }
        _ => {
            eprintln!("Error: Invalid operator. Use +, -, *, or /");
            std::process::exit(1);
        }
    };

    println!("{}", result);
}
