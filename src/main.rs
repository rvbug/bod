use std::{
    io::{self, stdout},
    fs,
    path::Path,
    process::Command,
    time::Duration,
};
use ratatui::{
    backend::CrosstermBackend,
    widgets::{Block, Borders, Tabs, Paragraph, List, ListItem},
    layout::{Layout, Direction, Constraint},
    style::{Style, Color},
    text::{Line, Span},
    Terminal,
};
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use chrono::Local;

struct App {
    tabs: Vec<String>,
    current_tab: usize,
    show_content: bool,
    show_editor_selection: bool,
    selected_editor: usize,
    current_dir_contents: Vec<DirEntry>,
}

#[derive(Clone)]
struct DirEntry {
    name: String,
    is_dir: bool,
}

impl App {
    fn new() -> io::Result<App> {
        let path = Path::new("~/Documents/rakesh/projects").expand_home()?;
        let mut tabs = Vec::new();
        
        for entry in fs::read_dir(path)? {
            if let Ok(entry) = entry {
                if entry.file_type()?.is_dir() {
                    tabs.push(entry.file_name().to_string_lossy().into_owned());
                }
            }
        }
        
        Ok(App {
            tabs,
            current_tab: 0,
            show_content: false,
            show_editor_selection: false,
            selected_editor: 0,
            current_dir_contents: Vec::new(),
        })
    }

    fn update_current_dir_contents(&mut self) -> io::Result<()> {
        if self.tabs.is_empty() {
            return Ok(());
        }

        let base_path = Path::new("~/Documents/rakesh/projects").expand_home()?;
        let current_dir = base_path.join(&self.tabs[self.current_tab]);
        let mut contents = Vec::new();

        for entry in fs::read_dir(current_dir)? {
            if let Ok(entry) = entry {
                let file_type = entry.file_type()?;
                contents.push(DirEntry {
                    name: entry.file_name().to_string_lossy().into_owned(),
                    is_dir: file_type.is_dir(),
                });
            }
        }

        // Sort directories first, then files, both alphabetically
        contents.sort_by(|a, b| {
            match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });

        self.current_dir_contents = contents;
        Ok(())
    }
}

trait PathExt {
    fn expand_home(&self) -> io::Result<std::path::PathBuf>;
}

impl PathExt for Path {
    fn expand_home(&self) -> io::Result<std::path::PathBuf> {
        if let Some(path_str) = self.to_str() {
            if path_str.starts_with("~/") {
                if let Some(home) = dirs::home_dir() {
                    return Ok(home.join(&path_str[2..]));
                }
            }
        }
        Ok(self.to_path_buf())
    }
}

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;
    let mut app = App::new()?;
    
    loop {
        terminal.draw(|f| {
            let size = f.size();
            
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(0),
                ].as_ref())
                .split(size);
            
            // Top bar layout
            let top_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(33),
                    Constraint::Percentage(34),
                    Constraint::Percentage(33),
                ].as_ref())
                .split(chunks[0]);
            
            // Date
            let date = Local::now().format("%Y-%m-%d").to_string();
            f.render_widget(
                Paragraph::new(date)
                    .block(Block::default().borders(Borders::ALL)),
                top_chunks[0],
            );
            
            // Name
            f.render_widget(
                Paragraph::new("Rakesh")
                    .block(Block::default().borders(Borders::ALL)),
                top_chunks[1],
            );
            
            // Time
            let time = Local::now().format("%H:%M:%S").to_string();
            f.render_widget(
                Paragraph::new(time)
                    .block(Block::default().borders(Borders::ALL)),
                top_chunks[2],
            );
            
            // Tabs
            let tab_titles: Vec<String> = app.tabs
                .iter()
                .enumerate()
                .map(|(i, name)| format!("{}_{}", i + 1, name))
                .collect();
            
            let tabs = Tabs::new(tab_titles)
                .block(Block::default().borders(Borders::ALL))
                .select(app.current_tab)
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().fg(Color::Yellow));
            
            f.render_widget(tabs, chunks[1]);
            
            // Content area
            if app.show_content {
                let items: Vec<ListItem> = app.current_dir_contents
                    .iter()
                    .map(|entry| {
                        let prefix = if entry.is_dir { "📁 " } else { "📄 " };
                        let content = Line::from(vec![
                            Span::raw(prefix),
                            Span::styled(
                                &entry.name,
                                Style::default().fg(if entry.is_dir { Color::Blue } else { Color::White })
                            )
                        ]);
                        ListItem::new(content)
                    })
                    .collect();

                let list = List::new(items)
                    .block(Block::default()
                        .title(format!(" Contents of {} ", app.tabs[app.current_tab]))
                        .borders(Borders::ALL));

                f.render_widget(list, chunks[2]);
            }
            
            // Editor selection popup
            if app.show_editor_selection {
                let popup_block = Block::default()
                    .borders(Borders::ALL)
                    .title("Select Editor");
                
                let editors = vec!["VSCode", "Neovim"];
                let editor_text = editors.join("\n");
                let selected_style = Style::default().fg(Color::Yellow);
                
                let popup = Paragraph::new(editor_text)
                    .block(popup_block)
                    .style(Style::default());
                
                let area = centered_rect(30, 20, size);
                f.render_widget(popup, area);
            }
        })?;
        
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char(num) => {
                        if let Some(digit) = num.to_digit(10) {
                            let index = digit as usize - 1;
                            if index < app.tabs.len() {
                                app.current_tab = index;
                                app.show_content = true;
                                app.update_current_dir_contents()?;
                            }
                        }
                    },
                    KeyCode::Char('c') => app.show_content = false,
                    KeyCode::Char('o') => app.show_editor_selection = true,
                    KeyCode::Up if app.show_editor_selection => {
                        app.selected_editor = 0;
                    },
                    KeyCode::Down if app.show_editor_selection => {
                        app.selected_editor = 1;
                    },
                    KeyCode::Enter if app.show_editor_selection => {
                        let path = Path::new("~/Documents/rakesh/projects")
                            .expand_home()?
                            .join(&app.tabs[app.current_tab]);
                        
                        match app.selected_editor {
                            0 => { // VSCode
                                Command::new("code")
                                    .arg(path)
                                    .spawn()?;
                            },
                            1 => { // Neovim
                                Command::new("nvim")
                                    .arg(path)
                                    .spawn()?;
                            },
                            _ => {},
                        }
                        app.show_editor_selection = false;
                    },
                    KeyCode::Esc => app.show_editor_selection = false,
                    _ => {},
                }
            }
        }
    }
    
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

fn centered_rect(percent_x: u16, percent_y: u16, r: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ].as_ref())
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ].as_ref())
        .split(popup_layout[1])[1]
}
