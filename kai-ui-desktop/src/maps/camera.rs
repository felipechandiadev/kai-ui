//! The camera of one map: its live view and the gesture or animation
//! moving it. It is UI state, like a scroll offset: gestures move it
//! without calling `update`, and once one settles [`CameraState::take_report`]
//! hands out the view for a single `on_view_change`. Time comes from the
//! caller, so tests drive it with a fake clock.

use std::time::{Duration, Instant};

use super::FitRequest;
use crate::geo::{
    Camera, LatLonBounds, MapView, WorldPoint, anchored_view, fit_bounds, project, unproject,
};
use crate::geometry::{Point, Size};

/// Zoom animations (buttons, wheel notches, double click, keys).
pub const ZOOM_DURATION: Duration = Duration::from_millis(250);
/// Model-driven moves (`set_view`, `fit_bounds`).
pub const FLY_DURATION: Duration = Duration::from_millis(400);
/// Quiet time after a wheel, pinch or key move before it is reported.
pub const SETTLE_DELAY: Duration = Duration::from_millis(200);
/// Inertia slows with this time constant (s).
const INERTIA_TAU: f64 = 0.325;
/// Below this speed (points/s) inertia stops; a release slower than
/// [`INERTIA_MIN_SPEED`] has none.
const INERTIA_STOP_SPEED: f64 = 20.0;
const INERTIA_MIN_SPEED: f64 = 150.0;
/// Pointer samples older than this do not count toward the release speed.
const VELOCITY_WINDOW: Duration = Duration::from_millis(100);

#[derive(Clone, Debug, PartialEq)]
struct Drag {
    last: Point,
    /// Recent pointer positions, for the release speed.
    samples: Vec<(Instant, Point)>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Motion {
    /// Gliding after a drag: `velocity` points/s at `start`, decaying.
    Inertia {
        start: Instant,
        velocity: (f64, f64),
        /// Distance already applied.
        applied: (f64, f64),
    },
    /// Zooming to `to` with `world` fixed under `anchor`.
    Zoom {
        start: Instant,
        anchor: Point,
        world: WorldPoint,
        from: f64,
        to: f64,
    },
    /// Moving to a view set by the app.
    Fly {
        start: Instant,
        from: MapView,
        to: MapView,
    },
}

/// Room left around fitted bounds, in points.
pub const FIT_PADDING: f32 = 16.0;
/// `fit_bounds` never zooms closer than this.
pub const FIT_MAX_ZOOM: f64 = 18.0;

/// The view that shows `bounds` on a map of `size`.
pub fn fit_view(bounds: LatLonBounds, size: Size) -> MapView {
    fit_bounds(bounds, size, FIT_PADDING, FIT_MAX_ZOOM).clamped()
}

fn ease_out(t: f64) -> f64 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

fn progress(start: Instant, now: Instant, duration: Duration) -> f64 {
    now.saturating_duration_since(start).as_secs_f64() / duration.as_secs_f64()
}

#[derive(Clone, Debug, PartialEq)]
pub struct CameraState {
    /// What the map shows now.
    pub view: MapView,
    /// The app's view as of the last rebuild.
    model: MapView,
    /// Area of the map in the last layout.
    pub size: Size,
    drag: Option<Drag>,
    motion: Option<Motion>,
    /// A gesture moved the view since the last report.
    moved: bool,
    /// Report no earlier than this (wheel, pinch and keys come in bursts).
    settle: Option<Instant>,
    /// The last `fit_bounds` request served.
    fitted: Option<u64>,
}

impl CameraState {
    pub fn new(view: MapView) -> Self {
        let view = view.clamped();
        Self {
            view,
            model: view,
            size: Size::default(),
            drag: None,
            motion: None,
            moved: false,
            settle: None,
            fitted: None,
        }
    }

    fn camera(&self) -> Camera {
        Camera::new(self.view, self.size)
    }

    /// The app's view in this rebuild. A new one moves the camera (gliding
    /// when `animate`) unless a drag is under way; the echo of a report
    /// changes nothing.
    pub fn sync_model(&mut self, model: MapView, animate: bool, now: Instant) {
        let model = model.clamped();
        if model == self.model {
            return;
        }
        self.model = model;
        if self.drag.is_some() || model == self.view {
            return;
        }
        self.moved = false;
        self.settle = None;
        if animate && self.size.is_finite_and_positive() {
            self.motion = Some(Motion::Fly {
                start: now,
                from: self.view,
                to: model,
            });
        } else {
            self.motion = None;
            self.view = model;
        }
    }

    /// The app asks to show `request.bounds`: the camera moves there
    /// (gliding when `animate`) and reports the view as a gesture would.
    /// Each request is served once, as soon as the map has a size.
    pub fn fit(&mut self, request: FitRequest, animate: bool, now: Instant) {
        if self.fitted == Some(request.serial) || !self.size.is_finite_and_positive() {
            return;
        }
        self.fitted = Some(request.serial);
        let to = fit_view(request.bounds, self.size);
        self.drag = None;
        self.settle = None;
        self.moved = true;
        if animate {
            self.motion = Some(Motion::Fly {
                start: now,
                from: self.view,
                to,
            });
        } else {
            self.motion = None;
            self.view = to;
        }
    }

    /// The pointer went down at `at` (local points): a drag starts and any
    /// motion stops where it is.
    pub fn press(&mut self, at: Point, now: Instant) {
        self.motion = None;
        self.drag = Some(Drag {
            last: at,
            samples: vec![(now, at)],
        });
    }

    /// Stop gliding or zooming where it is (the view is reported as
    /// usual).
    pub fn halt(&mut self) {
        self.motion = None;
    }

    pub fn dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// The pointer moved to `at` during a drag; returns whether the view
    /// changed.
    pub fn drag_to(&mut self, at: Point, now: Instant) -> bool {
        let Some(drag) = &mut self.drag else {
            return false;
        };
        let (dx, dy) = (f64::from(at.x - drag.last.x), f64::from(at.y - drag.last.y));
        drag.last = at;
        drag.samples.push((now, at));
        drag.samples
            .retain(|(time, _)| now.saturating_duration_since(*time) <= VELOCITY_WINDOW);
        if dx == 0.0 && dy == 0.0 {
            return false;
        }
        self.view = self.camera().panned(dx, dy);
        self.moved = true;
        true
    }

    /// The pointer went up: the map glides on if it was moving fast.
    pub fn release(&mut self, now: Instant) {
        let Some(drag) = self.drag.take() else {
            return;
        };
        let recent: Vec<_> = drag
            .samples
            .iter()
            .filter(|(time, _)| now.saturating_duration_since(*time) <= VELOCITY_WINDOW)
            .collect();
        let (Some((t0, p0)), Some((t1, p1))) = (recent.first(), recent.last()) else {
            return;
        };
        let dt = t1.saturating_duration_since(*t0).as_secs_f64();
        if dt <= 0.0 {
            return;
        }
        let velocity = (f64::from(p1.x - p0.x) / dt, f64::from(p1.y - p0.y) / dt);
        if velocity.0.hypot(velocity.1) >= INERTIA_MIN_SPEED {
            self.motion = Some(Motion::Inertia {
                start: now,
                velocity,
                applied: (0.0, 0.0),
            });
        }
    }

    /// Move the content by `(dx, dy)` points (trackpad scroll, arrow keys).
    pub fn pan(&mut self, dx: f64, dy: f64, now: Instant) {
        self.motion = None;
        self.view = self.camera().panned(dx, dy);
        self.moved = true;
        self.settle = Some(now + SETTLE_DELAY);
    }

    /// Zoom by `delta` levels around `anchor` (local points), animated:
    /// a zoom under way goes on from its target.
    pub fn zoom_by(&mut self, delta: f64, anchor: Point, now: Instant) {
        let from = self.view.zoom;
        let target = match self.motion {
            Some(Motion::Zoom { to, .. }) => to,
            _ => from,
        };
        let to = (target + delta).clamp(crate::geo::MIN_ZOOM, crate::geo::MAX_ZOOM);
        if to == from {
            return;
        }
        self.motion = Some(Motion::Zoom {
            start: now,
            anchor,
            world: self.camera().local_to_world(anchor),
            from,
            to,
        });
        self.moved = true;
    }

    /// Scale the map by `factor` around `anchor` at once (a pinch step).
    pub fn pinch(&mut self, factor: f64, anchor: Point, now: Instant) {
        if !(factor.is_finite() && factor > 0.0) {
            return;
        }
        self.motion = None;
        self.view = self
            .camera()
            .zoomed_around(anchor, self.view.zoom + factor.log2());
        self.moved = true;
        self.settle = Some(now + SETTLE_DELAY);
    }

    /// End a gesture whose end the platform tells (a pinch): report now.
    pub fn end_gesture(&mut self) {
        self.settle = None;
    }

    /// Advance motion to `now`; returns whether the view changed.
    pub fn step(&mut self, now: Instant) -> bool {
        let Some(motion) = self.motion else {
            return false;
        };
        let before = self.view;
        match motion {
            Motion::Inertia {
                start,
                velocity,
                applied,
            } => {
                let speed = velocity.0.hypot(velocity.1);
                let end = INERTIA_TAU * (speed / INERTIA_STOP_SPEED).ln().max(0.0);
                let t = now.saturating_duration_since(start).as_secs_f64().min(end);
                let travelled = INERTIA_TAU * (1.0 - (-t / INERTIA_TAU).exp());
                let total = (velocity.0 * travelled, velocity.1 * travelled);
                self.view = self
                    .camera()
                    .panned(total.0 - applied.0, total.1 - applied.1);
                self.motion = (t < end).then_some(Motion::Inertia {
                    start,
                    velocity,
                    applied: total,
                });
            }
            Motion::Zoom {
                start,
                anchor,
                world,
                from,
                to,
            } => {
                let t = progress(start, now, ZOOM_DURATION);
                let zoom = from + (to - from) * ease_out(t);
                self.view = anchored_view(world, anchor, zoom, self.size);
                if t >= 1.0 {
                    self.motion = None;
                }
            }
            Motion::Fly { start, from, to } => {
                let t = progress(start, now, FLY_DURATION);
                let e = ease_out(t);
                let (a, b) = (project(from.center), project(to.center));
                self.view = if t >= 1.0 {
                    self.motion = None;
                    to
                } else {
                    MapView {
                        center: unproject(WorldPoint {
                            x: a.x + (b.x - a.x) * e,
                            y: a.y + (b.y - a.y) * e,
                        }),
                        zoom: from.zoom + (to.zoom - from.zoom) * e,
                    }
                };
            }
        }
        self.view != before
    }

    /// Something moves the map without input: frames are needed.
    pub fn animating(&self) -> bool {
        self.motion.is_some()
    }

    /// No drag, motion or burst under way at `now`: the view holds.
    pub fn still(&self, now: Instant) -> bool {
        self.drag.is_none() && self.motion.is_none() && self.settle.is_none_or(|due| due <= now)
    }

    /// When a burst gesture is due to be reported.
    pub fn deadline(&self) -> Option<Instant> {
        self.settle.filter(|_| self.moved)
    }

    /// The view to report once a gesture moved the map and settled (no
    /// drag, motion or burst under way); each gesture reports once.
    pub fn take_report(&mut self, now: Instant) -> Option<MapView> {
        let settled = self.drag.is_none()
            && self.motion.is_none()
            && self.settle.is_none_or(|due| due <= now);
        if !(self.moved && settled) {
            return None;
        }
        self.moved = false;
        self.settle = None;
        self.model = self.view;
        Some(self.view)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{CameraState, FLY_DURATION, SETTLE_DELAY, ZOOM_DURATION, fit_view};
    use crate::geo::{Camera, LatLon, LatLonBounds, MapView};
    use crate::geometry::{Point, Size};
    use crate::maps::FitRequest;

    const PLAZA: LatLon = LatLon::new(-36.1415, -71.8265);
    const SIZE: Size = Size {
        width: 400.0,
        height: 300.0,
    };

    fn camera() -> CameraState {
        let mut camera = CameraState::new(MapView::new(PLAZA, 14.0));
        camera.size = SIZE;
        camera
    }

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn a_slow_drag_pans_and_reports_once_on_release() {
        let mut camera = camera();
        let t = Instant::now();
        camera.press(Point { x: 100.0, y: 100.0 }, t);
        assert!(camera.drag_to(Point { x: 110.0, y: 95.0 }, t + ms(200)));
        assert!(camera.drag_to(Point { x: 120.0, y: 90.0 }, t + ms(400)));
        assert_eq!(camera.take_report(t + ms(400)), None, "still dragging");
        camera.release(t + ms(700));
        assert!(!camera.animating(), "too slow for inertia");
        let plaza = Camera::new(camera.view, SIZE).to_local(PLAZA);
        assert!((plaza.x - 220.0).abs() < 1e-3 && (plaza.y - 140.0).abs() < 1e-3);
        let report = camera.take_report(t + ms(700));
        assert_eq!(report, Some(camera.view));
        assert_eq!(camera.take_report(t + ms(800)), None, "one report");
        // The app echoing the report back moves nothing.
        camera.sync_model(report.unwrap(), true, t + ms(800));
        assert!(!camera.animating());
    }

    #[test]
    fn a_fast_release_glides_and_stops() {
        let mut camera = camera();
        let t = Instant::now();
        camera.press(Point { x: 100.0, y: 100.0 }, t);
        for step in 1..=5 {
            camera.drag_to(
                Point {
                    x: 100.0 + 20.0 * step as f32,
                    y: 100.0,
                },
                t + ms(16 * step),
            );
        }
        let released = t + ms(80);
        camera.release(released);
        assert!(camera.animating());
        let at_release = Camera::new(camera.view, SIZE).to_local(PLAZA).x;
        assert_eq!(camera.take_report(released), None, "gliding");
        camera.step(released + ms(100));
        let gliding = Camera::new(camera.view, SIZE).to_local(PLAZA).x;
        assert!(gliding > at_release + 50.0, "{at_release} → {gliding}");
        camera.step(released + Duration::from_secs(5));
        assert!(!camera.animating());
        let stopped = Camera::new(camera.view, SIZE).to_local(PLAZA).x;
        // 1250 points/s × τ = 406 points at most.
        assert!(stopped - at_release < 410.0 && stopped - at_release > 380.0);
        assert!(
            camera
                .take_report(released + Duration::from_secs(5))
                .is_some()
        );
    }

    #[test]
    fn zoom_animations_keep_the_anchor_and_chain() {
        let mut camera = camera();
        let t = Instant::now();
        let anchor = Point { x: 300.0, y: 80.0 };
        let under = Camera::new(camera.view, SIZE).to_lat_lon(anchor);
        camera.zoom_by(1.0, anchor, t);
        camera.step(t + ms(100));
        let middle = camera.view.zoom;
        assert!(middle > 14.0 && middle < 15.0);
        let at = Camera::new(camera.view, SIZE).to_local(under);
        assert!((at.x - anchor.x).abs() < 1e-3 && (at.y - anchor.y).abs() < 1e-3);
        camera.zoom_by(1.0, anchor, t + ms(100));
        camera.step(t + ms(100) + ZOOM_DURATION);
        assert_eq!(
            camera.view.zoom, 16.0,
            "the second notch adds to the target"
        );
        assert!(!camera.animating());
        assert!(camera.take_report(t + ms(400)).is_some());
    }

    #[test]
    fn bursts_report_after_a_quiet_spell() {
        let mut camera = camera();
        let t = Instant::now();
        camera.pan(10.0, 0.0, t);
        camera.pan(10.0, 0.0, t + ms(50));
        assert_eq!(camera.deadline(), Some(t + ms(50) + SETTLE_DELAY));
        assert_eq!(camera.take_report(t + ms(100)), None);
        assert!(camera.take_report(t + ms(50) + SETTLE_DELAY).is_some());
        camera.pinch(2.0, Point { x: 200.0, y: 150.0 }, t + ms(500));
        assert_eq!(camera.view.zoom, 15.0);
        camera.end_gesture();
        assert!(camera.take_report(t + ms(500)).is_some());
    }

    #[test]
    fn model_changes_fly_unless_a_drag_is_under_way() {
        let mut camera = camera();
        let t = Instant::now();
        let target = MapView::new(LatLon::new(-36.13, -71.84), 15.0);
        camera.sync_model(target, true, t);
        assert!(camera.animating());
        camera.step(t + FLY_DURATION);
        assert_eq!(camera.view, target);
        assert_eq!(
            camera.take_report(t + FLY_DURATION),
            None,
            "the app moved it"
        );

        camera.press(Point::default(), t);
        camera.sync_model(MapView::new(PLAZA, 12.0), false, t);
        assert_eq!(camera.view, target, "the drag wins");
        camera.sync_model(MapView::new(PLAZA, 11.0), false, t);
        camera.release(t);
        camera.sync_model(MapView::new(PLAZA, 10.0), false, t);
        assert_eq!(
            camera.view,
            MapView::new(PLAZA, 10.0),
            "jumps without animation"
        );
    }

    #[test]
    fn a_fit_request_glides_to_the_bounds_once_and_reports() {
        let bounds = LatLonBounds {
            south: -36.148,
            west: -71.84,
            north: -36.135,
            east: -71.81,
        };
        let request = FitRequest { bounds, serial: 1 };
        let t = Instant::now();
        let mut no_size = CameraState::new(MapView::new(PLAZA, 14.0));
        no_size.fit(request, true, t);
        assert!(!no_size.animating(), "it waits for a size");

        let mut camera = camera();
        camera.fit(request, true, t);
        assert!(camera.animating());
        camera.step(t + FLY_DURATION);
        let view = Camera::new(camera.view, SIZE);
        for corner in [
            LatLon::new(bounds.north, bounds.west),
            LatLon::new(bounds.south, bounds.east),
        ] {
            let at = view.to_local(corner);
            assert!(at.x >= 15.9 && at.x <= SIZE.width - 15.9, "{at:?}");
            assert!(at.y >= 15.9 && at.y <= SIZE.height - 15.9, "{at:?}");
        }
        assert_eq!(camera.take_report(t + FLY_DURATION), Some(camera.view));

        camera.pan(50.0, 0.0, t + FLY_DURATION);
        let panned = camera.view;
        camera.fit(request, true, t + FLY_DURATION);
        assert_eq!(camera.view, panned, "the same request is served once");
        camera.fit(FitRequest { bounds, serial: 2 }, false, t + FLY_DURATION);
        assert_eq!(camera.view, fit_view(bounds, SIZE), "a new one jumps");
    }
}
