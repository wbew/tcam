use std::io;

use clap::Parser;
use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
};
use image::{ImageBuffer, Rgb};
use nokhwa::{
    Camera,
    pixel_format::RgbFormat,
    utils::{CameraIndex, RequestedFormat, RequestedFormatType},
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    prelude::CrosstermBackend,
    widgets::{Block, Borders, Paragraph},
};
use ratatui_image::{StatefulImage, picker::Picker};

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

fn run(
    terminal: &mut ratatui::Terminal<CrosstermBackend<io::Stdout>>,
    camera: &mut Camera,
    picker: &Picker,
) -> io::Result<()> {
    let mut status = "[SPACE] Take Photo [Q] Quit";

    loop {
        let frame = camera.frame().expect("Failed to get frame");
        let decoded = frame.decode_image::<RgbFormat>().expect("Failed to decode");
        let img = image::DynamicImage::ImageRgb8(decoded);
        let mut terminal_img_state = picker.new_resize_protocol(img);

        terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
                .split(frame.area());

            frame.render_stateful_widget(StatefulImage::new(), chunks[0], &mut terminal_img_state);

            // let area = frame.area();
            let block = Block::default()
                .title(" 📷 tcam ")
                .borders(Borders::ALL)
                .border_style(ratatui::style::Style::default().fg(ratatui::style::Color::Cyan));

            let text = Paragraph::new(status)
                .block(block)
                .alignment(ratatui::layout::Alignment::Center);

            frame.render_widget(text, chunks[1]);
        })?;

        // Poll for input (Crossterm)
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            return Ok(());
                        }
                        KeyCode::Char(' ') => {
                            status = "📸 Capturing...";
                            terminal.draw(|f| {
                                let p = Paragraph::new(status)
                                    .block(Block::default().borders(Borders::ALL))
                                    .alignment(Alignment::Center);
                                f.render_widget(p, f.area());
                            })?;

                            match take_photo(camera) {
                                Ok(_) => status = "✅ Saved to capture.png!",
                                Err(_) => status = "❌ Capture failed!",
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
