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
    layout::Alignment,
    prelude::CrosstermBackend,
    widgets::{Block, Borders, Paragraph},
};

#[derive(Parser)]
#[command(name = "tcam", version = "1.0.0", about = "terminal camera")]
struct Cli {}

fn main() {
    let _ = Cli::parse();
    crossterm::terminal::enable_raw_mode().expect("Can't enable raw mode");
    std::io::stdout()
        .execute(crossterm::terminal::EnterAlternateScreen)
        .expect("Can't enter alternate screen");

    let mut terminal = ratatui::Terminal::new(CrosstermBackend::new(std::io::stdout()))
        .expect("Can't create terminal");
    let result = run(&mut terminal);

    crossterm::terminal::disable_raw_mode().expect("Can't disable raw mode");
    std::io::stdout()
        .execute(crossterm::terminal::LeaveAlternateScreen)
        .expect("Can't leave alternate screen");

    if let Err(e) = result {
        eprintln!("Error: {}", e);
    }
    std::process::exit(0);
}

fn take_photo() -> Result<(), io::Error> {
    let mut camera = Camera::new(
        // Which camera to use
        CameraIndex::default(),
        // A format for the camera output (e.g. 1024x1024, MP4)
        RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate),
    )
    .expect("No camera?");
    camera.open_stream().expect("Failed to use camera");

    // Warm up - discard first ~30 frames for auto-exposure
    for _ in 0..30 {
        let _ = camera.frame();
    }

    let frame = camera.frame().expect("Failed to take a frame");
    let decoded = frame.decode_image::<RgbFormat>().expect("Failed to decode");

    // Convert to our image crate version
    let (width, height) = (decoded.width(), decoded.height());
    let raw: Vec<u8> = decoded.into_raw();
    let img: ImageBuffer<Rgb<u8>, _> =
        ImageBuffer::from_raw(width, height, raw).expect("Failed to create image buffer");

    img.save("capture.png").expect("Failed to save");

    Ok(())
}

fn run(terminal: &mut ratatui::Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let mut status = "[SPACE] Take Photo [Q] Quit";

    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            let block = Block::default()
                .title(" 📷 tcam ")
                .borders(Borders::ALL)
                .border_style(ratatui::style::Style::default().fg(ratatui::style::Color::Cyan));

            let text = Paragraph::new(status)
                .block(block)
                .alignment(ratatui::layout::Alignment::Center);

            frame.render_widget(text, area);
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

                            match take_photo() {
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
