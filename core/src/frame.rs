//! Input types shared by every module: one camera frame of hand landmarks.

/// Landmarks per Hand, as MediaPipe's hand model returns them.
pub const LANDMARKS_PER_HAND: usize = 21;

/// One landmark in normalised screen space: x and y in 0..1,
/// already mirrored like the preview the Player sees.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Landmark {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handedness {
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hand {
    pub handedness: Handedness,
    pub landmarks: [Landmark; LANDMARKS_PER_HAND],
}

/// One camera frame: capture timestamp on the camera timeline, and up to 2 Hands.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub timestamp_ns: u64,
    pub hands: Vec<Hand>,
}
