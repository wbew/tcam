use std::io;
use std::time::{Duration, Instant};

use clap::Parser;
use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
};
use image;
use nokhwa::{
    Camera,
    pixel_format::RgbFormat,
    utils::{CameraIndex, RequestedFormat, RequestedFormatType},
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    prelude::CrosstermBackend,
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph},
};
use ratatui_image::{StatefulImage, picker::Picker, protocol::StatefulProtocol};

#[derive(Parser)]
#[command(name = "tcam", version = "1.0.0", about = "terminal camera")]
struct Cli {}

fn main() {
    let _ = Cli::parse();

    let picker = Picker::halfblocks();
    crossterm::terminal::enable_raw_mode().expect("Can't enable raw mode");
    std::io::stdout()
        .execute(crossterm::terminal::EnterAlternateScreen)
        .expect("Can't enter alternate screen");

    let mut terminal = ratatui::Terminal::new(CrosstermBackend::new(std::io::stdout()))
        .expect("Can't create terminal");
    let mut camera = Camera::new(
        CameraIndex::default(),
        RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate),
    )
    .expect("No camera?");
    camera.open_stream().expect("Failed to use camera");

    let result = run(&mut terminal, &mut camera, &picker);

    crossterm::terminal::disable_raw_mode().expect("Can't disable raw mode");
    std::io::stdout()
        .execute(crossterm::terminal::LeaveAlternateScreen)
        .expect("Can't leave alternate screen");

    if let Err(e) = result {
        eprintln!("Error: {}", e);
    }
    std::process::exit(0);
}

fn take_photo(camera: &mut Camera) -> Result<(), io::Error> {
    // Warm up - discard first ~30 frames for auto-exposure
    for _ in 0..30 {
        let _ = camera.frame();
    }

    let frame = camera.frame().expect("Failed to take a frame");
    let decoded = frame.decode_image::<RgbFormat>().expect("Failed to decode");
    decoded
        .save(&format!(
            "capture-{}.png",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        ))
        .expect("Failed to save");

    Ok(())
}

fn render(frame: &mut Frame, image_state: &mut StatefulProtocol, status: &str) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
        .split(frame.area());

    frame.render_stateful_widget(StatefulImage::new(), chunks[0], image_state);

    let block = Block::default()
        .title(" 📷 tcam ")
        .borders(Borders::ALL)
        .border_style(ratatui::style::Style::default().fg(ratatui::style::Color::Cyan));

    let text = Paragraph::new(Text::from(vec![
        Line::from(status),
        Line::from("[SPACE] Take Photo  [Q] Quit"),
    ]))
    .block(block)
    .alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(text, chunks[1]);
}

fn run(
    terminal: &mut ratatui::Terminal<CrosstermBackend<io::Stdout>>,
    camera: &mut Camera,
    picker: &Picker,
) -> io::Result<()> {
    let mut status = "";
    let mut status_set_at: Option<Instant> = None;

    loop {
        // Clear status after timeout
        if let Some(t) = status_set_at {
            if t.elapsed() > Duration::from_secs(3) {
                status = "";
                status_set_at = None;
            }
        }

        let frame = camera.frame().expect("Failed to get frame");
        let decoded = frame.decode_image::<RgbFormat>().expect("Failed to decode");
        let img = image::DynamicImage::ImageRgb8(decoded);
        let mut img_state = picker.new_resize_protocol(img);

        terminal.draw(|f| render(f, &mut img_state, status))?;

        // Poll for input
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            return Ok(());
                        }
                        KeyCode::Char(' ') => {
                            // Show "Capturing..." before blocking
                            status = "📸 Capturing...";
                            terminal.draw(|f| render(f, &mut img_state, status))?;

                            match take_photo(camera) {
                                Ok(_) => status = "✅ Saved!",
                                Err(_) => status = "❌ Capture failed!",
                            }
                            status_set_at = Some(Instant::now());
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
