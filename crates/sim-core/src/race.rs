//! Deterministic local race-progress tracking for a single car.
//!
//! The tracker uses the race layout centreline from [`RaceData`], not the
//! visual racing line. It projects the car position onto the nearest layout
//! segment, derives normalized progress and sector, and uses progress wraps to
//! detect start/finish crossings.

use glam::Vec3;
use sim_types::{RaceData, RaceState};

use crate::vehicle::FIXED_DT;

/// Below this length a segment or layout is treated as having no extent.
const LENGTH_EPSILON: f32 = 1e-6;
/// Progress deltas below this are treated as stationary.
const MOVEMENT_EPSILON: f32 = 1e-7;
/// Tolerance for matching a progress value to a sector start.
const SECTOR_EPSILON: f32 = 1e-6;

/// Tracks lap, sector, progress, wrong-way state, and lap timing for one car.
///
/// Construct with [`RaceTracker::new`], feed positions with
/// [`RaceTracker::update`], and read the latest [`RaceState`] with
/// [`RaceTracker::state`]. All state is derived deterministically from the
/// supplied positions and ticks.
#[derive(Debug, Clone)]
pub struct RaceTracker {
    points: Vec<Vec3>,
    segment_lengths: Vec<f32>,
    cumulative: Vec<f32>,
    total_length: f32,
    sector_starts: Vec<f32>,
    valid: bool,
    state: RaceState,
    previous_progress: Option<f32>,
    started_tick: Option<u64>,
}

impl RaceTracker {
    /// Builds a tracker from race layout data.
    ///
    /// A layout with fewer than two finite points, or with zero total length,
    /// produces an inactive tracker that keeps returning its initial state.
    pub fn new(data: &RaceData) -> Self {
        let raw_points: Vec<Vec3> = data
            .layout
            .points
            .iter()
            .map(|point| Vec3::from_array(*point))
            .collect();
        let points_are_finite = raw_points.iter().all(|point| point.is_finite());

        let mut points = raw_points;
        let mut segment_lengths = Vec::new();
        let mut cumulative = vec![0.0f32];

        if points_are_finite && points.len() >= 2 {
            let segment_count = if data.layout.closed {
                points.len()
            } else {
                points.len() - 1
            };
            segment_lengths.reserve(segment_count);
            for index in 0..segment_count {
                let start = points[index];
                let end = points[(index + 1) % points.len()];
                let length = (end - start).length();
                segment_lengths.push(length);
                cumulative.push(cumulative.last().copied().unwrap_or(0.0) + length);
            }
        }

        let total_length = cumulative.last().copied().unwrap_or(0.0);
        let valid = points_are_finite
            && points.len() >= 2
            && !segment_lengths.is_empty()
            && total_length.is_finite()
            && total_length > LENGTH_EPSILON;

        if !valid {
            points.clear();
        }

        let mut sector_starts: Vec<f32> = data
            .sectors
            .iter()
            .filter(|sector| sector.start_fraction.is_finite())
            .map(|sector| sector.start_fraction.clamp(0.0, 1.0))
            .collect();
        sector_starts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        sector_starts.dedup_by(|a, b| (*a - *b).abs() <= SECTOR_EPSILON);

        Self {
            points,
            segment_lengths,
            cumulative,
            total_length,
            sector_starts,
            valid,
            state: RaceState::initial(),
            previous_progress: None,
            started_tick: None,
        }
    }

    /// Projects `position` onto the layout, updates race state, and returns it.
    ///
    /// Non-finite positions are ignored: the previous state is returned
    /// unchanged. The first call only establishes the baseline progress; a lap
    /// timer starts on the first observed forward movement.
    pub fn update(&mut self, position: Vec3, tick: u64) -> RaceState {
        if !self.valid || !position.is_finite() {
            return self.state.clone();
        }

        let (distance, progress) = self.project(position);

        let Some(previous) = self.previous_progress else {
            self.previous_progress = Some(progress);
            self.state.progress = progress;
            self.state.distance_m = distance;
            self.state.sector = self.sector_for(progress);
            return self.state.clone();
        };

        let forward_crossing = previous > 0.8 && progress < 0.2;
        let backward_crossing = previous < 0.2 && progress > 0.8;

        if forward_crossing {
            let lap_time = self.elapsed(tick);
            self.state.lap = self.state.lap.saturating_add(1);
            self.state.last_lap_time = Some(lap_time);
            self.state.best_lap_time = Some(match self.state.best_lap_time {
                Some(best) => best.min(lap_time),
                None => lap_time,
            });
            self.state.current_lap_time = 0.0;
            self.state.started = true;
            self.state.wrong_way = false;
            self.started_tick = Some(tick);
        } else if backward_crossing {
            self.state.wrong_way = true;
            if self.state.started {
                self.state.current_lap_time = self.elapsed(tick);
            }
        } else {
            let delta = progress - previous;
            if delta > MOVEMENT_EPSILON {
                self.state.wrong_way = false;
                if self.state.started {
                    self.state.current_lap_time = self.elapsed(tick);
                } else {
                    self.state.started = true;
                    self.state.current_lap_time = 0.0;
                    self.started_tick = Some(tick);
                }
            } else if delta < -MOVEMENT_EPSILON {
                self.state.wrong_way = true;
                if self.state.started {
                    self.state.current_lap_time = self.elapsed(tick);
                }
            } else if self.state.started {
                self.state.current_lap_time = self.elapsed(tick);
            }
        }

        self.state.progress = progress;
        self.state.distance_m = distance;
        self.state.sector = self.sector_for(progress);
        self.previous_progress = Some(progress);
        self.state.clone()
    }

    /// Returns the latest race state.
    pub fn state(&self) -> RaceState {
        self.state.clone()
    }

    /// Resets lap progress, sector, timer, started, and wrong-way state.
    ///
    /// Best (and last) lap times are preserved across a car reset.
    pub fn reset(&mut self) {
        let best_lap_time = self.state.best_lap_time;
        let last_lap_time = self.state.last_lap_time;
        self.state = RaceState::initial();
        self.state.best_lap_time = best_lap_time;
        self.state.last_lap_time = last_lap_time;
        self.previous_progress = None;
        self.started_tick = None;
    }

    fn project(&self, position: Vec3) -> (f32, f32) {
        let mut best_distance_sq = f32::INFINITY;
        let mut best_along = 0.0f32;

        for (index, length) in self.segment_lengths.iter().enumerate() {
            let start = self.points[index];
            let end = self.points[(index + 1) % self.points.len()];
            let segment = end - start;
            let length_sq = segment.length_squared();
            let t = if length_sq <= LENGTH_EPSILON * LENGTH_EPSILON {
                0.0
            } else {
                ((position - start).dot(segment) / length_sq).clamp(0.0, 1.0)
            };
            let nearest = start + segment * t;
            let distance_sq = (position - nearest).length_squared();
            if distance_sq < best_distance_sq {
                best_distance_sq = distance_sq;
                let along = self.cumulative[index] + t * *length;
                best_along = along;
            }
        }

        let distance = best_along.clamp(0.0, self.total_length);
        (distance, distance / self.total_length)
    }

    fn sector_for(&self, progress: f32) -> u32 {
        let mut sector = 1u32;
        for (index, start) in self.sector_starts.iter().enumerate() {
            if progress + SECTOR_EPSILON >= *start {
                sector = index as u32 + 1;
            } else {
                break;
            }
        }
        sector
    }

    fn elapsed(&self, tick: u64) -> f64 {
        self.started_tick
            .map(|start| tick.saturating_sub(start) as f64 * FIXED_DT as f64)
            .unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_types::{
        Layout, LayoutDirection, SectorSpec, SpawnPoint, StartFinish, RACE_DATA_VERSION,
    };

    fn sector(name: &str, start_fraction: f32, end_fraction: f32) -> SectorSpec {
        SectorSpec {
            name: String::from(name),
            start_fraction,
            end_fraction,
        }
    }

    fn race_data(points: Vec<[f32; 3]>, closed: bool, sectors: Vec<SectorSpec>) -> RaceData {
        RaceData {
            version: RACE_DATA_VERSION,
            layout: Layout {
                layout_id: String::from("test"),
                closed,
                direction: LayoutDirection::Forward,
                points,
                half_width: 5.0,
            },
            start_finish: StartFinish {
                section_id: String::from("test"),
                fraction: 0.0,
                position: [0.0, 0.0, 0.0],
                rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
            },
            grid: vec![SpawnPoint {
                name: String::from("grid_0"),
                position: [0.0, 0.0, 0.0],
                rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
            }],
            sectors,
        }
    }

    fn straight() -> RaceData {
        race_data(
            vec![[0.0, 0.0, 0.0], [10.0, 0.0, 0.0]],
            false,
            vec![sector("sector_1", 0.0, 1.0)],
        )
    }

    fn square() -> RaceData {
        race_data(
            vec![
                [0.0, 0.0, 0.0],
                [10.0, 0.0, 0.0],
                [10.0, 0.0, 10.0],
                [0.0, 0.0, 10.0],
            ],
            true,
            vec![
                sector("sector_1", 0.0, 0.25),
                sector("sector_2", 0.25, 0.5),
                sector("sector_3", 0.5, 0.75),
                sector("sector_4", 0.75, 1.0),
            ],
        )
    }

    fn square_point(progress: f32) -> Vec3 {
        let distance = progress * 40.0;
        if distance <= 10.0 {
            Vec3::new(distance, 0.0, 0.0)
        } else if distance <= 20.0 {
            Vec3::new(10.0, 0.0, distance - 10.0)
        } else if distance <= 30.0 {
            Vec3::new(10.0 - (distance - 20.0), 0.0, 10.0)
        } else {
            Vec3::new(0.0, 0.0, 10.0 - (distance - 30.0))
        }
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 1e-4,
            "expected {expected}, got {actual}"
        );
    }

    fn assert_time_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-6,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn projects_onto_nearest_segment() {
        let mut tracker = RaceTracker::new(&straight());
        let state = tracker.update(Vec3::new(2.5, 3.0, 0.0), 0);

        assert_close(state.distance_m, 2.5);
        assert_close(state.progress, 0.25);
        assert_eq!(state.lap, 1);
        assert_eq!(state.sector, 1);
        assert!(!state.started);
    }

    #[test]
    fn closed_layout_projects_onto_closing_segment() {
        let mut tracker = RaceTracker::new(&square());
        let state = tracker.update(Vec3::new(5.0, 0.0, 10.0), 0);

        assert_close(state.distance_m, 25.0);
        assert_close(state.progress, 0.625);
    }

    #[test]
    fn sector_changes_at_quarter_boundaries() {
        let mut tracker = RaceTracker::new(&square());

        assert_eq!(tracker.update(square_point(0.24), 0).sector, 1);
        assert_eq!(tracker.update(square_point(0.26), 1).sector, 2);
        assert_eq!(tracker.update(square_point(0.49), 2).sector, 2);
        assert_eq!(tracker.update(square_point(0.51), 3).sector, 3);
        assert_eq!(tracker.update(square_point(0.74), 4).sector, 3);
        assert_eq!(tracker.update(square_point(0.76), 5).sector, 4);
    }

    #[test]
    fn forward_crossing_increments_lap_and_records_times() {
        let mut tracker = RaceTracker::new(&square());
        tracker.update(square_point(0.10), 0);
        let started = tracker.update(square_point(0.20), 100);
        assert!(started.started);

        tracker.update(square_point(0.50), 600);
        tracker.update(square_point(0.90), 1100);
        let crossed = tracker.update(square_point(0.05), 1200);

        assert_eq!(crossed.lap, 2);
        assert_eq!(crossed.sector, 1);
        assert!(!crossed.wrong_way);
        assert!(crossed.started);
        assert_time_close(crossed.current_lap_time, 0.0);
        // Timer started at tick 100; crossing at tick 1200 is 1100 ticks.
        let expected_lap = 1100.0 * FIXED_DT as f64;
        assert_time_close(crossed.last_lap_time.expect("last lap"), expected_lap);
        assert_time_close(crossed.best_lap_time.expect("best lap"), expected_lap);
    }

    #[test]
    fn backward_crossing_sets_wrong_way_without_incrementing_lap() {
        let mut tracker = RaceTracker::new(&square());
        tracker.update(square_point(0.05), 0);
        tracker.update(square_point(0.10), 10);
        let crossed = tracker.update(square_point(0.90), 20);

        assert_eq!(crossed.lap, 1);
        assert!(crossed.wrong_way);
        assert_eq!(crossed.last_lap_time, None);
        assert_eq!(crossed.best_lap_time, None);
    }

    #[test]
    fn normal_reverse_sets_wrong_way_and_forward_clears_it() {
        let mut tracker = RaceTracker::new(&square());
        tracker.update(square_point(0.50), 0);
        tracker.update(square_point(0.60), 10);

        let reversed = tracker.update(square_point(0.55), 20);
        assert!(reversed.wrong_way);

        let forward = tracker.update(square_point(0.58), 30);
        assert!(!forward.wrong_way);
    }

    #[test]
    fn timer_starts_on_first_forward_movement() {
        let mut tracker = RaceTracker::new(&straight());

        let initial = tracker.update(Vec3::new(0.0, 0.0, 0.0), 0);
        assert!(!initial.started);
        assert_time_close(initial.current_lap_time, 0.0);

        let moving = tracker.update(Vec3::new(1.0, 0.0, 0.0), 50);
        assert!(moving.started);
        assert_time_close(moving.current_lap_time, 0.0);

        let later = tracker.update(Vec3::new(2.0, 0.0, 0.0), 100);
        assert_time_close(later.current_lap_time, 50.0 * FIXED_DT as f64);
    }

    #[test]
    fn reset_preserves_best_lap() {
        let mut tracker = RaceTracker::new(&square());
        tracker.update(square_point(0.10), 0);
        tracker.update(square_point(0.20), 100);
        tracker.update(square_point(0.90), 1000);
        let crossed = tracker.update(square_point(0.05), 1100);
        let best = crossed.best_lap_time.expect("best lap");
        assert!(best > 0.0);

        tracker.reset();
        let reset = tracker.state();

        assert_eq!(reset.lap, 1);
        assert_eq!(reset.sector, 1);
        assert_close(reset.progress, 0.0);
        assert_close(reset.distance_m, 0.0);
        assert!(!reset.started);
        assert!(!reset.wrong_way);
        assert_time_close(reset.current_lap_time, 0.0);
        assert_eq!(reset.best_lap_time, Some(best));
    }

    #[test]
    fn non_finite_position_is_ignored() {
        let mut tracker = RaceTracker::new(&square());
        tracker.update(square_point(0.30), 0);
        let before = tracker.update(square_point(0.40), 10);

        let nan = tracker.update(Vec3::new(f32::NAN, 0.0, 0.0), 20);
        assert_eq!(nan, before);
        let infinite = tracker.update(Vec3::new(0.0, f32::INFINITY, 0.0), 30);
        assert_eq!(infinite, before);
        assert_eq!(tracker.state(), before);
    }

    #[test]
    fn degenerate_layout_does_not_panic() {
        let empty = race_data(Vec::new(), true, Vec::new());
        let single = race_data(vec![[1.0, 2.0, 3.0]], true, Vec::new());
        let zero_length = race_data(vec![[1.0, 2.0, 3.0], [1.0, 2.0, 3.0]], true, Vec::new());

        for data in [empty, single, zero_length] {
            let mut tracker = RaceTracker::new(&data);
            let initial = tracker.state();
            let updated = tracker.update(Vec3::new(0.0, 0.0, 0.0), 10);
            assert_eq!(updated, initial);
            tracker.reset();
            assert_eq!(tracker.state(), initial);
        }
    }
}
