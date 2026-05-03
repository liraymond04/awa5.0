/// Example: Compile libfoo.awaml to AWASM and execute
use awa5_rs::compiler::compile_to_binary;
use std::fs;

fn main() {
    // Read the AwaML source file
    let source = match fs::read_to_string("examples/awaml/libfoo.awaml") {
        Ok(src) => src,
        Err(e) => {
            eprintln!("Failed to read file: {}", e);
            return;
        }
    };
    
    println!("=== Compiling libfoo.awaml to AWASM ===\n");
    
    // Compile directly to binary object code
    match compile_to_binary(&source) {
        Ok(binary) => {
            println!("✅ Compilation successful!");
            println!("   Source size: {} bytes", source.len());
            println!("   Binary size: {} bytes", binary.len());
            
            // Save the binary for inspection
            if let Err(e) = fs::write("libfoo.o", &binary) {
                eprintln!("Failed to write output: {}", e);
                return;
            }
            println!("   Saved to: libfoo.o\n");
            
            // Show what was generated
            println!("=== Pipeline ===");
            println!("1. AwaML source (libfoo.awaml)");
            println!("2. Parsed to AST");
            println!("3. Lowered to Core-IR");
            println!("4. Compiled to AWASM instructions");
            println!("5. Assembled to binary object code");
            println!("6. Ready for execution or further processing");
        }
        Err(e) => {
            eprintln!("Compilation failed: {:?}", e);
        }
    }
}
