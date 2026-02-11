//! PHP-RS Interpreter CLI

use phprs::engine::compile::compile_file;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: php <file>");
        std::process::exit(1);
    }

    let filename = &args[1];

    // Compile the PHP file
    let op_array = compile_file(filename)
        .map_err(|e| anyhow::anyhow!("Compile error: {}", e))?;

    // TODO: Execute the opcodes
    println!("Compiled {} successfully", filename);
    println!("Generated {} opcodes", op_array.ops.len());

    Ok(())
}
