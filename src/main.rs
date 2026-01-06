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
    );
}
