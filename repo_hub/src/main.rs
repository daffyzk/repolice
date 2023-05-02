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
        1.. => parse_args(args),
        _ => println!("todo"),
    }
    
    get_status(get_repos(get_cwd()), false);
}

//name extraction for the repo will not work if it has a slash on it, but whatever.
fn get_status(repos: Vec<String>, simple: bool){
    let re: Regex = Regex::new(r"([^/]+$)").unwrap();

    for path in repos{
        let repo_name: String = re.find(&path).unwrap().as_str().to_string();
        assert!(env::set_current_dir(&path).is_ok());
        assert_eq!(get_cwd().display().to_string(), path);

        let output: Output = Command::new("git").args(["status", "--short"]).stdout(Stdio::piped())
            .output().expect("Not a git Repository!");
        let status: String = String::from_utf8_lossy(&output.stdout).to_string();
        println!("| {}: {}", &repo_name, status_message(status, simple));        
        // todo add commits
    }
}

fn status_message(m: String, simple: bool) -> String{
    let gb: Output = Command::new("git").args(["branch", "--show-current"]).stdout(Stdio::piped())
        .output().expect("Error!");
    let branch = String::from_utf8_lossy(&gb.stdout).to_string().replace("\n", "");
    match simple {
        true => { return format!("[{}]\n| ?{} | +{} | ~{} | -{} |\n", branch,
                    count_matches(&m, "?? "),
                    count_matches(&m, "A "),
                    count_matches(&m, "M "),
                    count_matches(&m, "D "));
                }
        false => {return format!("[{}]\n{}", branch, get_files_formatted(&m));
                }
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
    let repo_list: Vec<String> = repo_results.lines().map(String::from).to_vec();     
    
    repo_list
}

fn get_files_formatted(m: &String) -> String{
    format!("{}{}{}{}", 
            get_files_list(&m, Regex::new(r"^\?\? (.*)").unwrap(), "New Files:\n".to_string()),
            get_files_list(&m, Regex::new(r"^A (.*)").unwrap(), "Added Files:\n".into()),
            get_files_list(&m, Regex::new(r"^M (.*)").unwrap(), "Modified Files:\n".into()),
            get_files_list(&m, Regex::new(r"^D (.*)").unwrap(), "Deleted Files:\n".into())
        ).to_string()
}

fn get_files_list(text: &String, re: Regex, title: String) -> String{
    let mut strang: String = "".to_string();

    for cap in re.captures_iter(text){
        strang.push_str(&cap[1]);
        strang.push_str("\n");
    }
    
    let s_len = strang.len().to_string().parse::<i32>().unwrap();

    if s_len > 2{
        strang.insert_str(0, &title);
        return strang;
    }
    else {
        strang.insert_str(0, &title);
        return format!("No {}", strang.replace(":", "."));
    }
}

fn count_matches(text: &String, sub_string: &str) -> String{
    format!("{}", text.matches(&sub_string).count().to_string())
}

fn parse_args(args : Vec<String>){
    for arg in args {
        match arg.as_str(){
            "-x" => {println!("x was passed as parameter")},
            "-f" => {println!("f was passed as parameter")},
            _ => {},
        }
    }
}

fn get_cwd() -> PathBuf{
    env::current_dir().unwrap()
}
