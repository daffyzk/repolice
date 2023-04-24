use std::env;
use std::path::PathBuf;
use std::process::{Stdio, Command, Output};
use to_vec::ToVec;
use regex::Regex;

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
    get_status(get_repos(get_cwd()));
}

//name extraction for the repo will not work if it has a slash on it, but whatever.
fn get_status(repos : Vec<String>){
    //let mut final_status : Vec<String> = vec![];
    let re : Regex = Regex::new(r"([^/]+$)").unwrap();

    for path in repos{
        let repo_name : String = re.find(&path).unwrap().as_str().to_string();
        assert!(env::set_current_dir(&path).is_ok());
        assert_eq!(get_cwd().display().to_string(), path);

        let output : Output = Command::new("git").args(["status", "--short"]).stdout(Stdio::piped())
            .output().expect("Not a git Repository!");
        let status : String = String::from_utf8_lossy(&output.stdout).to_string();
        println!("| {}: {}", &repo_name, filter_status_message(status));        
        // todo add commits
    }
}

fn filter_status_message(m : String) -> String{
    let gb : Output = Command::new("git").args(["branch", "--show-current"]).stdout(Stdio::piped())
        .output().expect("Error!");
    let branch = String::from_utf8_lossy(&gb.stdout).to_string().replace("\n", "");
    let filtered = format!("[{}]\n| ?{} | +{} | ~{} | -{} |", branch,
        count_matches(&m, "?? "),
        count_matches(&m, "A "),
        count_matches(&m, "M "),
        count_matches(&m, "D "));

    filtered
}

fn get_repos(path : PathBuf) -> Vec<String> {
    let dir : String = path.into_os_string().into_string().unwrap();
    let output : Output = Command::new("find")
        .args([&dir,"-name", ".git","-type", "d"])
        .stdout(Stdio::piped())
        .output().expect("Error!");
    let repo_results : String = String::from_utf8_lossy(&output.stdout).to_string()
        .replace("/.git", "");
    let repo_list : Vec<String> = repo_results.lines().map(String::from).to_vec();     
    
    repo_list
}

fn count_matches(text : &String, sub_string : &str) -> String{
    format!("{}", text.matches(&sub_string).count().to_string())
}

fn get_cwd() -> PathBuf{
    env::current_dir().unwrap()
}
