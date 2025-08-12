use std::path::PathBuf;
use std::process::{Stdio, Command, Output};
use std::sync::Arc;
use std::thread;
use regex::Regex;
use to_vec::ToVec;
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, Stream};
use gix;

#[derive(Clone)]
pub struct FileTracker {
    pub status: String,
    pub amount: usize,
    pub files: Option<Vec<String>>
}

impl FileTracker {
    fn new(status: &str, amount: usize, files: Option<Vec<String>>) -> Self {
        Self {
           status: status.to_string(), 
           amount,
           files
        }
    }
}

#[derive(Clone)]
pub struct RepoInfo {
    pub name: String,
    pub branch: String,
    pub new_files: FileTracker,
    pub added_files: FileTracker,
    pub modified_files: FileTracker,
    pub deleted_files: FileTracker,
}

impl RepoInfo {
    pub fn has_changes(&self) -> bool {
        self.new_files.amount > 0 || self.added_files.amount > 0 || self.modified_files.amount > 0 || self.deleted_files.amount > 0
    }

    pub fn total_changes(&self) -> usize {
        self.new_files.amount + self.added_files.amount + self.modified_files.amount + self.deleted_files.amount
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

    /// Creates a stream of RepoInfo as repositories.
    /// Processes repos concurrently and send results as they are found
    pub async fn stream_repos(path: PathBuf, verbose: bool, _depth: u8) -> impl Stream<Item = RepoInfo> {
        let (tx, rx) = mpsc::channel(100);
        
        tokio::spawn(async move {
            let repo_paths = Self::get_repos(path);
            let re: Arc<Regex> = Arc::new(Regex::new(r"([^/]+$)").unwrap());
            
            let mut handles = Vec::new();
            
            for path in repo_paths {
                let tx_clone = tx.clone();
                let re_clone = re.clone();
                
                let handle = tokio::spawn(async move {
                    let repo_name = re_clone.find(&path).unwrap().as_str().to_string();
                    
                    let repo_info = tokio::task::spawn_blocking(move || { 
                        Self::find_repo_info(&path, &repo_name, verbose)
                    }).await;
                    
                    if let Ok(Some(repo_info)) = repo_info {
                        let _ = tx_clone.send(repo_info).await;
                    }
                });
                
                handles.push(handle);
            }
            
            for handle in handles {
                let _ = handle.await;
            }
        });
        
        ReceiverStream::new(rx)
    }

    /// Collects info for all repos inside a dir tree
    pub fn collect_repos(repo_list: Vec<String>, verbose: bool, _depth: u8) -> Vec<RepoInfo> {
        //name extraction for the repo will not work if it has a slash on it, but whatever.
        let re: Arc<Regex> = Arc::new(Regex::new(r"([^/]+$)").unwrap());
        let mut repos = Vec::new();

        for path in repo_list {
            let reg = re.clone(); // new ref
            let thread = thread::spawn( move || {
                let repo_name = reg.clone().find(&path).unwrap().as_str();

                Self::find_repo_info(&path, &repo_name, verbose).unwrap()
            });
            repos.push(thread.join().unwrap());
        }

        // sort repositories, by total changes descending, with unchanged ones going last, sorted alphabetically
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

    fn find_repo_info(path: &str, repo_name: &str, verbose: bool) -> Option<RepoInfo> {
        let repo = gix::open(path).ok()?;
        
        let branch = match repo.head() {
            Ok(head) => {
                match head.referent_name() {
                    Some(name) => {
                        let full_name = name.as_bstr().to_string();
                        full_name.strip_prefix("refs/heads/").unwrap_or(&full_name).to_string()
                    }
                    None => "HEAD".to_string(),
                }
            }
            _ => "HEAD".to_string(),
        };

        let mut new_files = Vec::new();
        let mut added_files = Vec::new();
        let mut modified_files = Vec::new();
        let mut deleted_files = Vec::new();

        // Use simple dirty check and parse output manually to match git status --short
        if let Ok(is_dirty) = repo.is_dirty() {
            if is_dirty {
                // Fallback to git command for now to maintain compatibility
                let output = std::process::Command::new("git")
                    .args(["-C", path, "status", "--porcelain"])
                    .output();
                
                if let Ok(output) = output {
                    let status = String::from_utf8_lossy(&output.stdout);
                    for line in status.lines() {
                        if line.len() >= 3 {
                            let status_code = &line[..2];
                            let file_path = &line[3..];
                            
                            match status_code {
                                "??" => new_files.push(file_path.to_string()),
                                "A " | "AM" => added_files.push(file_path.to_string()),
                                " M" | "MM" | "M " => modified_files.push(file_path.to_string()),
                                " D" | "D " => deleted_files.push(file_path.to_string()),
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        if verbose {
            Some(RepoInfo {
                name: repo_name.to_string(),
                branch,
                new_files: FileTracker::new("New", new_files.len(), Some(new_files)),
                added_files: FileTracker::new("Added", added_files.len(), Some(added_files)),
                modified_files: FileTracker::new("Modified", modified_files.len(), Some(modified_files)),
                deleted_files: FileTracker::new("Deleted", deleted_files.len(), Some(deleted_files)),
            })
        } else {
            Some(RepoInfo {
                name: repo_name.to_string(),
                branch,
                new_files: FileTracker::new("??", new_files.len(), None),
                added_files: FileTracker::new("A", added_files.len(), None),
                modified_files: FileTracker::new("M", modified_files.len(), None),
                deleted_files: FileTracker::new("D", deleted_files.len(), None),
            })
        }
    } 


}

