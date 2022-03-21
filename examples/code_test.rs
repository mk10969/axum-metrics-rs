use std::{error::Error, io::BufRead};

fn main() -> Result<(), Box<dyn Error>> {
    // このコードは標準入力と標準出力を用いたサンプルコードです。
    // このコードは好きなように編集・削除していただいて構いません。
    // ---
    // This is a sample code to use stdin and stdout.
    // Edit and remove this code as you like.

    let lines: Vec<String> = std::io::stdin()
        .lock()
        .lines()
        .map(|line| line.expect("Failed to read from stdin"))
        .collect();

    let num1: usize = lines[0]
        .clone()
        .parse::<usize>()
        .expect("Failed to parse a number");

    let num2: Vec<i64> = lines[1]
        .clone()
        .split(" ")
        .map(|v| v.parse::<i64>().expect("Failed to parse line 2"))
        .collect();

    println!("{}", num1);
    println!("{:?}", num2);
    Ok(())
}
