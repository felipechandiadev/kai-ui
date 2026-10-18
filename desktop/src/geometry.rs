//! Pure geometry primitives. `Point`, `Size`, and `Rect` use logical window
//! units (points); `PhysicalPoint` is used only at the platform boundary.

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub fn is_finite_and_positive(self) -> bool {
        self.width.is_finite() && self.height.is_finite() && self.width > 0.0 && self.height > 0.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Insets {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn contains(self, point: Point) -> bool {
        self.is_finite_and_non_empty()
            && point.x.is_finite()
            && point.y.is_finite()
            && point.x >= self.x
            && point.y >= self.y
            && point.x <= self.x + self.width
            && point.y <= self.y + self.height
    }

    pub fn intersection(self, other: Self) -> Option<Self> {
        if !self.is_finite_and_non_empty() || !other.is_finite_and_non_empty() {
            return None;
        }

        let left = self.x.max(other.x);
        let top = self.y.max(other.y);
        let right = (self.x + self.width).min(other.x + other.width);
        let bottom = (self.y + self.height).min(other.y + other.height);
        (right > left && bottom > top).then_some(Self {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        })
    }

    pub fn translated(self, offset: Point) -> Self {
        Self {
            x: self.x + offset.x,
            y: self.y + offset.y,
            ..self
        }
    }

    pub fn inset(self, insets: Insets) -> Self {
        Self {
            x: self.x + insets.left,
            y: self.y + insets.top,
            width: self.width - insets.left - insets.right,
            height: self.height - insets.top - insets.bottom,
        }
    }

    pub fn to_ndc(self, viewport: Size) -> Option<NdcRect> {
        if !self.is_finite_and_non_empty() || !viewport.is_finite_and_positive() {
            return None;
        }

        Some(NdcRect {
            left: self.x / viewport.width * 2.0 - 1.0,
            right: (self.x + self.width) / viewport.width * 2.0 - 1.0,
            top: 1.0 - self.y / viewport.height * 2.0,
            bottom: 1.0 - (self.y + self.height) / viewport.height * 2.0,
        })
    }

    fn is_finite_and_non_empty(self) -> bool {
        self.x.is_finite()
            && self.y.is_finite()
            && self.width.is_finite()
            && self.height.is_finite()
            && self.width > 0.0
            && self.height > 0.0
            && (self.x + self.width).is_finite()
            && (self.y + self.height).is_finite()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicalPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScaleFactor(f64);

impl ScaleFactor {
    pub fn new(value: f64) -> Option<Self> {
        (value.is_finite() && value > 0.0).then_some(Self(value))
    }

    pub fn get(self) -> f64 {
        self.0
    }

    pub fn as_f32(self) -> f32 {
        self.0 as f32
    }

    pub fn to_logical_point(self, point: PhysicalPoint) -> Point {
        Point {
            x: (point.x / self.0) as f32,
            y: (point.y / self.0) as f32,
        }
    }

    pub fn to_physical_point(self, point: Point) -> PhysicalPoint {
        PhysicalPoint {
            x: f64::from(point.x) * self.0,
            y: f64::from(point.y) * self.0,
        }
    }

    pub fn to_logical_size(self, width: u32, height: u32) -> Size {
        Size {
            width: width as f32 / self.as_f32(),
            height: height as f32 / self.as_f32(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NdcRect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

#[cfg(test)]
mod tests {
    use super::{Insets, PhysicalPoint, Point, Rect, ScaleFactor, Size};

    const SAMPLE_RECT: Rect = Rect {
        x: 10.0,
        y: 20.0,
        width: 30.0,
        height: 40.0,
    };

    #[test]
    fn hit_test_includes_edges_and_rejects_points_outside_or_non_finite() {
        assert!(SAMPLE_RECT.contains(Point { x: 10.0, y: 20.0 }));
        assert!(SAMPLE_RECT.contains(Point { x: 40.0, y: 60.0 }));
        assert!(!SAMPLE_RECT.contains(Point { x: 9.99, y: 20.0 }));
        assert!(!SAMPLE_RECT.contains(Point {
            x: f32::NAN,
            y: 20.0
        }));
    }

    #[test]
    fn empty_or_invalid_rectangles_do_not_hit_or_map_to_ndc() {
        for rect in [
            Rect {
                width: 0.0,
                ..SAMPLE_RECT
            },
            Rect {
                height: -1.0,
                ..SAMPLE_RECT
            },
            Rect {
                x: f32::INFINITY,
                ..SAMPLE_RECT
            },
        ] {
            assert!(!rect.contains(Point { x: 10.0, y: 20.0 }));
            assert!(
                rect.to_ndc(Size {
                    width: 100.0,
                    height: 100.0
                })
                .is_none()
            );
        }
    }

    #[test]
    fn intersection_translation_and_insets_follow_logical_units() {
        assert_eq!(
            SAMPLE_RECT.intersection(Rect {
                x: 20.0,
                y: 40.0,
                width: 30.0,
                height: 30.0,
            }),
            Some(Rect {
                x: 20.0,
                y: 40.0,
                width: 20.0,
                height: 20.0,
            })
        );
        assert_eq!(
            SAMPLE_RECT.translated(Point { x: 2.0, y: -3.0 }),
            Rect {
                x: 12.0,
                y: 17.0,
                ..SAMPLE_RECT
            }
        );
        assert_eq!(
            SAMPLE_RECT.inset(Insets {
                top: 1.0,
                right: 2.0,
                bottom: 3.0,
                left: 4.0,
            }),
            Rect {
                x: 14.0,
                y: 21.0,
                width: 24.0,
                height: 36.0,
            }
        );
    }

    #[test]
    fn scale_factor_converts_between_physical_and_logical_coordinates() {
        let scale = ScaleFactor::new(2.0).unwrap();
        let logical = scale.to_logical_point(PhysicalPoint { x: 80.0, y: 60.0 });
        assert_eq!(logical, Point { x: 40.0, y: 30.0 });
        assert_eq!(
            scale.to_physical_point(logical),
            PhysicalPoint { x: 80.0, y: 60.0 }
        );
        assert_eq!(
            scale.to_logical_size(200, 100),
            Size {
                width: 100.0,
                height: 50.0
            }
        );
        assert!(ScaleFactor::new(0.0).is_none());
        assert!(ScaleFactor::new(f64::INFINITY).is_none());
    }

    #[test]
    fn logical_rect_maps_to_normalized_device_coordinates() {
        assert_eq!(
            SAMPLE_RECT.to_ndc(Size {
                width: 100.0,
                height: 100.0,
            }),
            Some(super::NdcRect {
                left: -0.8,
                top: 0.6,
                right: -0.19999999,
                bottom: -0.20000005,
            })
        );
    }
}
