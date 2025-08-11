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
    pub simple: bool,
    pub scroll_offset: usize,
    pub loading: bool,
    pub total_found: usize,
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
    pub fn new_streaming(simple: bool) -> App {
        App { 
            repos: Vec::new(),
            simple, 
            scroll_offset: 0,
            loading: true,
            total_found: 0,
        }
    }

    pub fn new(repos: &Vec<RepoInfo>, simple: bool) -> App {
        App { 
            repos: repos.clone(),               // cloning here for the memes 
            simple, 
            scroll_offset: 0,
            loading: false,
            total_found: repos.len(),
        }
    }

    pub fn scroll_down(&mut self, cols: usize, visible_rows: usize) {
        let total_rows = (self.repos.len() + cols - 1) / cols;
        if self.scroll_offset + visible_rows < total_rows {
            self.scroll_offset += 1;
        }
    }

    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }
}

pub fn run_tui_with_repos(repos: &Vec<RepoInfo>, simple: bool) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = App::new(repos, simple);
    let res = run_app_loop(&mut terminal, app);

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

pub async fn run_streaming_tui<S>(repo_stream: S, simple: bool) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    S: Stream<Item = RepoInfo> + Unpin,
{
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = App::new_streaming(simple);
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
        let cols = 3;
        let available_height = size.height.saturating_sub(6);
        let visible_rows = (available_height / 6).max(1) as usize;
        
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
            terminal.draw(|f| ui(f, &app, cols, visible_rows))?;
            last_render = std::time::Instant::now();
        }

        // Check for user input (non-blocking)
        if let Ok(true) = event::poll(Duration::from_millis(50)) {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Down => app.scroll_down(cols, visible_rows),
                    KeyCode::Up => app.scroll_up(),
                    _ => {}
                }
            }
        }
        
        // Break if loading is complete and stream is exhausted
        if !app.loading {
            // Continue handling input after loading is complete
            if let Ok(true) = event::poll(Duration::from_millis(100)) {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Down => app.scroll_down(cols, visible_rows),
                        KeyCode::Up => app.scroll_up(),
                        _ => {}
                    }
                }
            }
            // Final render after loading complete
            terminal.draw(|f| ui(f, &app, cols, visible_rows))?;
        }
    }
}

fn run_app_loop<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
) -> io::Result<()> {
    loop {
        let size = terminal.size()?;
        let cols = 3;
        let available_height = size.height.saturating_sub(6);
        let visible_rows = (available_height / 6).max(1) as usize;
        
        terminal.draw(|f| ui(f, &app, cols, visible_rows))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Down => app.scroll_down(cols, visible_rows),
                KeyCode::Up => app.scroll_up(),
                _ => {}
            }
        }
    }
}

fn ui(f: &mut Frame, app: &App, cols: usize, visible_rows: usize) {
    let size = f.area();

    // Create main layout with title and status
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(1)].as_ref())
        .split(size);

    // Title with scroll status and loading indicator
    let total_rows = (app.repos.len() + cols - 1) / cols;
    let title_text = if app.loading {
        format!("Repolice - Loading repositories... ({} found)", app.total_found)
    } else if total_rows > visible_rows {
        format!("Repolice (Showing rows {}-{} of {})", 
                app.scroll_offset + 1, 
                (app.scroll_offset + visible_rows).min(total_rows), 
                total_rows)
    } else {
        format!("Repolice ({} repositories)", app.total_found)
    };
    
    let title = Paragraph::new(title_text)
        .style(Style::default().fg(Color::Cyan))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    // Create grid layout for visible repos only
    if !app.repos.is_empty() {
        let start_repo = app.scroll_offset * cols;
        let end_repo = (start_repo + (visible_rows * cols)).min(app.repos.len());
        let visible_repos = &app.repos[start_repo..end_repo];
        let actual_visible_rows = (visible_repos.len() + cols - 1) / cols;

        if actual_visible_rows > 0 {
            let row_constraints: Vec<Constraint> = (0..actual_visible_rows)
                .map(|_| Constraint::Percentage(100 / actual_visible_rows as u16))
                .collect();

            let row_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(row_constraints)
                .split(chunks[1]);

            for (row_idx, row_chunk) in row_chunks.iter().enumerate() {
                let col_constraints = vec![Constraint::Percentage(33); 3];
                let col_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(col_constraints)
                    .split(*row_chunk);

                for col_idx in 0..cols {
                    let repo_idx = row_idx * cols + col_idx;
                    if repo_idx < visible_repos.len() {
                        let repo = &visible_repos[repo_idx];
                        render_repo_widget(f, col_chunks[col_idx], repo, app.simple);
                    }
                }
            }
        }
    } else {
        let no_repos = Paragraph::new("No repositories found")
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(no_repos, chunks[1]);
    }

    // Instructions
    let instruction_text = if total_rows > visible_rows {
        "Press ↑/↓ to scroll, 'q' to quit"
    } else {
        "Press 'q' to quit"
    };
    
    let instructions = Paragraph::new(instruction_text)
        .style(Style::default().fg(Color::Gray));
    f.render_widget(instructions, chunks[2]);
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
