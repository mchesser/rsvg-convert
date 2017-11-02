use std::env;
use std::path::Path;
use std::process::Command;
use std::fs;

fn main() {
    let mut cached_file = env::temp_dir();
    cached_file.push("rsvg-convert-cache");

    let _ = fs::create_dir(&cached_file);

    let output = env::args().nth(5).expect("Output file not specified");
    let input = env::args().nth(6).expect("Input file not specified");

    let output_path = Path::new(&output);

    let output_file_name = output_path.file_name().unwrap();
    cached_file.push(output_file_name);

    if !cached_file.exists() {
        eprintln!("Converting file: {:?}", input);
        Command::new("inkscape")
            .arg(&input)
            .arg("-A")
            .arg(&cached_file)
            .output()
            .expect("failed to execute process");
    }
    else {
        eprintln!("Loading from cache: {:?}", cached_file);    
    }

    fs::copy(cached_file, output_path).unwrap();
}