use std::sync::Arc;
use std::thread;
use std::env;
use std::path::PathBuf;
use std::process::{Stdio, Command, Output};
use regex::Regex;
use clap::Parser;

mod reader;
mod tui;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Set a specific path to run in, instead of the current directory
    #[arg(short, long, value_name = "PATH")]
    path: Option<String>,

    /// Set a max depth to search for repositories in the file-system
    #[arg(short, long, value_name = "DEPTH")]
    depth: Option<u8>,

    /// Display a more verbose list of files staged for commits 
    #[arg(short, long, action)]
    verbose: Option<bool>,
    
    /// Display the status for a repository if it has new files or branches
    #[arg(short, long, action)]
    fetch: Option<bool>,

    /// Disable TUI and print to stdout instead
    #[arg(long)]
    no_tui: Option<bool>,

}

fn main() {
    let args = Args::parse();
    
    let mut exec_path : PathBuf = get_cwd();
    let mut exec_depth : u8 = 10; 
    let mut exec_simple : bool = true; 
    let mut exec_fetch : bool = false;
    let mut exec_no_tui : bool = false;

    match args.no_tui{
        Some(p) => {exec_no_tui = p},
        None => {},
    }

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

    match args.fetch{
        Some(_) => {exec_fetch = true; println!("fetch = {}", exec_fetch)},
        None => {println!("fetch = {}", exec_fetch)},
    }

    
    let repos = collect_repo_info(reader::get_repos(exec_path.clone()), exec_simple, exec_depth);
    
    if exec_no_tui {
        print_repos_simple(repos, exec_simple);
    } else {
        match tui::run_tui_with_repos(repos, exec_simple) {
            Ok(_) => {},
            Err(_) => {
                println!("TUI failed, falling back to simple output...");
                let repos = collect_repo_info(reader::get_repos(exec_path), exec_simple, exec_depth);
                print_repos_simple(repos, exec_simple);
            }
        }
    }
}


//name extraction for the repo will not work if it has a slash on it, but whatever.
fn collect_repo_info(repo_list: Vec<String>, simple: bool, _depth: u8) -> Vec<tui::RepoInfo> {
    let re: Arc<Regex> = Arc::new(Regex::new(r"([^/]+$)").unwrap());
    let mut repos = Vec::new();

    for path in repo_list {
        let new_path = path.clone();
        let new_re = re.clone();

        let thread = thread::spawn(move || {
            let repo_name: String = new_re.find(new_path.clone().as_str()).unwrap().as_str().to_string();
            assert!(env::set_current_dir(new_path.clone().as_str()).is_ok());

            // Get git status --short for file status
            let output: Output = Command::new("git").args(["status", "--short"]).stdout(Stdio::piped())
                .output().expect("Not a git Repository!");
            let status: String = String::from_utf8_lossy(&output.stdout).to_string();

            // Get current branch
            let gb: Output = Command::new("git").args(["branch", "--show-current"]).stdout(Stdio::piped())
                .output().expect("Error!");
            let branch = String::from_utf8_lossy(&gb.stdout).to_string().replace("\n", "");


            tui::RepoInfo {
                name: repo_name,
                branch,
                new_files: count_matches(&status, "?? "),
                added_files: count_matches(&status, "A "),
                modified_files: count_matches(&status, "M "),
                deleted_files: count_matches(&status, "D "),
                verbose_info: if simple { String::new() } else { get_files_formatted(&status) },
            }
        });
        repos.push(thread.join().unwrap());
    }

    repos
}

fn print_repos_simple(repos: Vec<tui::RepoInfo>, simple: bool) {
    for repo in repos {
        if simple {
            println!("| {}: [{}]", repo.name, repo.branch);
            println!("| ?{} | +{} | ~{} | -{} |", 
                repo.new_files, repo.added_files, repo.modified_files, repo.deleted_files);
        } else {
            println!("| {}: [{}]", repo.name, repo.branch);
            println!("| ?{} | +{} | ~{} | -{} |", 
                repo.new_files, repo.added_files, repo.modified_files, repo.deleted_files);
            println!("{}", repo.verbose_info);
        }
        println!("");
    }
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
