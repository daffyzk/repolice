use std::env;
use std::path::PathBuf;
use std::io::Result;
use std::process::{Command, Output};

fn main() {
    let start_dir : Result<PathBuf> = env::current_dir();

    let args: Vec<String> = env::args().collect();
    
    let query = &args[1];
    let file_path = &args[2];
    
    println!("this is the cwd: {}", start_dir.expect("REASON").display());
    println!("Searching for {}", query);
    println!("In file {}", file_path);
    get_repos();
}

fn get_repos() {
    let cwd_s : String = env::current_dir().unwrap().into_os_string().into_string().unwrap(); 
    let output : Output = Command::new("find").arg(cwd_s)
        .arg("-name")
        .arg(".git")
        .arg("-type")
        .arg("d")
        .output().expect("Error!");
    let repo_dirs : String = String::from_utf8(output.stdout).unwrap();
    let repo_list : Vec<&str> = repo_dirs.lines().collect();
    println!("{:?}", repo_dirs);
    println!("{:?}", repo_list);
    
}
