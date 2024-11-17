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
    style::{Style, Color, Modifier},
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
    selected_item: Option<usize>,
    show_confirmation: bool,
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
        
        let mut app = App {
            tabs,
            current_tab: 0,
            show_content: true,  // Set to true by default
            show_editor_selection: false,
            selected_editor: 0,
            current_dir_contents: Vec::new(),
            selected_item: None,
            show_confirmation: false,
        };
        
        // Initialize directory contents
        app.update_current_dir_contents()?;
        
        Ok(app)
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

    fn switch_tab(&mut self, tab_index: usize) -> io::Result<()> {
        if tab_index < self.tabs.len() {
            self.current_tab = tab_index;
            self.selected_item = None;
            self.update_current_dir_contents()?;
        }
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

            // Keyboard shortcuts
            let shortcuts = vec![
                Span::styled("1-9", Style::default().fg(Color::Yellow)),
                Span::raw(": Switch Tabs | "),
                Span::styled("↑/↓", Style::default().fg(Color::Yellow)),
                Span::raw(": Navigate | "),
                Span::styled("Enter", Style::default().fg(Color::Yellow)),
                Span::raw(": Select | "),
                Span::styled("y/n", Style::default().fg(Color::Yellow)),
                Span::raw(": Confirm | "),
                Span::styled("q", Style::default().fg(Color::Yellow)),
                Span::raw(": Quit"),
            ];
        
            f.render_widget(
                Paragraph::new(Line::from(shortcuts))
                    .block(Block::default().borders(Borders::ALL))
                    .style(Style::default().fg(Color::White)),
                chunks[2],
            );
            
            // Content area
            if app.show_content {
                let items: Vec<ListItem> = app.current_dir_contents
                    .iter()
                    .enumerate()
                    .map(|(index, entry)| {
                        let is_selected = app.selected_item == Some(index);
                        let (icon, color) = if entry.is_dir {
                            ("📁", Color::Cyan)
                        } else {
                            ("📄", Color::White)
                        };
                        
                        let style = if is_selected {
                            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(color)
                        };
                        
                        let content = Line::from(vec![
                            Span::raw(icon),
                            Span::raw(" "),
                            Span::styled(&entry.name, style)
                        ]);
                        ListItem::new(content)
                    })
                    .collect();

                let list = List::new(items)
                    .block(Block::default()
                        .title(format!(" Contents of {} ", app.tabs[app.current_tab]))
                        .borders(Borders::ALL));

                f.render_widget(list, chunks[3]);
            }

            // Add confirmation popup if needed
            if app.show_confirmation {
                let popup = Paragraph::new("Open in Neovim? (y/n)")
                    .block(Block::default()
                        .borders(Borders::ALL)
                        .style(Style::default().fg(Color::Yellow)));
                
                let area = centered_rect(30, 20, size);
                f.render_widget(popup, area);
            }
            
            // Editor selection popup
            if app.show_editor_selection {
                let popup_block = Block::default()
                    .borders(Borders::ALL)
                    .title("Select Editor");
                
                let editors = vec!["VSCode", "Neovim"];
                let editor_text = editors.join("\n");
                let popup = Paragraph::new(editor_text)
                    .block(popup_block)
                    .style(Style::default());
                
                let area = centered_rect(30, 20, size);
                f.render_widget(popup, area);
            }
        })?;
        
    // ******************************** start ***********************************************
        
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char(c) => {
                        // Handle number keys 1-9 for tab switching
                        if let Some(digit) = c.to_digit(10) {
                            if digit > 0 && digit <= 9 {
                                app.switch_tab((digit - 1) as usize)?;
                            }
                        }
                    },
                    KeyCode::Up => {
                        if app.show_content {
                            if let Some(selected) = app.selected_item {
                                if selected > 0 {
                                    app.selected_item = Some(selected - 1);
                                }
                            } else {
                                app.selected_item = Some(0);
                            }
                        }
                    },
                    KeyCode::Down => {
                        if app.show_content {
                            if let Some(selected) = app.selected_item {
                                if selected < app.current_dir_contents.len() - 1 {
                                    app.selected_item = Some(selected + 1);
                                }
                            } else {
                                app.selected_item = Some(0);
                            }
                        }
                    },
                    KeyCode::Enter => {
                        if app.show_content && app.selected_item.is_some() {
                            app.show_confirmation = false;
                        }
                    },
                    KeyCode::Char('y') if app.show_confirmation => {
                        if let Some(selected) = app.selected_item {
                            let entry = &app.current_dir_contents[selected];
                            let path = Path::new("~/Documents/rakesh/projects")
                                .expand_home()?
                                .join(&app.tabs[app.current_tab])
                                .join(&entry.name);
                            
                            Command::new("alacritty")
                                .args(&["-e", "nvim"])
                                .arg(path)
                                .spawn()?;
                        }
                        app.show_confirmation = false;
                    },
                    KeyCode::Char('n') if app.show_confirmation => {
                        app.show_confirmation = false;
                    },
                    KeyCode::Esc => app.show_editor_selection = false,
                    _ => {},
                }
            }
        }





    // ******************************** END ***********************************************

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
