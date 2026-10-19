//! Window background effects: a full-window layer the renderer draws under
//! the content, in window coordinates (it does not scroll).
//!
//! An app picks one with [`crate::runtime::App::background`]; the default
//! is [`BackgroundEffect::NONE`], the solid window color. The effects
//! follow the GlowStudio reference (CSS blurred orbs, a rotating conic
//! gradient): soft color over the window color of the theme, so they work
//! with Kai and with the AppKit themes, in light and dark. The closest
//! AppKit relative is a window-sized `NSVisualEffectView` behind the
//! content (11 §9 has no equivalent material; this is Kai's own).
//!
//! Static effects render once and are cached by the renderer; animated
//! ones redraw only while the window is visible and active
//! (`docs/dev/04-renderer-y-texto.md`).

use std::time::{Duration, Instant};

use crate::render::Color;
use crate::theme::Appearance;

/// Which effect.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BackgroundKind {
    /// Solid window color (no effect layer).
    #[default]
    None,
    /// Four blurred color orbs blended over the window color.
    Mesh,
    /// A blurred conic gradient that turns slowly ("aurora boreal").
    Aurora,
    /// A large central glow and two floating orbs ("ambient glow").
    Glow,
}

impl BackgroundKind {
    pub const ALL: [Self; 4] = [Self::None, Self::Mesh, Self::Aurora, Self::Glow];

    /// `none`, `mesh`, `aurora` or `glow` (as in `KAI_BG`).
    pub const fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Mesh => "mesh",
            Self::Aurora => "aurora",
            Self::Glow => "glow",
        }
    }

    /// Inverse of [`Self::name`], case-insensitive; an empty string is
    /// `None`.
    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim().to_ascii_lowercase();
        if value.is_empty() {
            return Some(Self::None);
        }
        Self::ALL.into_iter().find(|kind| kind.name() == value)
    }
}

/// Still or moving.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BackgroundMotion {
    /// One frame (the reference's first keyframe); no redraws of its own.
    #[default]
    Static,
    /// The reference's CSS animations, one cycle every
    /// [`BackgroundSpeed::cycle_seconds`].
    Animated,
}

/// Animation cycle of the reference's speed slider.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BackgroundSpeed {
    Slow,
    #[default]
    Normal,
    Fast,
}

impl BackgroundSpeed {
    /// 30, 16 and 8 s (`SPEED_MAP` of the reference).
    pub const fn cycle_seconds(self) -> f32 {
        match self {
            Self::Slow => 30.0,
            Self::Normal => 16.0,
            Self::Fast => 8.0,
        }
    }
}

/// Color sets of the reference ("tonos suaves"), with a deeper variant of
/// each for dark appearances.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BackgroundPalette {
    /// "Atardecer suave": amber, pink, lavender, sky.
    #[default]
    Sunset,
    /// "Azul menta": mint, sky, indigo, slate.
    Mint,
    /// "Lavanda y melocotón": purple, peach, pink, light blue.
    Lavender,
    /// "Rosa pasteles": rose, yellow, indigo, purple.
    Pastel,
}

impl BackgroundPalette {
    /// The four colors (`c1`–`c4`) for `appearance`. Light uses the
    /// reference's Tailwind 100–200 tones as they are; dark uses deeper
    /// tones of the same hues (the reference is light only).
    pub const fn colors(self, appearance: Appearance) -> [Color; 4] {
        let hex = Color::hex;
        match (self, appearance) {
            (Self::Sunset, Appearance::Light) => {
                [hex(0xfde68a), hex(0xfbcfe8), hex(0xddd6fe), hex(0xbae6fd)]
            }
            (Self::Sunset, Appearance::Dark) => {
                [hex(0xb45309), hex(0xbe185d), hex(0x6d28d9), hex(0x0369a1)]
            }
            (Self::Mint, Appearance::Light) => {
                [hex(0xa7f3d0), hex(0xbae6fd), hex(0xc7d2fe), hex(0xf1f5f9)]
            }
            (Self::Mint, Appearance::Dark) => {
                [hex(0x047857), hex(0x0369a1), hex(0x4338ca), hex(0x334155)]
            }
            (Self::Lavender, Appearance::Light) => {
                [hex(0xe9d5ff), hex(0xfed7aa), hex(0xfbcfe8), hex(0xe0f2fe)]
            }
            (Self::Lavender, Appearance::Dark) => {
                [hex(0x7e22ce), hex(0xc2410c), hex(0xbe185d), hex(0x0369a1)]
            }
            (Self::Pastel, Appearance::Light) => {
                [hex(0xfecdd3), hex(0xfef08a), hex(0xe0e7ff), hex(0xf3e8ff)]
            }
            (Self::Pastel, Appearance::Dark) => {
                [hex(0xbe123c), hex(0xa16207), hex(0x4338ca), hex(0x7e22ce)]
            }
        }
    }
}

/// Background effect of a window. `Copy`; build it with
/// [`BackgroundEffect::new`] and the `with_*` methods.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BackgroundEffect {
    pub kind: BackgroundKind,
    pub motion: BackgroundMotion,
    pub palette: BackgroundPalette,
    pub speed: BackgroundSpeed,
    /// Blur of the color shapes in points (CSS `blur()`, a standard
    /// deviation); 80 in the reference (slider 20–150).
    pub blur: f32,
    /// Multiplies the opacity of every color layer: 1 is the reference, 0
    /// leaves the window color.
    pub intensity: f32,
}

impl BackgroundEffect {
    /// The solid window color.
    pub const NONE: Self = Self::new(BackgroundKind::None);

    /// `kind`, static, with the reference's defaults (sunset palette,
    /// normal speed, blur 80, full intensity).
    pub const fn new(kind: BackgroundKind) -> Self {
        Self {
            kind,
            motion: BackgroundMotion::Static,
            palette: BackgroundPalette::Sunset,
            speed: BackgroundSpeed::Normal,
            blur: 80.0,
            intensity: 1.0,
        }
    }

    pub const fn with_motion(self, motion: BackgroundMotion) -> Self {
        Self { motion, ..self }
    }

    pub const fn with_palette(self, palette: BackgroundPalette) -> Self {
        Self { palette, ..self }
    }

    pub const fn with_speed(self, speed: BackgroundSpeed) -> Self {
        Self { speed, ..self }
    }

    pub const fn with_blur(self, blur: f32) -> Self {
        Self { blur, ..self }
    }

    pub const fn with_intensity(self, intensity: f32) -> Self {
        Self { intensity, ..self }
    }

    /// Whether there is a layer to draw.
    pub fn is_visible(&self) -> bool {
        self.kind != BackgroundKind::None && self.intensity > 0.0
    }

    /// Whether it needs frames of its own (visible and animated).
    pub fn is_animated(&self) -> bool {
        self.is_visible() && self.motion == BackgroundMotion::Animated
    }
}

impl Default for BackgroundEffect {
    fn default() -> Self {
        Self::NONE
    }
}

/// Opacity of the color layers in dark appearances, relative to light:
/// deep tones over a dark window read much stronger than pastels over
/// white, and text on translucent group boxes must stay readable.
pub const DARK_INTENSITY: f32 = 0.65;

/// Extra share of white over the aurora in light appearances, on top of the
/// reference's 40 % white veil (Felipe: same hues, at least 20 % whiter).
/// Dark appearances keep the veil only: whitening a dark window would grey
/// it out.
pub const AURORA_WHITEN: f32 = 0.20;

/// An effect resolved for one window and frame: what the renderer draws.
/// Paint builds it from the effect, the root's background color and the
/// appearance; the runtime sets the time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BackgroundLayer {
    /// Never [`BackgroundKind::None`].
    pub kind: BackgroundKind,
    /// Window color under the effect (the root's background).
    pub base: Color,
    pub colors: [Color; 4],
    pub dark: bool,
    pub blur: f32,
    pub intensity: f32,
    pub animated: bool,
    /// Seconds per animation cycle.
    pub cycle: f32,
    /// Animation time in seconds; always 0 when static.
    pub time: f32,
}

impl BackgroundLayer {
    /// `None` when `effect` draws nothing. Dark layers get
    /// [`DARK_INTENSITY`].
    pub fn new(effect: &BackgroundEffect, base: Color, appearance: Appearance) -> Option<Self> {
        let dark = appearance == Appearance::Dark;
        let tone = if dark { DARK_INTENSITY } else { 1.0 };
        effect.is_visible().then(|| Self {
            kind: effect.kind,
            base: base.with_alpha(255),
            colors: effect.palette.colors(appearance),
            dark,
            blur: effect.blur.max(0.0),
            intensity: effect.intensity.clamp(0.0, 2.0) * tone,
            animated: effect.motion == BackgroundMotion::Animated,
            cycle: effect.speed.cycle_seconds(),
            time: 0.0,
        })
    }

    /// Same layer at `seconds` of animation (static layers stay at 0, so
    /// their frames are identical and the renderer can reuse them).
    pub fn at(self, seconds: f32) -> Self {
        Self {
            time: if self.animated { seconds.max(0.0) } else { 0.0 },
            ..self
        }
    }
}

/// Time between animation frames (60 Hz). The background moves a few
/// points per second, so a faster display gains nothing.
pub const FRAME_INTERVAL: Duration = Duration::from_micros(16_667);

/// Longest step the clock takes between two frames, so a stalled frame
/// does not make the effect jump.
const MAX_STEP: f32 = 0.1;

/// Animation time that only advances while the effect is running (window
/// visible and active, motion not reduced), so it resumes where it paused.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BackgroundClock {
    elapsed: f32,
    last: Option<Instant>,
}

impl BackgroundClock {
    /// Advance to `now` if `running` and return the animation time.
    pub fn tick(&mut self, now: Instant, running: bool) -> f32 {
        if running {
            if let Some(last) = self.last {
                self.elapsed += now
                    .saturating_duration_since(last)
                    .as_secs_f32()
                    .min(MAX_STEP);
            }
            self.last = Some(now);
        } else {
            self.last = None;
        }
        self.elapsed
    }

    pub fn elapsed(&self) -> f32 {
        self.elapsed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_parse_by_name_and_none_is_the_default() {
        for kind in BackgroundKind::ALL {
            assert_eq!(BackgroundKind::parse(kind.name()), Some(kind));
        }
        assert_eq!(
            BackgroundKind::parse("Aurora"),
            Some(BackgroundKind::Aurora)
        );
        assert_eq!(BackgroundKind::parse(" glow "), Some(BackgroundKind::Glow));
        assert_eq!(BackgroundKind::parse(""), Some(BackgroundKind::None));
        assert_eq!(BackgroundKind::parse("bokeh"), None);
        assert_eq!(BackgroundEffect::default(), BackgroundEffect::NONE);
        assert_eq!(BackgroundKind::default(), BackgroundKind::None);
        assert_eq!(BackgroundMotion::default(), BackgroundMotion::Static);
    }

    #[test]
    fn only_visible_animated_effects_need_frames() {
        let mesh = BackgroundEffect::new(BackgroundKind::Mesh);
        assert!(mesh.is_visible() && !mesh.is_animated());
        let moving = mesh.with_motion(BackgroundMotion::Animated);
        assert!(moving.is_animated());
        assert!(!moving.with_intensity(0.0).is_animated());
        assert!(
            !BackgroundEffect::NONE
                .with_motion(BackgroundMotion::Animated)
                .is_animated()
        );
        assert_eq!(
            [
                BackgroundSpeed::Slow,
                BackgroundSpeed::Normal,
                BackgroundSpeed::Fast
            ]
            .map(BackgroundSpeed::cycle_seconds),
            [30.0, 16.0, 8.0]
        );
    }

    #[test]
    fn layers_follow_the_theme_and_static_ones_ignore_time() {
        let base = Color::hex(0xffffff);
        assert_eq!(
            BackgroundLayer::new(&BackgroundEffect::NONE, base, Appearance::Light),
            None
        );
        let effect = BackgroundEffect::new(BackgroundKind::Glow);
        let light = BackgroundLayer::new(&effect, base, Appearance::Light).unwrap();
        assert_eq!(light.colors[0], Color::hex(0xfde68a), "reference sunset c1");
        assert!(!light.dark && !light.animated && light.cycle == 16.0);
        assert_eq!(light.at(3.0).time, 0.0, "static");
        let dark =
            BackgroundLayer::new(&effect, Color::rgba(30, 30, 30, 200), Appearance::Dark).unwrap();
        assert!(dark.dark);
        assert_eq!(dark.intensity, DARK_INTENSITY);
        assert_eq!(light.intensity, 1.0);
        assert_eq!(dark.base, Color::rgb(30, 30, 30), "opaque base");
        assert_ne!(dark.colors, light.colors);
        let moving = BackgroundLayer::new(
            &effect.with_motion(BackgroundMotion::Animated),
            base,
            Appearance::Light,
        )
        .unwrap();
        assert_eq!(moving.at(3.0).time, 3.0);
    }

    #[test]
    fn the_clock_runs_only_while_running_and_never_jumps() {
        let start = Instant::now();
        let at = |ms: u64| start + Duration::from_millis(ms);
        let mut clock = BackgroundClock::default();
        assert_eq!(clock.tick(at(0), true), 0.0);
        assert!((clock.tick(at(50), true) - 0.05).abs() < 1e-6);
        // Paused: time stops, and resuming does not count the pause.
        assert!((clock.tick(at(1000), false) - 0.05).abs() < 1e-6);
        assert!((clock.tick(at(5000), true) - 0.05).abs() < 1e-6);
        assert!((clock.tick(at(5016), true) - 0.066).abs() < 1e-5);
        // A stalled frame advances at most MAX_STEP.
        assert!((clock.tick(at(9000), true) - (0.066 + MAX_STEP)).abs() < 1e-5);
        assert!((clock.elapsed() - (0.066 + MAX_STEP)).abs() < 1e-5);
    }
}
