use anyhow::Result;
use chrono::Local;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures_util::StreamExt;
use std::io;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Cell, Row, Table},
    Frame, Terminal,
};

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkerData {
    name: String,
    busy: bool,
    #[serde(default)]
    extra: Option<serde_json::Value>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct StatusMessage {
    #[serde(rename = "type")]
    msg_type: String,
    #[serde(default)]
    data: Option<Vec<WorkerData>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Connect to the WebSocket endpoint
    let (mut socket, _) = connect_async("ws://127.0.0.1:8080/ws/status").await?;
    println!("Connected to /ws/status\nPress 'q' to quit.");

    // Create channel for data between websocket and UI thread
    let (tx, mut rx) = mpsc::channel::<Vec<WorkerData>>(16);

    // Spawn background task: receive WS messages
    tokio::spawn(async move {
        while let Some(Ok(msg)) = socket.next().await {
            if let tokio_tungstenite::tungstenite::Message::Text(txt) = msg {
                if let Ok(parsed) = serde_json::from_str::<StatusMessage>(&txt) {
                    if parsed.msg_type == "PoolStatus" {
                        if let Some(data) = parsed.data {
                            let _ = tx.send(data).await;
                        }
                    }
                }
            }
        }
    });

    // Setup terminal UI
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let tick_rate = Duration::from_millis(200);
    let mut last_data: Vec<WorkerData> = Vec::new();

    loop {
        // Draw UI
        terminal.draw(|f| ui(f, &last_data))?;

        // Handle keyboard + updates
        let timeout = tick_rate;
        let now = Instant::now();

        // Wait for keyboard or incoming data
        if crossterm::event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }

        if let Ok(Some(new_data)) = rx.try_recv().map(Some) {
            last_data = new_data;
        }


        if now.elapsed() > Duration::from_secs(5) {
            // optional: refresh info periodically
        }
    }

    // cleanup terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

fn ui<B: Backend>(f: &mut Frame<B>, data: &[WorkerData]) {
    let size = f.size();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Percentage(100)].as_ref())
        .split(size);

    let header_cells = ["Worker", "Busy", "PID", "GPU", "Mem (MB)", "Uptime", "Model"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow)));
    let header = Row::new(header_cells)
        .style(Style::default().add_modifier(Modifier::BOLD))
        .height(1);

    let rows = data.iter().map(|w| {
        let e = w.extra.as_ref().and_then(|x| x.as_object());
        let pid = e.and_then(|m| m.get("pid")).and_then(|x| x.as_i64()).unwrap_or(0);
        let gpu_mem = e.and_then(|m| m.get("gpu_mem_used")).and_then(|x| x.as_i64()).unwrap_or(0);
        let model = e.and_then(|m| m.get("model")).and_then(|x| x.as_str()).unwrap_or("-");
        let uptime = e.and_then(|m| m.get("uptime")).and_then(|x| x.as_i64()).unwrap_or(0);
        let gpu = e.and_then(|m| m.get("gpu")).and_then(|x| x.as_str()).unwrap_or("-");
        Row::new(vec![
            Cell::from(w.name.clone()),
            Cell::from(if w.busy { "yes" } else { "no" }),
            Cell::from(pid.to_string()),
            Cell::from(gpu.to_string()),
            Cell::from(gpu_mem.to_string()),
            Cell::from(format!("{}s", uptime)),
            Cell::from(model.to_string()),
        ])
    });

    let table = Table::new(rows)
        .header(header)
        .block(Block::default()
            .title(Span::styled(
                format!("AI-SMI  @ {}", Local::now().format("%H:%M:%S")),
                Style::default().add_modifier(Modifier::BOLD)
            ))
            .borders(Borders::ALL))
        .widths(&[
            Constraint::Length(12),
            Constraint::Length(6),
            Constraint::Length(6),
            Constraint::Length(6),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Min(20),
        ]);

    f.render_widget(table, chunks[0]);
}
