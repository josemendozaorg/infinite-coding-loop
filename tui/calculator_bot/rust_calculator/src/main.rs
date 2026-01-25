use std::io;

fn main() {
    println!("Rust Calculator");
    println!("---------------");

    loop {
        println!("
Enter first number (or q to quit):");
        let mut input1 = String::new();
        io::stdin().read_line(&mut input1).expect("Failed to read line");
        if input1.trim() == "q" { break; }
        let num1: f64 = match input1.trim().parse() {
            Ok(n) => n,
            Err(_) => { println!("Invalid number"); continue; }
        };

        println!("Enter operator (+, -, *, /):");
        let mut operator = String::new();
        io::stdin().read_line(&mut operator).expect("Failed to read line");
        let operator = operator.trim();

        println!("Enter second number:");
        let mut input2 = String::new();
        io::stdin().read_line(&mut input2).expect("Failed to read line");
        let num2: f64 = match input2.trim().parse() {
            Ok(n) => n,
            Err(_) => { println!("Invalid number"); continue; }
        };

        let result = match operator {
            "+" => num1 + num2,
            "-" => num1 - num2,
            "*" => num1 * num2,
            "/" => {
                if num2 == 0.0 { println!("Cannot divide by zero"); continue; }
                num1 / num2
            },
            _ => { println!("Invalid operator"); continue; }
        };

        println!("Result: {}", result);
    }
}
