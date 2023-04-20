use std::env;
use std::path::PathBuf;
use std::io::Result;
use std::process::{Command, Output};

fn main() {

    let args: Vec<String> = env::args().collect();
    
    match args.len() as i64{
        0 => println!("no args"),
        1 => println!("args: {}", &args[0]),
        2 => println!("args: {} - {}", &args[0], &args[1]),
        3 => println!("too many args!"),
        _ => println!("todo"),
    }
    
    println!("this is the cwd: {}", get_cwd().expect("REASON").display());
    get_repos();
}

fn get_repos() {
    let cwd_s : String = get_cwd().unwrap().into_os_string().into_string().unwrap(); 
    println!("{}", cwd_s);
    let output : Output = Command::new("find").arg(cwd_s)
        .arg("-name .git")
        .arg("-type d")
        .output().expect("Error!");
    let repo_dirs : String = String::from_utf8(output.stdout).unwrap();
    let repo_list : Vec<&str> = repo_dirs.lines().collect();
    println!("{:?}", repo_dirs);
    println!("----list of repositories----");
    println!("{:?}", repo_list);
    

}

fn get_cwd() -> Result<PathBuf>{
    return env::current_dir();
}
