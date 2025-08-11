use crate::reader::RepoInfo;



pub fn print_repos(repos: Vec<RepoInfo>, simple: bool) {
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

