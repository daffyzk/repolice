use std::env;
use std::path::PathBuf;
use std::io::Result;

fn main() {
    let start_dir : Result<PathBuf> = env::current_dir();

    let args: Vec<String> = env::args().collect();
    
    let query = &args[1];
    let file_path = &args[2];
    
    println!("this is the cwd: {}", start_dir.expect("REASON").display());
    println!("Searching for {}", query);
    println!("In file {}", file_path);

}
