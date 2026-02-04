use sha2::{Digest, Sha256};

/// Generate a color hex string from an input string
pub fn color_for_string(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();

    // Take first 3 bytes for RGB
    format!("#{:02x}{:02x}{:02x}", result[0], result[1], result[2])
}

/// Calculate relative luminance of a color
fn relative_luminance(hex_color: &str) -> f64 {
    let hex = hex_color.trim_start_matches('#');

    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f64 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f64 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f64 / 255.0;

    let r = if r <= 0.03928 {
        r / 12.92
    } else {
        ((r + 0.055) / 1.055).powf(2.4)
    };
    let g = if g <= 0.03928 {
        g / 12.92
    } else {
        ((g + 0.055) / 1.055).powf(2.4)
    };
    let b = if b <= 0.03928 {
        b / 12.92
    } else {
        ((b + 0.055) / 1.055).powf(2.4)
    };

    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// Calculate contrast ratio between two colors
pub fn contrast_ratio(color1: &str, color2: &str) -> f64 {
    let l1 = relative_luminance(color1);
    let l2 = relative_luminance(color2);

    let lighter = l1.max(l2);
    let darker = l1.min(l2);

    (lighter + 0.05) / (darker + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_for_string() {
        let color = color_for_string("test");
        assert!(color.starts_with('#'));
        assert_eq!(color.len(), 7);

        // Same input should produce same color
        assert_eq!(color, color_for_string("test"));
    }

    #[test]
    fn test_contrast_ratio() {
        // Black and white should have maximum contrast
        let ratio = contrast_ratio("#000000", "#ffffff");
        assert!(ratio > 20.0); // Should be 21:1

        // Same colors should have 1:1 contrast
        let ratio = contrast_ratio("#000000", "#000000");
        assert!((ratio - 1.0).abs() < 0.01);
    }
}
