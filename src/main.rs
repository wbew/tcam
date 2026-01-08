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

    let result = run(&mut terminal, &mut camera);

    crossterm::terminal::disable_raw_mode().expect("Can't disable raw mode");
    std::io::stdout()
        .execute(crossterm::terminal::LeaveAlternateScreen)
        .expect("Can't leave alternate screen");

    if let Err(e) = result {
        eprintln!("Error: {}", e);
    }
    std::process::exit(0);
}

#[derive(Clone, Copy, Default)]
enum PhotoStyle {
    #[default]
    Real,
    Pixel,
    Ascii,
}

impl PhotoStyle {
    fn next(self) -> Self {
        match self {
            PhotoStyle::Real => PhotoStyle::Pixel,
            PhotoStyle::Pixel => PhotoStyle::Ascii,
            PhotoStyle::Ascii => PhotoStyle::Real,
        }
    }

    fn name(self) -> &'static str {
        match self {
            PhotoStyle::Real => "real",
            PhotoStyle::Pixel => "pixel",
            PhotoStyle::Ascii => "ascii",
        }
    }
}

impl From<PhotoStyle> for Picker {
    fn from(style: PhotoStyle) -> Self {
        match style {
            PhotoStyle::Real => Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks()),
            PhotoStyle::Pixel => Picker::halfblocks(),
            PhotoStyle::Ascii => Picker::halfblocks(), // fallback to halfblocks for now
        }
    }
}

fn take_photo(camera: &mut Camera, style: PhotoStyle) -> Result<image::DynamicImage, io::Error> {
    // Warm up - discard first ~30 frames for auto-exposure
    for _ in 0..30 {
        let _ = camera.frame();
    }

    let frame = camera.frame().expect("Failed to take a frame");
    let decoded = frame.decode_image::<RgbFormat>().expect("Failed to decode");

    let final_img = match style {
        PhotoStyle::Pixel => {
            // Downsample to 80x60, then upscale to 1280x960 with nearest-neighbor
            let small = image::DynamicImage::ImageRgb8(decoded).resize_exact(
                80,
                60,
                image::imageops::FilterType::Nearest,
            );
            small.resize_exact(1280, 960, image::imageops::FilterType::Nearest)
        }
        _ => image::DynamicImage::ImageRgb8(decoded),
    };

    final_img
        .save(&format!(
            "capture-{}-{}.png",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            style.name()
        ))
        .expect("Failed to save");

    Ok(final_img)
}

fn render(
    frame: &mut Frame,
    image_state: &mut StatefulProtocol,
    status: &str,
    captured_photos: &[image::DynamicImage],
    picker: &Picker,
    current_style: PhotoStyle,
) {
    // Main vertical split: camera (fill) | bottom bar (fixed)
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(20), Constraint::Length(15)])
        .split(frame.area());

    // Render camera preview
    frame.render_stateful_widget(StatefulImage::new(), main_chunks[0], image_state);

    // Bottom bar split: status row (3 lines) | photo row (remaining)
    let bottom_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Fill(1)])
        .split(main_chunks[1]);

    // Status bar
    let block = Block::default()
        .title(" 📷 tcam ")
        .borders(Borders::ALL)
        .border_style(ratatui::style::Style::default().fg(ratatui::style::Color::Cyan));

    let text = Paragraph::new(Text::from(vec![
        Line::from(status),
        Line::from(format!(
            "[SPACE] Take Photo  [S] Style: {}  [Q] Quit",
            current_style.name()
        )),
    ]))
    .block(block)
    .alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(text, bottom_chunks[0]);

    // Photo slots: 4 equal horizontal sections
    let photo_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
        ])
        .split(bottom_chunks[1]);

    // Render photos horizontally
    for (i, chunk) in photo_chunks.iter().enumerate() {
        if i < captured_photos.len() {
            let photo = &captured_photos[i];
            let mut photo_state = picker.new_resize_protocol(photo.clone());
            frame.render_stateful_widget(StatefulImage::new(), *chunk, &mut photo_state);
        } else {
            // Empty placeholder
            let placeholder = Block::default()
                .borders(Borders::ALL)
                .border_style(ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray));
            frame.render_widget(placeholder, *chunk);
        }
    }
}

fn run(
    terminal: &mut ratatui::Terminal<CrosstermBackend<io::Stdout>>,
    camera: &mut Camera,
) -> io::Result<()> {
    let mut status = "";
    let mut status_set_at: Option<Instant> = None;
    let mut captured_photos: Vec<image::DynamicImage> = Vec::with_capacity(4);
    let mut current_style = PhotoStyle::default();
    let mut picker: Picker = current_style.into();

    loop {
        // Clear status after timeout
        if let Some(t) = status_set_at {
            if t.elapsed() > Duration::from_secs(2) {
                status = "";
                status_set_at = None;
            }
        }

        let frame = camera.frame().expect("Failed to get frame");
        let decoded = frame.decode_image::<RgbFormat>().expect("Failed to decode");
        let img = image::DynamicImage::ImageRgb8(decoded);
        let mut img_state = picker.new_resize_protocol(img);

        terminal.draw(|f| {
            render(
                f,
                &mut img_state,
                status,
                &captured_photos,
                &picker,
                current_style,
            )
        })?;

        // Poll for input
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            return Ok(());
                        }
                        KeyCode::Char('s') => {
                            current_style = current_style.next();
                            picker = current_style.into();
                        }
                        KeyCode::Char(' ') => {
                            // Show "Capturing..." before blocking
                            status = "📸 Capturing...";
                            terminal.draw(|f| {
                                render(
                                    f,
                                    &mut img_state,
                                    status,
                                    &captured_photos,
                                    &picker,
                                    current_style,
                                )
                            })?;

                            match take_photo(camera, current_style) {
                                Ok(img) => {
                                    // Keep only the last 4 photos
                                    if captured_photos.len() >= 4 {
                                        captured_photos.remove(0);
                                    }
                                    captured_photos.push(img);
                                    status = "✅ Saved!";
                                }
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
