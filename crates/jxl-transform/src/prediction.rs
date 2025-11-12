//! Prediction modes for lossy compression

/// Prediction mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PredictionMode {
    /// No prediction
    None,
    /// Left pixel
    Left,
    /// Top pixel
    Top,
    /// Average of left and top
    Average,
    /// Paeth predictor (PNG-style)
    Paeth,
    /// Gradient predictor
    Gradient,
}

/// Apply prediction to a channel
pub fn apply_prediction(
    input: &[f32],
    output: &mut [f32],
    width: usize,
    height: usize,
    mode: PredictionMode,
) {
    assert_eq!(input.len(), width * height);
    assert_eq!(output.len(), width * height);

    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            let pixel = input[idx];

            let prediction = match mode {
                PredictionMode::None => 0.0,
                PredictionMode::Left => {
                    if x > 0 {
                        input[idx - 1]
                    } else {
                        0.0
                    }
                }
                PredictionMode::Top => {
                    if y > 0 {
                        input[idx - width]
                    } else {
                        0.0
                    }
                }
                PredictionMode::Average => {
                    let left = if x > 0 { input[idx - 1] } else { 0.0 };
                    let top = if y > 0 { input[idx - width] } else { 0.0 };
                    (left + top) / 2.0
                }
                PredictionMode::Paeth => paeth_predictor(input, x, y, width),
                PredictionMode::Gradient => gradient_predictor(input, x, y, width),
            };

            output[idx] = pixel - prediction;
        }
    }
}

/// Reverse prediction
pub fn reverse_prediction(
    input: &[f32],
    output: &mut [f32],
    width: usize,
    height: usize,
    mode: PredictionMode,
) {
    assert_eq!(input.len(), width * height);
    assert_eq!(output.len(), width * height);

    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            let residual = input[idx];

            let prediction = match mode {
                PredictionMode::None => 0.0,
                PredictionMode::Left => {
                    if x > 0 {
                        output[idx - 1]
                    } else {
                        0.0
                    }
                }
                PredictionMode::Top => {
                    if y > 0 {
                        output[idx - width]
                    } else {
                        0.0
                    }
                }
                PredictionMode::Average => {
                    let left = if x > 0 { output[idx - 1] } else { 0.0 };
                    let top = if y > 0 { output[idx - width] } else { 0.0 };
                    (left + top) / 2.0
                }
                PredictionMode::Paeth => paeth_predictor(output, x, y, width),
                PredictionMode::Gradient => gradient_predictor(output, x, y, width),
            };

            output[idx] = residual + prediction;
        }
    }
}

fn paeth_predictor(data: &[f32], x: usize, y: usize, width: usize) -> f32 {
    if x == 0 && y == 0 {
        return 0.0;
    }

    let left = if x > 0 { data[y * width + x - 1] } else { 0.0 };
    let top = if y > 0 {
        data[(y - 1) * width + x]
    } else {
        0.0
    };
    let top_left = if x > 0 && y > 0 {
        data[(y - 1) * width + x - 1]
    } else {
        0.0
    };

    let p = left + top - top_left;
    let pa = (p - left).abs();
    let pb = (p - top).abs();
    let pc = (p - top_left).abs();

    if pa <= pb && pa <= pc {
        left
    } else if pb <= pc {
        top
    } else {
        top_left
    }
}

fn gradient_predictor(data: &[f32], x: usize, y: usize, width: usize) -> f32 {
    if x == 0 && y == 0 {
        return 0.0;
    }

    let left = if x > 0 { data[y * width + x - 1] } else { 0.0 };
    let top = if y > 0 {
        data[(y - 1) * width + x]
    } else {
        0.0
    };
    let top_left = if x > 0 && y > 0 {
        data[(y - 1) * width + x - 1]
    } else {
        0.0
    };

    left + top - top_left
}
