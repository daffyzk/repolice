use crate::reader::RepoInfo;


pub struct Printer {}

impl Printer {

    pub fn print_repos(repos: Vec<RepoInfo>, verbose: bool) {
        let unchanged: Vec<String> = vec![];
        for repo in repos {
            if repo.has_changes() {
                if verbose {
                    println!("| {}: [{}]", repo.name, repo.branch);
                    Self::get_verbose_format(repo);
                } else {
                    println!("| {}: [{}]", repo.name, repo.branch);
                    println!("| ?{} | +{} | ~{} | -{} |", 
                        repo.new_files.amount, 
                        repo.added_files.amount, 
                        repo.modified_files.amount, 
                        repo.deleted_files.amount);
                } 
            }
        }
    }

    fn get_verbose_format(repo: RepoInfo) {
        // print new, added, modified, and deleted only if there are matches
        if repo.has_changes() { 
            if repo.new_files.files.is_some() {
                println!("New");
                Self::formatted_list(&repo.new_files.files.unwrap());
            }
            if repo.added_files.files.is_some() {
                println!("Added");
                Self::formatted_list(&repo.added_files.files.unwrap());
            }
            if repo.modified_files.files.is_some() {
                println!("Modified");
                Self::formatted_list(&repo.modified_files.files.unwrap());
            }
            if repo.deleted_files.files.is_some() {
                println!("Deleted");
                Self::formatted_list(&repo.deleted_files.files.unwrap());
            }
        } else {
            println!("Nothing new!");
        } 
    }

    fn formatted_list(list: &Vec<String>) {
        for item in list {
            println!("| _ {}", item);
        }
    }
}
