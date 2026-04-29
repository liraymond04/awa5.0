extern crate clap;

use awa5_rs::*;

use clap::{Arg, Command};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

fn main() {
    let mut cmd = Command::new("awa5_rs")
        .version("1.0")
        .about("An AWA5.0 CLI tool written in Rust (btw)")
        .arg(
            Arg::new("input")
                .index(1)
                .help("File to interpret or convert")
                .num_args(1),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .help("Output to file with new format .awasm .awa .o")
                .num_args(1),
        )
        .arg(
            Arg::new("string")
                .short('s')
                .long("string")
                .help("String to interpret or convert")
                .num_args(1),
        )
        .arg(
            Arg::new("awasm")
                .long("awasm")
                .help("Parse string as awasm")
                .num_args(0),
        )
        .arg(
            Arg::new("awa")
                .long("awa")
                .help("Parse string as awatalk")
                .num_args(0),
        )
        .arg(
            Arg::new("awaml")
                .long("awaml")
                .help("Parse string as AwaML (compiler scaffold)")
                .num_args(0),
        )
        .arg(
            Arg::new("path")
                .short('p')
                .long("path")
                .help("Search paths separated by ';' for shared libraries")
                .num_args(1),
        )
        .arg(
            Arg::new("include")
                .short('i')
                .long("include")
                .help("Include paths separated by ';' for source files")
                .num_args(1),
        );

    let matches = cmd.clone().get_matches();

    if matches.contains_id("input") && matches.contains_id("string") {
        eprintln!(
            "Warning: Both file input and 'string' flag are given. Please provide only one.\n"
        );
        cmd.print_help().unwrap();
        std::process::exit(1);
    }

    let mut path = "/usr/local/lib";
    let mut include_paths = "";

    if matches.contains_id("path") {
        path = matches.get_one::<String>("path").unwrap();
    }

    if matches.contains_id("include") {
        include_paths = matches.get_one::<String>("include").unwrap();
    }

    if matches.contains_id("input") {
        let input_file = matches.get_one::<String>("input").unwrap();

        let file_type = detect_file_format(input_file);

        match file_type {
            "awasm" => {
                let lines = read_lines(input_file).unwrap();

                let mut macro_table = parser::awasm::MacroTable::new();
                let mut already_included: HashSet<String> = HashSet::new();
                let mut label_included: HashSet<String> = HashSet::new();
                let instructions = parser::awasm::parse_lines(
                    &mut macro_table,
                    &mut already_included,
                    &mut label_included,
                    include_paths,
                    "",
                    lines,
                );

                let object_vec = assembler::make_object_vec(&instructions);

                if matches.contains_id("output") {
                    let output_file = matches.get_one::<String>("output").unwrap();

                    match Path::new(output_file)
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .unwrap()
                    {
                        "awasm" => {
                            let _ = fs::copy(input_file, output_file);
                        }
                        "awa" => {
                            let result = assembler::object_to_awa(&object_vec);
                            let _ = write_string_file(output_file, &result);
                        }
                        "o" => {
                            let _ = write_object_file(output_file, object_vec);
                        }
                        _ => {}
                    }
                } else {
                    interpet_object(object_vec, path);
                }
            }
            "awa" => {
                let content = fs::read_to_string(input_file).unwrap();

                let instructions = parser::awatalk::parse_string(&content);

                let object_vec = assembler::make_object_vec(&instructions);

                if matches.contains_id("output") {
                    let output_file = matches.get_one::<String>("output").unwrap();

                    match Path::new(output_file)
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .unwrap()
                    {
                        "awasm" => {
                            let result = assembler::object_to_awasm(&object_vec);
                            let _ = write_string_file(output_file, &result);
                        }
                        "awa" => {
                            let _ = fs::copy(input_file, output_file);
                        }
                        "o" => {
                            let _ = write_object_file(output_file, object_vec);
                        }
                        _ => {}
                    }
                } else {
                    interpet_object(object_vec, path);
                }
            }
            "awaml" => {
                let content = fs::read_to_string(input_file).unwrap();

                match compiler::compile_and_render(&content) {
                    Ok(rendered) => {
                        if matches.contains_id("output") {
                            let output_file = matches.get_one::<String>("output").unwrap();
                            let _ = write_string_file(output_file, &rendered);
                        } else {
                            println!("{}", rendered);
                        }
                    }
                    Err(err) => {
                        eprintln!("AwaML compile error: {err:?}");
                    }
                }
            }
            "o" => match read_binary_file(input_file) {
                Ok(binary_data) => {
                    if matches.contains_id("output") {
                        let output_file = matches.get_one::<String>("output").unwrap();

                        match Path::new(output_file)
                            .extension()
                            .and_then(|ext| ext.to_str())
                            .unwrap()
                        {
                            "awasm" => {
                                let result = assembler::object_to_awasm(&binary_data);
                                let _ = write_string_file(output_file, &result);
                            }
                            "awa" => {
                                let result = assembler::object_to_awa(&binary_data);
                                let _ = write_string_file(output_file, &result);
                            }
                            "o" => {
                                let _ = fs::copy(input_file, output_file);
                            }
                            _ => {}
                        }
                    } else {
                        interpet_object(binary_data, path);
                    }
                }
                Err(err) => {
                    eprintln!("Error reading file: {}", err);
                }
            },
            _ => panic!("Could not autodetect file type"),
        }
    } else {
        let mut input_string = String::new();
        if matches.contains_id("string") {
            input_string = matches.get_one::<String>("string").unwrap().clone();
        } else {
            let _ = std::io::stdin().read_line(&mut input_string);
        }

        if matches.get_flag("awasm") && matches.get_flag("awa") {
            eprintln!(
                "Warning: Both 'awasm' and 'awa' flags are given. Please provide only one.\n"
            );
            cmd.print_help().unwrap();
            std::process::exit(1);
        }

        if matches.get_flag("awaml") && (matches.get_flag("awasm") || matches.get_flag("awa")) {
            eprintln!("Warning: 'awaml' cannot be combined with 'awasm' or 'awa'. Please provide only one.\n");
            cmd.print_help().unwrap();
            std::process::exit(1);
        }

        if !matches.get_flag("awasm") && !matches.get_flag("awa") && !matches.get_flag("awaml") {
            eprintln!("Warning: Neither 'awasm', 'awa', nor 'awaml' flags are given. Please provide at least one.\n");
            cmd.print_help().unwrap();
            std::process::exit(1);
        }

        if matches.get_flag("awasm") {
            let lines = input_string.lines().map(|line| line.to_string());

            let mut macro_table = parser::awasm::MacroTable::new();
            let mut already_included: HashSet<String> = HashSet::new();
            let mut label_included: HashSet<String> = HashSet::new();
            let instructions = parser::awasm::parse_lines(
                &mut macro_table,
                &mut already_included,
                &mut label_included,
                include_paths,
                "",
                lines,
            );

            let object_vec = assembler::make_object_vec(&instructions);

            if matches.contains_id("output") {
                let output_file = matches.get_one::<String>("output").unwrap();

                match Path::new(output_file)
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .unwrap()
                {
                    "awasm" => {
                        let result = assembler::object_to_awasm(&object_vec);
                        let _ = write_string_file(output_file, &result);
                    }
                    "awa" => {
                        let result = assembler::object_to_awa(&object_vec);
                        let _ = write_string_file(output_file, &result);
                    }
                    "o" => {
                        let _ = write_object_file(output_file, object_vec);
                    }
                    _ => {}
                }
            } else {
                interpet_object(object_vec, path);
            }
        }

        if matches.get_flag("awa") {
            let instructions = parser::awatalk::parse_string(&input_string);

            let object_vec = assembler::make_object_vec(&instructions);

            if matches.contains_id("output") {
                let output_file = matches.get_one::<String>("output").unwrap();

                match Path::new(output_file)
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .unwrap()
                {
                    "awasm" => {
                        let result = assembler::object_to_awasm(&object_vec);
                        let _ = write_string_file(output_file, &result);
                    }
                    "awa" => {
                        let result = assembler::object_to_awa(&object_vec);
                        let _ = write_string_file(output_file, &result);
                    }
                    "o" => {
                        let _ = write_object_file(output_file, object_vec);
                    }
                    _ => {}
                }
            } else {
                interpet_object(object_vec, path);
            }
        }

        if matches.get_flag("awaml") {
            match compiler::compile_and_render(&input_string) {
                Ok(rendered) => {
                    if matches.contains_id("output") {
                        let output_file = matches.get_one::<String>("output").unwrap();
                        let _ = write_string_file(output_file, &rendered);
                    } else {
                        println!("{}", rendered);
                    }
                }
                Err(err) => {
                    eprintln!("AwaML compile error: {err:?}");
                }
            }
        }
    }
}

