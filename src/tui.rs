use std::io;
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

#[derive(Clone)]
pub struct RepoInfo {
    pub name: String,
    pub branch: String,
    pub new_files: String,
    pub added_files: String,
    pub modified_files: String,
    pub deleted_files: String,
    pub verbose_info: String,
}

pub struct App {
    pub repos: Vec<RepoInfo>,
    pub simple: bool,
    pub scroll_offset: usize,
}

impl App {
    pub fn new(repos: Vec<RepoInfo>, simple: bool) -> App {
        App { repos, simple, scroll_offset: 0 }
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

pub fn run_app() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let repos = vec![]; // This will be populated from main
    let app = App::new(repos, true);
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

pub fn run_tui_with_repos(repos: Vec<RepoInfo>, simple: bool) -> Result<(), Box<dyn std::error::Error>> {
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

    // Title with scroll status
    let total_rows = (app.repos.len() + cols - 1) / cols;
    let title_text = if total_rows > visible_rows {
        format!("Repolice (Showing rows {}-{} of {})", 
                app.scroll_offset + 1, 
                (app.scroll_offset + visible_rows).min(total_rows), 
                total_rows)
    } else {
        "Repolice".to_string()
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

fn render_repo_widget(f: &mut Frame, area: Rect, repo: &RepoInfo, simple: bool) {
    let content = if simple {
        vec![
            Line::from(vec![
                Span::styled(&repo.name, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled(format!("[{}]", &repo.branch), Style::default().fg(Color::Green)),
            ]),
            Line::from(vec![
                Span::styled(format!("?{} ", &repo.new_files), Style::default().fg(Color::Blue)),
                Span::styled(format!("+{} ", &repo.added_files), Style::default().fg(Color::Green)),
                Span::styled(format!("~{} ", &repo.modified_files), Style::default().fg(Color::Yellow)),
                Span::styled(format!("-{}", &repo.deleted_files), Style::default().fg(Color::Red)),
            ]),
        ]
    } else {
        let mut lines = vec![
            Line::from(vec![
                Span::styled(&repo.name, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled(format!("[{}]", &repo.branch), Style::default().fg(Color::Green)),
            ]),
            Line::from(vec![
                Span::styled(format!("?{} ", &repo.new_files), Style::default().fg(Color::Blue)),
                Span::styled(format!("+{} ", &repo.added_files), Style::default().fg(Color::Green)),
                Span::styled(format!("~{} ", &repo.modified_files), Style::default().fg(Color::Yellow)),
                Span::styled(format!("-{}", &repo.deleted_files), Style::default().fg(Color::Red)),
            ]),
        ];
        
        // Add verbose info lines
        for line in repo.verbose_info.lines() {
            lines.push(Line::from(line.to_string()));
        }
        
        lines
    };

    let paragraph = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL))
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}
