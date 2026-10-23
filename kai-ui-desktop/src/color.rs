//! Straight-alpha sRGB colors for themes, widgets and paint.
//!
//! Apps parse and author colors here ([`Color::parse_hex`], [`Color::hex`])
//! without reaching into the hidden `render` module.

/// Straight-alpha color in the sRGB color space, as authored in design tokens.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn hex(value: u32) -> Self {
        Self::rgb((value >> 16) as u8, (value >> 8) as u8, value as u8)
    }

    pub const fn with_alpha(self, a: u8) -> Self {
        Self { a, ..self }
    }

    /// An opaque color written as CSS hex: `#rgb` or `#rrggbb`, with or
    /// without `#`, in either case, surrounding spaces ignored.
    pub fn parse_hex(text: &str) -> Option<Self> {
        let digits = text.trim();
        let digits = digits.strip_prefix('#').unwrap_or(digits);
        if !digits.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let value = u32::from_str_radix(digits, 16).ok()?;
        match digits.len() {
            6 => Some(Self::hex(value)),
            3 => {
                let double = |nibble: u32| (nibble * 0x11) as u8;
                Some(Self::rgb(
                    double(value >> 8),
                    double((value >> 4) & 0xf),
                    double(value & 0xf),
                ))
            }
            _ => None,
        }
    }

    /// `#rrggbb` in lowercase; alpha is left out.
    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// CSS `color-mix(in srgb, self weight, other)` for opaque colors.
    pub fn mix(self, other: Self, weight: f32) -> Self {
        let channel =
            |a: u8, b: u8| (f32::from(a) * weight + f32::from(b) * (1.0 - weight)).round() as u8;
        Self {
            r: channel(self.r, other.r),
            g: channel(self.g, other.g),
            b: channel(self.b, other.b),
            a: channel(self.a, other.a),
        }
    }

    /// Components in linear light, for blending on an sRGB render target.
    /// Only for `KAI_LINEAR_BLEND=1`: Kai blends in gamma since Paso 14.
    pub fn to_linear(self) -> [f32; 4] {
        [
            srgb_to_linear(self.r),
            srgb_to_linear(self.g),
            srgb_to_linear(self.b),
            f32::from(self.a) / 255.0,
        ]
    }

    /// Components still sRGB-encoded, for non-sRGB render targets (gamma
    /// blending, the default since Paso 14).
    pub fn to_srgb(self) -> [f32; 4] {
        [
            f32::from(self.r) / 255.0,
            f32::from(self.g) / 255.0,
            f32::from(self.b) / 255.0,
            f32::from(self.a) / 255.0,
        ]
    }
}

pub fn srgb_to_linear(component: u8) -> f32 {
    let c = f32::from(component) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
mod tests {
    use super::{Color, srgb_to_linear};

    #[test]
    fn srgb_conversion_matches_reference_points() {
        assert_eq!(srgb_to_linear(0), 0.0);
        assert_eq!(srgb_to_linear(255), 1.0);
        assert!((srgb_to_linear(128) - 0.2158).abs() < 0.001);
        assert!((srgb_to_linear(10) - 0.003035).abs() < 0.0001);
        assert_eq!(Color::hex(0x1e73ae), Color::rgb(0x1e, 0x73, 0xae));
        assert_eq!(Color::rgba(0, 0, 0, 51).to_linear()[3], 0.2);
        assert_eq!(Color::rgb(255, 0, 0).to_srgb(), [1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn hex_text_goes_to_a_color_and_back() {
        let azul = Color::rgb(0x1e, 0x73, 0xae);
        for text in ["#1e73ae", "1e73ae", "#1E73AE", "  #1e73Ae "] {
            assert_eq!(Color::parse_hex(text), Some(azul), "{text:?}");
        }
        assert_eq!(azul.to_hex(), "#1e73ae");
        assert_eq!(Color::parse_hex(&azul.to_hex()), Some(azul));
        assert_eq!(Color::parse_hex("#f80"), Some(Color::rgb(0xff, 0x88, 0x00)));
        assert_eq!(Color::rgba(1, 2, 3, 4).to_hex(), "#010203");
        for text in [
            "",
            "#",
            "#1e73a",
            "#1e73aeff",
            "#12g",
            "rojo",
            "#+1e73a",
            "##1e73ae",
        ] {
            assert_eq!(Color::parse_hex(text), None, "{text:?}");
        }
    }
}
