use std::path::PathBuf;
use std::process::{Stdio, Command, Output};
use to_vec::ToVec;

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
