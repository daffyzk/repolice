use crate::reader::RepoInfo;

use std::io;
use std::time::Duration;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame, Terminal,
};
use tokio_stream::StreamExt;
use futures::stream::Stream;


pub struct App {
    pub repos: Vec<RepoInfo>,
    pub verbose: bool,
    pub scroll_offset: usize,
    pub loading: bool,
    pub total_found: usize,
    pub clean_scroll_offset: usize,
}

impl App {
    pub fn add_repo(&mut self, repo: RepoInfo) {
        self.repos.push(repo);
        self.sort_repos();
        self.total_found = self.repos.len();
    }

    pub fn set_loading_complete(&mut self) {
        self.loading = false;
    }

    fn sort_repos(&mut self) {
        self.repos.sort_by(|a, b| {
            match (a.has_changes(), b.has_changes()) {
                (true, false) => std::cmp::Ordering::Less,    // repos with changes come first
                (false, true) => std::cmp::Ordering::Greater, // clean repos come last
                (true, true) => b.total_changes().cmp(&a.total_changes()), // sort by most changes first
                (false, false) => a.name.cmp(&b.name), // clean repos sorted alphabetically
            }
        });
    }
    pub fn new(verbose: bool) -> App {
        App { 
            repos: Vec::new(),
            verbose, 
            scroll_offset: 0,
            loading: true,
            total_found: 0,
            clean_scroll_offset: 0,
        }
    }

    pub fn scroll_down(&mut self, cols: usize, available_height: usize) {
        let repos_with_changes: Vec<_> = self.repos.iter().filter(|r| r.has_changes()).collect();
        let total_rows = (repos_with_changes.len() + cols - 1) / cols;
        
        let estimated_visible_rows = (available_height / 6).max(1); // estimate
        
        if self.scroll_offset + estimated_visible_rows < total_rows {
            self.scroll_offset += 1;
        }
    }

    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    pub fn scroll_clean_left(&mut self) {
        if self.clean_scroll_offset > 0 {
            self.clean_scroll_offset -= 1;
        }
    }

    pub fn scroll_clean_right(&mut self, visible_clean_repos: usize) {
        let clean_repos: Vec<_> = self.repos.iter().filter(|r| !r.has_changes()).collect();
        if self.clean_scroll_offset + visible_clean_repos < clean_repos.len() {
            self.clean_scroll_offset += 1;
        }
    }
}

pub async fn run_streaming_tui<S>(repo_stream: S, verbose: bool) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    S: Stream<Item = RepoInfo> + Unpin,
{
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = App::new(verbose);
    let res = run_streaming_app_loop(&mut terminal, app, repo_stream).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

async fn run_streaming_app_loop<B: Backend, S>(
    terminal: &mut Terminal<B>,
    mut app: App,
    mut repo_stream: S,
) -> io::Result<()>
where
    S: Stream<Item = RepoInfo> + Unpin,
{
    let mut last_render = std::time::Instant::now();
    let render_interval = Duration::from_millis(100); // Render at most 10 times per second
    
    loop {
        let size = terminal.size()?;
        let cols = 4;
        let available_height = size.height.saturating_sub(10); // More space for dynamic content
        
        // Check for new repos from the stream (non-blocking)
        match tokio::time::timeout(Duration::from_millis(10), repo_stream.next()).await {
            Ok(Some(repo_info)) => {
                app.add_repo(repo_info);
            }
            Ok(None) => {
                // Stream is exhausted
                app.set_loading_complete();
            }
            Err(_) => {
                // Timeout - no new repos in this cycle, continue
            }
        }
        
        // Throttle rendering to avoid excessive redraws
        if last_render.elapsed() >= render_interval {
            terminal.draw(|f| ui(f, &app, cols, available_height))?;
            last_render = std::time::Instant::now();
        }

        // Check for user input (non-blocking)
        if let Ok(true) = event::poll(Duration::from_millis(50)) {
            if let Event::Key(key) = event::read()? {
                let visible_clean_repos = (size.width / 12).max(1) as usize; // Estimate how many clean repos fit
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Down => app.scroll_down(cols, available_height as usize),
                    KeyCode::Up => app.scroll_up(),
                    KeyCode::Left => app.scroll_clean_left(),
                    KeyCode::Right => app.scroll_clean_right(visible_clean_repos),
                    _ => {}
                }
            }
        }
        
        // break if loading is complete and stream is exhausted
        if !app.loading {
            if let Ok(true) = event::poll(Duration::from_millis(100)) {
                if let Event::Key(key) = event::read()? {
                    let visible_clean_repos = (size.width / 12).max(1) as usize;
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Down => app.scroll_down(cols, available_height as usize),
                        KeyCode::Up => app.scroll_up(),
                        KeyCode::Left => app.scroll_clean_left(),
                        KeyCode::Right => app.scroll_clean_right(visible_clean_repos),
                        _ => {}
                    }
                }
            }
            terminal.draw(|f| ui(f, &app, cols, available_height))?;
        }
    }
}

fn ui(f: &mut Frame, app: &App, cols: usize, available_height: u16) {
    let size = f.area();

    // Separate repos with changes from clean repos
    let repos_with_changes: Vec<_> = app.repos.iter().filter(|r| r.has_changes()).collect();
    let clean_repos: Vec<_> = app.repos.iter().filter(|r| !r.has_changes()).collect();

    // create main layout with title, main content, clean repos footer, and instructions
    let constraints = if clean_repos.is_empty() {
        vec![Constraint::Length(3), Constraint::Min(0), Constraint::Length(1)]
    } else {
        vec![Constraint::Length(3), Constraint::Min(0), Constraint::Length(3), Constraint::Length(1)]
    };
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(constraints)
        .split(size);

    // title with scroll status and loading indicator
    let total_rows = (repos_with_changes.len() + cols - 1) / cols;
    let estimated_visible_rows = (available_height / 6).max(1) as usize;
    let title_text = if app.loading {
        format!("Repolice - Loading repositories... ({} found)", app.total_found)
    } else if total_rows > estimated_visible_rows {
        format!("Repolice - Repos with changes (Scroll: {}/{})", 
                app.scroll_offset + 1, 
                total_rows)
    } else {
        format!("Repolice ({} with changes, {} clean)", repos_with_changes.len(), clean_repos.len())
    };
    
    let title = Paragraph::new(title_text)
        .style(Style::default().fg(Color::Cyan))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    // create grid layout for visible repos with changes only
    if !repos_with_changes.is_empty() {
        // Calculate how many repos can fit in the available height
        let mut current_height = 0u16;
        let mut visible_repos = Vec::new();
        let mut row_idx = app.scroll_offset;
        
        while current_height < available_height && row_idx * cols < repos_with_changes.len() {
            let mut max_height_in_row = 0u16;
            let mut repos_in_row = Vec::new();
            
            for col_idx in 0..cols {
                let repo_idx = row_idx * cols + col_idx;
                if repo_idx < repos_with_changes.len() {
                    let repo = repos_with_changes[repo_idx];
                    let repo_height = calculate_repo_height(repo, app.verbose);
                    max_height_in_row = max_height_in_row.max(repo_height);
                    repos_in_row.push(repo);
                }
            }
            
            if current_height + max_height_in_row <= available_height {
                visible_repos.extend(repos_in_row);
                current_height += max_height_in_row;
                row_idx += 1;
            } else {
                break;
            }
        }
        
        let actual_visible_rows = (visible_repos.len() + cols - 1) / cols;

        if actual_visible_rows > 0 {
            // calculate dynamic heights for each row based on content
            let mut row_heights = Vec::new();
            for row_idx in 0..actual_visible_rows {
                let mut max_height_in_row = 3; // Minimum height (name + branch + border)
                
                for col_idx in 0..cols {
                    let repo_idx = row_idx * cols + col_idx;
                    if repo_idx < visible_repos.len() {
                        let repo = visible_repos[repo_idx];
                        let repo_height = calculate_repo_height(repo, app.verbose);
                        max_height_in_row = max_height_in_row.max(repo_height);
                    }
                }
                row_heights.push(max_height_in_row);
            }
            
            // create constraints based on calculated heights
            let row_constraints: Vec<Constraint> = row_heights
                .iter()
                .map(|&height| Constraint::Length(height))
                .collect();

            let row_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(row_constraints)
                .split(chunks[1]);

            for (row_idx, row_chunk) in row_chunks.iter().enumerate() {
                let col_constraints = vec![Constraint::Percentage(25); 4];
                let col_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(col_constraints)
                    .split(*row_chunk);

                for col_idx in 0..cols {
                    let repo_idx = row_idx * cols + col_idx;
                    if repo_idx < visible_repos.len() {
                        let repo = visible_repos[repo_idx];
                        render_repo_widget(f, col_chunks[col_idx], repo, app.verbose);
                    }
                }
            }
        }
    } else {
        let no_repos = Paragraph::new("No repositories with changes")
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(no_repos, chunks[1]);
    }

    // only render clean repos footer if there are any
    if !clean_repos.is_empty() {
        render_clean_repos_footer(f, chunks[2], &clean_repos, app.clean_scroll_offset, size.width);
    }

    let instruction_text = if clean_repos.is_empty() {
        if total_rows > estimated_visible_rows {
            "Press ↑/↓ to scroll, 'q' to quit"
        } else {
            "Press 'q' to quit"
        }
    } else {
        if total_rows > estimated_visible_rows {
            "Press ↑/↓ to scroll repos, ←/→ to scroll clean repos, 'q' to quit"
        } else {
            "Press ←/→ to scroll clean repos, 'q' to quit"
        }
    };
    
    let instructions = Paragraph::new(instruction_text)
        .style(Style::default().fg(Color::Gray));
    let instruction_chunk = if clean_repos.is_empty() { chunks[2] } else { chunks[3] };
    f.render_widget(instructions, instruction_chunk);
}

fn calculate_repo_height(repo: &RepoInfo, verbose: bool) -> u16 {
    let mut height = 4; // base height: name + branch + borders
    
    if repo.has_changes() {
        if verbose {
            // in verbose mode, each file type gets its own line
            if repo.new_files.amount > 0 { height += 1; }
            if repo.added_files.amount > 0 { height += 1; }
            if repo.modified_files.amount > 0 { height += 1; }
            if repo.deleted_files.amount > 0 { height += 1; }
        } else {
            // in simple mode, all changes fit on one line
            height += 1;
        }
    } else {
        height += 1;
    }
    
    height
}

fn render_clean_repos_footer(f: &mut Frame, area: Rect, clean_repos: &[&RepoInfo], scroll_offset: usize, terminal_width: u16) {
    let repo_width = 12; // Each clean repo takes 12 characters
    let visible_count = (terminal_width / repo_width).max(1) as usize;
    let start_idx = scroll_offset;
    let end_idx = (start_idx + visible_count).min(clean_repos.len());
    let visible_clean_repos = &clean_repos[start_idx..end_idx];
    
    let mut spans = vec![];
    for (i, repo) in visible_clean_repos.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" "));
        }
        spans.push(Span::styled(
            format!("[{}]", &repo.name),
            Style::default().fg(Color::Green)
        ));
    }
    
    let scroll_indicator = if clean_repos.len() > visible_count {
        format!(" ({}/{} clean)", end_idx, clean_repos.len())
    } else {
        format!(" ({} clean)", clean_repos.len())
    };
    
    spans.push(Span::styled(scroll_indicator, Style::default().fg(Color::Gray)));
    
    let content = Line::from(spans);
    let paragraph = Paragraph::new(vec![content])
        .block(Block::default().borders(Borders::ALL).title("Clean Repositories"));
    
    f.render_widget(paragraph, area);
}

fn render_repo_widget(f: &mut Frame, area: Rect, repo: &RepoInfo, verbose: bool) {
    let title = Line::from(vec![
        Span::styled(&repo.name, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    ]);
    let branch = Line::from(vec![
        Span::styled(format!("[{}]", &repo.branch), Style::default().fg(Color::Green)),
    ]);
    let changes = |repo: &RepoInfo| -> Vec<Line> {
        if repo.has_changes() {
            if verbose {
                vec![
                    Line::from(vec![Span::styled(
                        format!("{}: {}", &repo.new_files.status, &repo.new_files.amount), 
                        Style::default().fg(Color::Blue))]),
                    Line::from(vec![Span::styled(
                        format!("{}: {}", &repo.new_files.status, &repo.new_files.amount), 
                        Style::default().fg(Color::Blue))]),
                        //TODO: for each of the files, make a new Line with the file name and color

                    Line::from(vec![Span::styled(
                        format!("{}: {}", &repo.added_files.status, &repo.added_files.amount), 
                        Style::default().fg(Color::Green))]),
                        //TODO: for each of the files, make a new Line with the file name and color
                    Line::from(vec![Span::styled(
                        format!("{}: {}", &repo.modified_files.status, &repo.modified_files.amount), 
                        Style::default().fg(Color::Yellow))]),
                        //TODO: for each of the files, make a new Line with the file name and color
                    Line::from(vec![Span::styled(
                        format!("{}: {}", &repo.deleted_files.status, &repo.deleted_files.amount),
                        Style::default().fg(Color::Red))]),
                        //TODO: for each of the files, make a new Line with the file name and color
                ]
            } else {
                vec![Line::from(vec![
                    Span::styled(
                        format!("{}:{} ", &repo.new_files.status, &repo.new_files.amount), 
                        Style::default().fg(Color::Blue)),
                    Span::styled(
                        format!("{}:{} ", &repo.added_files.status, &repo.added_files.amount), 
                        Style::default().fg(Color::Green)),
                    Span::styled(
                        format!("{}:{} ", &repo.modified_files.status, &repo.modified_files.amount), 
                        Style::default().fg(Color::Yellow)),
                    Span::styled(
                        format!("{}:{} ", &repo.deleted_files.status, &repo.deleted_files.amount),
                        Style::default().fg(Color::Red)),
                ])]
            }
        } else {
            vec![Line::from(
                Span::styled("Nothing new here!", Style::default().fg(Color::LightCyan).add_modifier(Modifier::ITALIC))
            )]
        }
    };

    let content: Vec<Line> = vec![title, branch, changes(repo).into_iter().flatten().collect()];

    let paragraph = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL))
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}
