use image::{ImageBuffer, Rgb};
use nokhwa::{
    Camera,
    pixel_format::RgbFormat,
    utils::{CameraIndex, RequestedFormat, RequestedFormatType},
};

fn main() {
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
    println!("Saved to capture.png");
}
