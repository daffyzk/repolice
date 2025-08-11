use std::env;
use std::path::PathBuf;
use printer::Printer;
use reader::Reader;
use reader::RepoInfo;
use clap::Parser;

mod printer;
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
    #[arg(short, long)]
    verbose: bool,
    
    /// Display the status for a repository if it has new files or branches
    #[arg(short, long)]
    fetch: bool,

    /// Disable TUI and print to stdout instead
    #[arg(long)]
    no_tui: bool,

}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    
    let mut exec_path : PathBuf = env::current_dir().unwrap();  // cwd by default
    let mut exec_depth : u8 = 10; 
    let exec_no_tui : bool = args.no_tui;
    let exec_verbose : bool = args.verbose; 
    let exec_fetch : bool = false;

    match args.path{
        Some(p) => {exec_path = PathBuf::from(p)},
        None => {},
    }

    match args.depth{
        Some(d) => {exec_depth = d; println!("depth = {}, {}", d, exec_depth)},
        None => {},
    }

    if args.fetch {
        println!("fetch = {}", exec_fetch)
    }
    if exec_no_tui {
        let repos: Vec<RepoInfo> = Reader::collect_repos(Reader::get_repos(exec_path.clone()), exec_verbose, exec_depth);
        Printer::print_repos(repos, exec_verbose);
    } else {
        let repo_stream = Reader::stream_repos(exec_path.clone(), exec_verbose, exec_depth).await;
        match tui::run_streaming_tui(repo_stream, exec_verbose).await {
            Ok(_) => {},
            Err(_) => {
                println!("TUI failed, falling back to printed output...");
                let repos: Vec<RepoInfo> = Reader::collect_repos(Reader::get_repos(exec_path), exec_verbose, exec_depth);
                Printer::print_repos(repos, exec_verbose);
            }
        }
    }
}


