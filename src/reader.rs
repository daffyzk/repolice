use std::path::PathBuf;
use std::process::{Stdio, Command, Output};
use std::sync::Arc;
use std::{env, thread};
use regex::Regex;
use to_vec::ToVec;

#[derive(Clone)]
pub struct RepoInfo {
    pub name: String,
    pub branch: String,
    pub new_files: usize,
    pub added_files: usize,
    pub modified_files: usize,
    pub deleted_files: usize,
    pub verbose_info: String,
}

impl RepoInfo {
    pub fn has_changes(&self) -> bool {
        self.new_files > 0 || self.added_files > 0 || self.modified_files > 0 || self.deleted_files > 0
    }

    pub fn total_changes(&self) -> usize {
        self.new_files + self.added_files + self.modified_files + self.deleted_files
    }
}

pub struct Reader {}

impl Reader {
    pub fn get_repos(path: PathBuf) -> Vec<String> {
        let dir: String = path.into_os_string().into_string().unwrap();
        let output: Output = Command::new("find")
            .args([&dir,"-name", ".git","-type", "d"])
            .stdout(Stdio::piped())
            .output().expect("Error!");
        let repo_results: String = String::from_utf8_lossy(&output.stdout).to_string()
            .replace("/.git", "");

        repo_results.lines().map(String::from).to_vec()
    }

    /// Collects info
    pub fn collect_repo_info(repo_list: Vec<String>, verbose: bool, _depth: u8) -> Vec<RepoInfo> {
        //name extraction for the repo will not work if it has a slash on it, but whatever.
        let re: Arc<Regex> = Arc::new(Regex::new(r"([^/]+$)").unwrap());
        let mut repos = Vec::new();

        for path in repo_list {
            let reg = re.clone(); // new ref
            let thread = thread::spawn( move || {
                let repo_name = reg.clone().find(&path).unwrap().as_str();
                assert!(env::set_current_dir(&path).is_ok());

                // Get git status --short for file status
                let output: Output = Command::new("git").args(["status", "--short"]).stdout(Stdio::piped())
                    .output().expect("Not a git Repository!");
                let status: String = String::from_utf8_lossy(&output.stdout).to_string();

                // Get current branch
                let gb: Output = Command::new("git").args(["branch", "--show-current"]).stdout(Stdio::piped())
                    .output().expect("Error!");
                let branch = String::from_utf8_lossy(&gb.stdout).to_string().replace("\n", "");

                RepoInfo {
                    name: repo_name.to_string(),
                    branch,
                    new_files: Self::count_matches(&status, "?? "),
                    added_files: Self::count_matches(&status, "A "),
                    modified_files: Self::count_matches(&status, "M "),
                    deleted_files: Self::count_matches(&status, "D "),
                    verbose_info: if verbose{ Self::get_files_formatted(&status) } else { String::new() },
                }
            });
            repos.push(thread.join().unwrap());
        }

        // Sort repositories: ones with changes first (by total changes descending), then clean ones alphabetically
        repos.sort_by(|a, b| {
            match (a.has_changes(), b.has_changes()) {
                (true, false) => std::cmp::Ordering::Less,                      // repos with changes come first
                (false, true) => std::cmp::Ordering::Greater,                   // clean repos come last
                (true, true) => b.total_changes().cmp(&a.total_changes()),      // sort by most changes first
                (false, false) => a.name.cmp(&b.name),                          // clean repos sorted alphabetically
            }
        });

        repos
    }

    fn get_files_formatted(m: &String) -> String{
        let mut file_list: Vec<(String, String)> = vec![];
        file_list.push(("New".to_string(), Self::get_files_list(&m, Regex::new(r"\?\? (.*)\n").unwrap())));
        file_list.push(("Added".to_string(), Self::get_files_list(&m, Regex::new(r"A (.*)\n").unwrap())));
        file_list.push(("Modified".to_string(), Self::get_files_list(&m, Regex::new(r"M (.*)\n").unwrap())));
        file_list.push(("Deleted".to_string(), Self::get_files_list(&m, Regex::new(r"D (.*)\n").unwrap())));
        
        Self::formatted_list(file_list)
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

    fn count_matches(text: &String, sub_string: &str) -> usize {
        text.matches(&sub_string).count()
    }

}






