use std::env;
use std::path::PathBuf;
use std::io::Result;
use std::process::{Stdio, Command, Output};

fn main() {

    let args: Vec<String> = env::args().collect();
    let arg_len = args.len().to_string().parse::<i32>().unwrap() - 1;
    match arg_len{
        0 => println!("no args"),
        1 => println!("args: {}", &args[1]),
        2 => println!("args: {} - {}", &args[1], &args[2]),
        3 => println!("too many args!"),
        _ => println!("todo"),
    }
    
    println!("this is the cwd: {}", get_cwd().expect("REASON").display());
    get_repos();
}

fn get_repos() {
    let cwd_s : String = get_cwd().unwrap().into_os_string().into_string().unwrap();
    let output : Output = Command::new("find")
        .args([&cwd_s,"-name", ".git","-type", "d"])
        .stdout(Stdio::piped())
        .output().expect("Error!");
    let repo_results : String = String::from_utf8_lossy(&output.stdout).to_string();
    let repo_list : Vec<&str> = repo_results.lines().collect();
    //repo_list.iter().filter();
    println!("----list of repositories----");
    println!("{:?}", repo_list);
    

}

fn get_cwd() -> Result<PathBuf>{
    return env::current_dir();
}
