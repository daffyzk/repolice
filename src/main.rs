use std::env;
use std::path::PathBuf;
use std::process::{Stdio, Command, Output};
use to_vec::ToVec;
use regex::Regex;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Set a specific path to run in, instead of the current directory
    #[arg(short, long, value_name = "PATH")]
    path: Option<String>,

    /// Set a max depth to search for repositories in the file-system
    #[arg(short, long, value_name = "DEPTH")]
    depth: Option<i8>,

    /// Display a more verbose list of files staged for commits 
    #[arg(short, long, action)]
    verbose: Option<bool>,
    
    /// displays the status the repository if it has new files or branches
    #[arg(short, long, action)]
    fetch: Option<bool>,

}

fn main() {
    let mut exec_path : PathBuf = get_cwd();
    let mut exec_depth : i8 = 127; 
    let mut exec_simple : bool = true; 
    let mut exec_fetch : bool = false;

    let args = Args::parse();

    match args.path{
        Some(p) => {exec_path = PathBuf::from(p)},
        None => {},
    }

    match args.verbose{
        Some(_) => {exec_simple = false},
        None => {},
    }

    match args.depth{
        Some(d) => {exec_depth = d; println!("depth = {}, {}", d, exec_depth)},
        None => {},
    }

    match args.verbose{
        Some(f) => {exec_fetch = true; println!("fetch = {}, {}", f, exec_fetch)},
        None => {},
    }
    
    get_status(get_repos(exec_path), exec_simple);
}

//name extraction for the repo will not work if it has a slash on it, but whatever.
fn get_status(repo_list: Vec<String>, simple: bool){
    let re: Regex = Regex::new(r"([^/]+$)").unwrap();
    for path in repo_list{
        let repo_name: String = re.find(&path).unwrap().as_str().to_string();
        assert!(env::set_current_dir(&path).is_ok());

        // could check with 'git remote get-url origin' to see if it's a hosted repository and return different response based on that information
        let output: Output = Command::new("git").args(["status", "--short"]).stdout(Stdio::piped())
            .output().expect("Not a git Repository!");
        let status: String = String::from_utf8_lossy(&output.stdout).to_string();

        // This is where it all renders out (one print line statement lol)
        println!("| {}: {}", &repo_name, status_message(status, simple));        
    }
}

fn status_message(m: String, simple: bool) -> String{
    let gb: Output = Command::new("git").args(["branch", "--show-current"]).stdout(Stdio::piped())
        .output().expect("Error!");
    let branch = String::from_utf8_lossy(&gb.stdout).to_string().replace("\n", "");
    match simple {
        true => { 
            // here maybe collect matches and return nothing ?? idk probably better to refactor
            // the whole thing
            return format!("[{}]\n| ?{} | +{} | ~{} | -{} |\n", branch,
                    count_matches(&m, "?? "),
                    count_matches(&m, "A "),
                    count_matches(&m, "M "),
                    count_matches(&m, "D "));
                }
        false => {return format!("[{}]\n{}", branch, get_files_formatted(&m));}
    }
}

fn get_repos(path: PathBuf) -> Vec<String> {
    let dir: String = path.into_os_string().into_string().unwrap();
    let output: Output = Command::new("find")
        .args([&dir,"-name", ".git","-type", "d"])
        .stdout(Stdio::piped())
        .output().expect("Error!");
    let repo_results: String = String::from_utf8_lossy(&output.stdout).to_string()
        .replace("/.git", "");

    repo_results.lines().map(String::from).to_vec()
}

fn get_files_formatted(m: &String) -> String{
    let mut file_list: Vec<(String, String)> = vec![];
    file_list.push(("New".to_string(), get_files_list(&m, Regex::new(r"\?\? (.*)\n").unwrap())));
    file_list.push(("Added".to_string(), get_files_list(&m, Regex::new(r"A (.*)\n").unwrap())));
    file_list.push(("Modified".to_string(), get_files_list(&m, Regex::new(r"M (.*)\n").unwrap())));
    file_list.push(("Deleted".to_string(), get_files_list(&m, Regex::new(r"D (.*)\n").unwrap())));
    
    formatted_list(file_list)
}

fn formatted_list(list: Vec<(String, String)>) -> String{
    let mut final_list: String = "".to_string();
    let mut not_list: Vec<String> = vec![];
    for item in list {
        let i_s = item.1.len().to_string().parse::<i32>().unwrap();
        if i_s > 1 {
            let title: String = format!("| {} Files:\n", item.0).to_string();
            final_list.push_str(&title);
            final_list.push_str(&item.1);
        }
        else {
            not_list.push(item.0);
        }
    }
    let mut final_no_element_list: String = "".to_string();
    let e_s = not_list.len().to_string().parse::<i32>().unwrap();
    match e_s {
        1 => {
            final_no_element_list = format!("{}", not_list.get(0).unwrap()).to_string()},
        2 => {
            final_no_element_list = format!("{} or {}", not_list.get(0).unwrap(), 
            not_list.get(1).unwrap()).to_string()},
        3 => {
            final_no_element_list = format!("{}, {} or {}", not_list.get(0).unwrap(),
            not_list.get(1).unwrap(), not_list.get(2).unwrap()).to_string()},
        4 => {
            final_no_element_list = format!("{}, {}, {} or {}", not_list.get(0).unwrap(), 
            not_list.get(1).unwrap(), not_list.get(2).unwrap(), 
            not_list.get(3).unwrap()).to_string()},
        _ => {}
    }
    if final_no_element_list.is_empty(){
        return final_list;
    }
    else {
        return format!("| No {} Files.\n{}", final_no_element_list, final_list).to_string();
    }
}

fn get_files_list(text: &String, re: Regex) -> String{
    let mut strang: String = "".to_string();
    for cap in re.captures_iter(text){
        let m: String = format!("| _ {}\n", &cap[1]);
        strang.push_str(&m);
    }
    
    strang
}

fn count_matches(text: &String, sub_string: &str) -> String{
    text.matches(&sub_string).count().to_string()
}

fn get_cwd() -> PathBuf{
    env::current_dir().unwrap()
}
