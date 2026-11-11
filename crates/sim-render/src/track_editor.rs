use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::f32::consts::{PI, TAU};

use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};
use sim_types::{
    Aabb, AssetKind, AssetManifest, AssetStats, CameraSpec, EntryPoints, LayoutSpec, MaterialRef,
    MeshBuffers, MeshRef, SectorSpec, SpawnPoint, SurfaceSpec, TextureRef, TrackCollision,
    TrackMetadata, TrackVisual, ENGINE_FRAME, ENGINE_UNITS, MAGIC, SCHEMA_VERSION,
};

pub const TRACK_PROJECT_SCHEMA: &str = "lapster/track-project@2";
pub const TRACK_PROJECT_SCHEMA_V1: &str = "lapster/track-project@1";
pub const EDITOR_PROJECT_SCHEMA: &str = "lapster/track-project@3";
pub const TRACK_PROJECT_SCHEMA_V4: &str = "lapster/track-project@4";
pub const DEFAULT_LAYOUT_WIDTH: f32 = 12.0;
pub const MIN_LAYOUT_WIDTH: f32 = 6.0;
pub const MAX_LAYOUT_WIDTH: f32 = 30.0;

const SEGMENTS: usize = 120;
const TRACK_SURFACE: u16 = 0;
const GRASS_SURFACE: u16 = 1;
const DIRT_SURFACE: u16 = 2;
const KERB_SURFACE: u16 = 3;
const WALL_SURFACE: u16 = 4;
const TERRAIN_SIZE: f32 = 300.0;
const TERRAIN_CELLS: usize = 128;
const MOUNTAIN_INNER_RADIUS: f32 = 120.0;
const MOUNTAIN_RISE_START: f32 = 700.0;
const MOUNTAIN_OUTER_RADIUS: f32 = 3600.0;
const MOUNTAIN_SEGMENTS: usize = 192;
const MOUNTAIN_RINGS: usize = 48;
const MOUNTAIN_GROUND_COLOR: [f32; 3] = [0.302, 0.431, 0.208];
const MOUNTAIN_ROCK_COLOR: [f32; 3] = [0.12, 0.16, 0.22];
const KERB_INNER_RADIUS: f32 = 6.0;
const KERB_OUTER_RADIUS: f32 = 6.8;
const KERB_BOTTOM_OFFSET: f32 = 0.02;
const KERB_SURFACE_OFFSET: f32 = 0.06;
const KERB_TOP_OFFSET: f32 = 0.155;
const OUTER_EDGE_RADIUS: f32 = 12.0;
const ASPHALT_HALF_WIDTH: f32 = 6.0;
const SURFACE_HEIGHT_OFFSET: f32 = 0.06;
const RACING_LINE_MATERIAL: u16 = 12;
const RACING_LINE_HALF_WIDTH: f32 = 1.6;
const RACING_LINE_EDGE_MARGIN: f32 = 0.3;
#[allow(dead_code)]
const RACING_LINE_MAX_OFFSET: f32 =
    ASPHALT_HALF_WIDTH - RACING_LINE_HALF_WIDTH - RACING_LINE_EDGE_MARGIN;
const RACING_LINE_REFERENCE_CURVATURE: f32 = 1.0 / 42.0;
const RACING_LINE_SMOOTHING_PASSES: usize = 16;
const RACING_LINE_LIFT: f32 = 0.02;
const RACING_LINE_ASPHALT_COLOR: [f32; 3] = [0.045, 0.045, 0.05];
const RACING_LINE_DARK_COLOR: [f32; 3] = [0.016, 0.016, 0.021];
const RACING_LINE_PROFILE: [f32; 7] = [-1.0, -0.8, -0.5, 0.0, 0.5, 0.8, 1.0];
const TARMAC_TEXTURE_INDEX: u32 = 1;
const TARMAC_TEXTURE_SIZE: usize = 256;
const TARMAC_TILE_METERS: f32 = 7.0;
// Keep in sync with TARMAC_GAIN and TARMAC_UV_ACROSS in track.wgsl.
const TARMAC_GAIN: f32 = 2.2;
const TARMAC_UV_ACROSS: f32 = 4.0;
const GRASS_TEXTURE_INDEX: u32 = 2;
const GRASS_TEXTURE_SIZE: usize = 256;
const TERRAIN_TILE_METERS: f32 = 8.0;
const PROP_OFFSET: f32 = 14.0;
const PROP_FAR_OFFSET: f32 = 19.0;
const MIN_PROP_OFFSET: f32 = 1.0;
const MAX_PROP_OFFSET: f32 = PROP_FAR_OFFSET;
const BLOCK_HALF_WIDTH: f32 = 0.3;
const BLOCK_HALF_HEIGHT: f32 = 0.4;
const BLOCK_HALF_LENGTH: f32 = 0.9;
const TYRE_RADIUS: f32 = 0.4;
const TYRE_LAYER_HEIGHT: f32 = 0.3;
const TYRE_LAYERS: usize = 3;
const TREE_SIDES: usize = 6;
const TREE_CONIFER_SIDES: usize = 6;
const TREE_CONIFER_TRUNK_SIDES: usize = 5;
const TREE_CONIFER_SIZE: f32 = 0.6;
const TREE_CARD_VARIANTS: usize = 5;
const TREE_CARD_ATLAS_WIDTH: usize = 640;
const TREE_CARD_ATLAS_HEIGHT: usize = 256;
const TREE_CARD_ATLAS_RGBA: &[u8] = include_bytes!("../assets/tree_cards.rgba");
const TREE_TRUNK_COLOR: [f32; 3] = [0.32, 0.23, 0.15];
const TREE_COMMON_COLOR: [f32; 3] = [0.34, 0.56, 0.21];
const TREE_COMMON_COLOR_ALT: [f32; 3] = [0.43, 0.63, 0.25];
const TREE_PINE_COLOR: [f32; 3] = [0.20, 0.42, 0.18];
const TREE_PINE_COLOR_ALT: [f32; 3] = [0.27, 0.49, 0.21];
const TREE_PINE_DARK: [f32; 3] = [0.12, 0.28, 0.12];
const TREE_CONIFER_TALL: &[(f32, f32, f32)] = &[
    (1.0, 2.0, 2.2),
    (2.1, 1.65, 2.0),
    (3.2, 1.3, 1.9),
    (4.3, 1.0, 1.8),
    (5.4, 0.7, 1.7),
    (6.5, 0.4, 1.5),
];
const TREE_CONIFER_BROAD: &[(f32, f32, f32)] = &[
    (1.1, 2.4, 2.4),
    (2.3, 1.9, 2.2),
    (3.5, 1.45, 2.0),
    (4.7, 1.05, 1.8),
    (5.9, 0.65, 1.6),
];
const TREE_CONIFER_YOUNG: &[(f32, f32, f32)] = &[
    (0.5, 1.25, 1.5),
    (1.4, 0.95, 1.4),
    (2.3, 0.65, 1.3),
    (3.2, 0.4, 1.2),
];
const MIN_TURN_RADIUS: f32 = OUTER_EDGE_RADIUS.max(PROP_FAR_OFFSET) + 1.0;
const CURVATURE_RELAXATION: f32 = 0.5;
const CURVATURE_RELAXATION_ITERATIONS: usize = 1024;
const BLOCK_SPACING: f32 = 2.0;
const TYRE_SPACING: f32 = 0.8;
const TREE_SPACING: f32 = 4.5;
const DEFAULT_NEAR_PROP_SEGMENTS: usize = 4;
const DEFAULT_NEAR_PROP_GAP: f32 = 2.0;
const DEFAULT_TREE_RING_SEGMENTS: usize = 6;
const DEFAULT_TREE_RING_GAP: f32 = 4.0;
const DEFAULT_KERB_BENDS: usize = 4;
const DEFAULT_KERB_BEND_HALF_LENGTH: f32 = 5.0;
const DEFAULT_KERB_SEGMENTS_PER_BEND: usize = 2;
const DEFAULT_KERB_SEGMENT_GAP: f32 = 2.0;
const KERB_MIN_CURVATURE: f32 = 0.015;
const KERB_PEAK_SPACING: f32 = 10.0;
const PROP_LATERAL_JITTER: f32 = 0.03;
const PROP_LONGITUDINAL_JITTER: f32 = 0.03;
const PROP_YAW_JITTER: f32 = 0.025;
const CONTROL_POINTS: [[f32; 3]; 8] = [
    [-30.0, 0.0, -20.0],
    [0.0, 0.0, -35.0],
    [30.0, 0.0, -25.0],
    [40.0, 0.0, 0.0],
    [28.130531, 0.0, 26.741423],
    [0.0, 0.0, 15.0],
    [-25.0, 0.0, 30.0],
    [-40.0, 0.0, 5.0],
];

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct KerbSpan {
    pub start: f32,
    pub end: f32,
    pub side: i8,
}

const DEFAULT_KERB_SPANS: [KerbSpan; 8] = [
    KerbSpan {
        start: 7.1667256,
        end: 23.240198,
        side: -1,
    },
    KerbSpan {
        start: 39.0,
        end: 61.400776,
        side: -1,
    },
    KerbSpan {
        start: 69.32667,
        end: 80.09991,
        side: -1,
    },
    KerbSpan {
        start: 87.39344,
        end: 116.45184,
        side: -1,
    },
    KerbSpan {
        start: 8.592459,
        end: 22.00123,
        side: 1,
    },
    KerbSpan {
        start: 36.739464,
        end: 55.780273,
        side: 1,
    },
    KerbSpan {
        start: 72.0,
        end: 77.918,
        side: 1,
    },
    KerbSpan {
        start: 91.39807,
        end: 112.857765,
        side: 1,
    },
];

#[allow(dead_code)]
pub fn default_kerb_spans() -> Vec<KerbSpan> {
    DEFAULT_KERB_SPANS.to_vec()
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PropKind {
    Blocks,
    Tyres,
    Trees,
    Off,
}

fn serde_default_prop_offset() -> f32 {
    PROP_OFFSET
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct PropLine {
    pub start: f32,
    pub end: f32,
    pub side: i8,
    #[serde(default = "serde_default_prop_offset")]
    pub offset: f32,
    pub kind: PropKind,
}

/// Venue graph document authored by the procedural track editor. One project
/// contains one venue graph, its globally shared start/finish and pit lane, and
/// any number of closed routes over the shared edges.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TrackProject {
    pub schema: String,
    pub name: String,
    pub settings: VenueSettings,
    pub start_finish: StartFinish,
    #[serde(default)]
    pub pit_lane: Option<PitLaneProject>,
    pub control_points: Vec<ControlPoint>,
    pub edges: Vec<EdgeProject>,
    /// Legacy `@3` decoration list retained for migration input only. It is
    /// never written back into the `@4` document; per-segment decorations live
    /// on [`EdgeProject`].
    #[serde(default, skip_serializing)]
    pub segments: Vec<EditorSegment>,
    pub layouts: Vec<LayoutProject>,
    #[serde(default)]
    pub paths: Vec<PathProject>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ControlPointKind {
    #[default]
    Normal,
    Fork,
    PitEntry,
    PitExit,
    StartFinish,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SplineRole {
    #[default]
    Surface,
    Prop,
    Racing,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SplineSpec {
    pub id: String,
    pub role: SplineRole,
    #[serde(default)]
    pub base: Option<String>,
    #[serde(default)]
    pub offset: f32,
    #[serde(default)]
    pub surface: Option<String>,
}

/// A cross-section profile: surface-spline id -> offset in metres.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    #[serde(default)]
    pub values: BTreeMap<String, f32>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SideWidths {
    pub left: f32,
    pub right: f32,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KerbSettings {
    pub width: f32,
    pub depth: f32,
    pub element_length: f32,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PitProfile {
    pub asphalt: SideWidths,
    pub runoff: SideWidths,
    #[serde(default)]
    pub kerbs: bool,
}

impl Default for PitProfile {
    fn default() -> Self {
        Self {
            asphalt: SideWidths {
                left: 4.0,
                right: 4.0,
            },
            runoff: SideWidths {
                left: 2.0,
                right: 2.0,
            },
            kerbs: false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VenueSettings {
    pub splines: Vec<SplineSpec>,
    pub kerb: KerbSettings,
    #[serde(default)]
    pub pit: PitProfile,
}

impl Default for VenueSettings {
    fn default() -> Self {
        Self {
            splines: default_splines(DEFAULT_LAYOUT_WIDTH),
            kerb: KerbSettings {
                width: 0.8,
                depth: 0.135,
                element_length: 2.0,
            },
            pit: PitProfile::default(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StartFinish {
    pub edge: String,
    #[serde(default)]
    pub fraction: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ControlPoint {
    pub id: String,
    pub position: [f32; 2],
    #[serde(default)]
    pub kind: ControlPointKind,
    #[serde(default)]
    pub profile: Option<Profile>,
    /// Branch tangents keyed by incident edge id. Each tangent is the XZ
    /// direction of canonical edge travel at this control point.
    #[serde(default)]
    pub branch_tangents: BTreeMap<String, [f32; 2]>,
}

/// A prop placement span. `start` and `end` are fractions in `[0, 1]` along
/// the containing edge.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PropSpan {
    pub spline: String,
    pub start: f32,
    pub end: f32,
    pub kind: PropKind,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EdgeProject {
    pub id: String,
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub start_profile: Option<Profile>,
    #[serde(default)]
    pub end_profile: Option<Profile>,
    #[serde(default)]
    pub kerbs: Vec<KerbSpan>,
    #[serde(default)]
    pub prop_spans: Vec<PropSpan>,
    #[serde(default)]
    pub points: Vec<[f32; 2]>,
    #[serde(default)]
    pub from_tangent: Option<[f32; 2]>,
    #[serde(default)]
    pub to_tangent: Option<[f32; 2]>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LayoutProject {
    pub id: String,
    pub name: String,
    pub route: Vec<String>,
    #[serde(default)]
    pub sectors: Vec<SectorSpec>,
    #[serde(default)]
    pub cameras: Vec<CameraSpec>,
    #[serde(default)]
    pub spawns: Vec<SpawnPoint>,
    #[serde(default)]
    pub start_finish: StartFinish,
}

/// A drivable, non-selectable path over canonical edges. The pit lane is a
/// path; paths may be open and are never listed as layouts.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PathProject {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub route: Vec<String>,
    #[serde(default)]
    pub boxes: Vec<SpawnPoint>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PitLaneProject {
    pub id: String,
    pub route: Vec<String>,
    #[serde(default)]
    pub boxes: Vec<SpawnPoint>,
}

/// Public editor document (`lapster/track-project@3`). Loops are closed
/// sequences of shared control-point ids; the venue graph is derived from them
/// internally for geometry.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EditorProject {
    pub schema: String,
    pub name: String,
    pub settings: EditorSettings,
    pub loops: Vec<EditorLoop>,
    pub control_points: Vec<EditorControlPoint>,
    #[serde(default)]
    pub segments: Vec<EditorSegment>,
    #[serde(default)]
    pub pit_lane: Option<EditorPitLane>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EditorSettings {
    pub width: f32,
    pub kerb: KerbSettings,
    #[serde(default)]
    pub pit: Option<PitProfile>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EditorLoop {
    pub id: String,
    pub name: String,
    pub control_points: Vec<String>,
    #[serde(default)]
    pub sectors: Vec<SectorSpec>,
    #[serde(default)]
    pub cameras: Vec<CameraSpec>,
    #[serde(default)]
    pub spawns: Vec<SpawnPoint>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EditorControlPoint {
    pub id: String,
    pub position: [f32; 2],
    #[serde(default)]
    pub kind: ControlPointKind,
    #[serde(default)]
    pub tangent: Option<[f32; 2]>,
}

/// Read-only loop control-point details used by the editor UI.
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EditorPointInfo {
    pub id: String,
    pub position: [f32; 2],
    pub kind: ControlPointKind,
    pub shared: bool,
    pub shared_count: usize,
}

/// Whether a segment belongs to a main route or a branch route. The role is
/// informational for validation; it only affects derived branch tangents.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EditorSegmentRole {
    #[default]
    Main,
    Branch,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EditorSegment {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub role: EditorSegmentRole,
    #[serde(default)]
    pub kerbs: Vec<KerbSpan>,
    #[serde(default)]
    pub prop_spans: Vec<PropSpan>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EditorPitLane {
    pub control_points: Vec<String>,
    #[serde(default)]
    pub boxes: Vec<SpawnPoint>,
}

fn default_layout_width() -> f32 {
    DEFAULT_LAYOUT_WIDTH
}

fn default_splines(asphalt_width: f32) -> Vec<SplineSpec> {
    let half = asphalt_width * 0.5;
    vec![
        SplineSpec {
            id: "runoff-left".to_string(),
            role: SplineRole::Surface,
            base: None,
            offset: -asphalt_width,
            surface: Some("grass".to_string()),
        },
        SplineSpec {
            id: "asphalt-left".to_string(),
            role: SplineRole::Surface,
            base: None,
            offset: -half,
            surface: Some("asphalt".to_string()),
        },
        SplineSpec {
            id: "asphalt-right".to_string(),
            role: SplineRole::Surface,
            base: None,
            offset: half,
            surface: Some("asphalt".to_string()),
        },
        SplineSpec {
            id: "runoff-right".to_string(),
            role: SplineRole::Surface,
            base: None,
            offset: asphalt_width,
            surface: Some("grass".to_string()),
        },
        SplineSpec {
            id: "prop-near-left".to_string(),
            role: SplineRole::Prop,
            base: Some("runoff-left".to_string()),
            offset: -2.0,
            surface: None,
        },
        SplineSpec {
            id: "prop-far-left".to_string(),
            role: SplineRole::Prop,
            base: Some("runoff-left".to_string()),
            offset: -7.0,
            surface: None,
        },
        SplineSpec {
            id: "prop-near-right".to_string(),
            role: SplineRole::Prop,
            base: Some("runoff-right".to_string()),
            offset: 2.0,
            surface: None,
        },
        SplineSpec {
            id: "prop-far-right".to_string(),
            role: SplineRole::Prop,
            base: Some("runoff-right".to_string()),
            offset: 7.0,
            surface: None,
        },
    ]
}

/// Resolves the effective lateral offset of a prop spline relative to the
/// centreline by adding its base surface spline offset.
fn prop_spline_lateral(splines: &[SplineSpec], spline: &SplineSpec) -> f32 {
    let base_offset = spline
        .base
        .as_ref()
        .and_then(|base| splines.iter().find(|candidate| candidate.id == *base))
        .map(|base| base.offset)
        .unwrap_or(0.0);
    base_offset + spline.offset
}

/// Picks the prop spline that best matches a legacy prop line. Lines on the
/// left map to left splines and vice versa; the near/far threshold splits the
/// two default lanes.
fn select_prop_spline_id(splines: &[SplineSpec], line: &PropLine, width: f32) -> Option<String> {
    let left = line.side < 0;
    let threshold = (default_prop_offset(width) + default_prop_far_offset(width)) * 0.5;
    let want_near = line.offset.abs() <= threshold;
    let mut best: Option<(f32, &SplineSpec)> = None;
    for spline in splines
        .iter()
        .filter(|spline| spline.role == SplineRole::Prop)
    {
        let lateral = prop_spline_lateral(splines, spline);
        if (lateral < 0.0) != left {
            continue;
        }
        if (lateral.abs() <= threshold) != want_near {
            continue;
        }
        let distance = (lateral.abs() - line.offset.abs()).abs();
        let closer = match best {
            Some((best_distance, _)) => distance < best_distance,
            None => true,
        };
        if closer {
            best = Some((distance, spline));
        }
    }
    if best.is_none() {
        for spline in splines
            .iter()
            .filter(|spline| spline.role == SplineRole::Prop)
        {
            let lateral = prop_spline_lateral(splines, spline);
            if (lateral < 0.0) != left {
                continue;
            }
            let distance = (lateral.abs() - line.offset.abs()).abs();
            let closer = match best {
                Some((best_distance, _)) => distance < best_distance,
                None => true,
            };
            if closer {
                best = Some((distance, spline));
            }
        }
    }
    best.map(|(_, spline)| spline.id.clone())
}

fn unique_id<'a>(existing: impl Iterator<Item = &'a str>, base: &str) -> String {
    let existing = existing.collect::<BTreeSet<_>>();
    if !existing.contains(base) {
        return base.to_string();
    }
    let mut suffix = 2;
    loop {
        let candidate = format!("{base}-{suffix}");
        if !existing.contains(candidate.as_str()) {
            return candidate;
        }
        suffix += 1;
    }
}

#[allow(dead_code)]
impl TrackProject {
    /// The built-in single-layout venue; matches the original editor loop.
    pub fn default_project() -> Self {
        let control_points = CONTROL_POINTS
            .iter()
            .map(|point| [point[0], point[2]])
            .collect::<Vec<_>>();
        Self::ring_project(
            "Procedural Drift Loop",
            "procedural-drift-loop",
            &control_points,
        )
    }

    fn ring_project(name: &str, layout_id: &str, control_points: &[[f32; 2]]) -> Self {
        let control_points = control_points
            .iter()
            .enumerate()
            .map(|(index, position)| ControlPoint {
                id: format!("control-{index}"),
                position: *position,
                kind: ControlPointKind::Normal,
                profile: None,
                branch_tangents: BTreeMap::new(),
            })
            .collect::<Vec<_>>();
        let edges = (0..control_points.len())
            .map(|index| EdgeProject {
                id: format!("edge-{index}"),
                from: control_points[index].id.clone(),
                to: control_points[(index + 1) % control_points.len()]
                    .id
                    .clone(),
                start_profile: None,
                end_profile: None,
                kerbs: Vec::new(),
                prop_spans: Vec::new(),
                points: Vec::new(),
                from_tangent: None,
                to_tangent: None,
            })
            .collect::<Vec<_>>();
        let route = edges.iter().map(|edge| edge.id.clone()).collect::<Vec<_>>();
        let start_finish = StartFinish {
            edge: edges
                .first()
                .map(|edge| edge.id.clone())
                .unwrap_or_default(),
            fraction: 0.0,
        };
        Self {
            schema: TRACK_PROJECT_SCHEMA_V4.to_string(),
            name: name.to_string(),
            settings: VenueSettings::default(),
            start_finish: start_finish.clone(),
            pit_lane: None,
            control_points,
            edges,
            segments: Vec::new(),
            layouts: vec![LayoutProject {
                id: layout_id.to_string(),
                name: name.to_string(),
                route,
                sectors: Vec::new(),
                cameras: Vec::new(),
                spawns: Vec::new(),
                start_finish,
            }],
            paths: Vec::new(),
        }
    }

    /// Parses an `@4` venue graph, the current `@2` project, a legacy `@1`
    /// project, or migrates an `@3` loop document through [`EditorProject`].
    /// `@1` layouts are migrated into independent rings; the start/finish is
    /// taken from the first edge of the legacy default layout.
    pub fn from_json(json: &str) -> Result<Self, String> {
        let value = serde_json::from_str::<serde_json::Value>(json)
            .map_err(|error| format!("invalid track project JSON: {error}"))?;
        let schema = value
            .get("schema")
            .and_then(|schema| schema.as_str())
            .unwrap_or_default()
            .to_string();
        match schema.as_str() {
            TRACK_PROJECT_SCHEMA | TRACK_PROJECT_SCHEMA_V4 => {
                serde_json::from_value::<TrackProject>(value)
                    .map_err(|error| format!("invalid track project: {error}"))?
                    .normalize()
            }
            TRACK_PROJECT_SCHEMA_V1 => {
                let legacy = serde_json::from_value::<LegacyTrackProject>(value)
                    .map_err(|error| format!("invalid legacy track project: {error}"))?;
                legacy.migrate()?.normalize()
            }
            EDITOR_PROJECT_SCHEMA => EditorProject::from_json(json)?.to_graph()?.normalize(),
            other => Err(format!(
                "unsupported track project schema '{other}'; expected {TRACK_PROJECT_SCHEMA_V4}, {EDITOR_PROJECT_SCHEMA}, {TRACK_PROJECT_SCHEMA}, or {TRACK_PROJECT_SCHEMA_V1}"
            )),
        }
    }

    pub fn normalize(mut self) -> Result<Self, String> {
        self.normalize_in_place()?;
        Ok(self)
    }

    fn normalize_in_place(&mut self) -> Result<(), String> {
        if self.schema != TRACK_PROJECT_SCHEMA && self.schema != TRACK_PROJECT_SCHEMA_V4 {
            return Err(format!(
                "unsupported track project schema '{}'; expected {TRACK_PROJECT_SCHEMA_V4} or {TRACK_PROJECT_SCHEMA}",
                self.schema
            ));
        }
        self.name = self.name.trim().to_string();
        self.start_finish.fraction = if self.start_finish.fraction.is_finite() {
            if self.start_finish.fraction < 0.0 || self.start_finish.fraction >= 1.0 {
                0.0
            } else {
                self.start_finish.fraction
            }
        } else {
            0.0
        };

        if self.control_points.is_empty() {
            return Err("track project needs at least one control point".to_string());
        }
        validate_unique_ids(
            self.control_points.iter().map(|point| point.id.as_str()),
            "control point",
        )?;
        validate_unique_ids(self.edges.iter().map(|edge| edge.id.as_str()), "edge")?;
        validate_unique_ids(
            self.layouts.iter().map(|layout| layout.id.as_str()),
            "layout",
        )?;
        validate_unique_ids(
            self.settings
                .splines
                .iter()
                .map(|spline| spline.id.as_str()),
            "spline",
        )?;

        for point in &self.control_points {
            if point.id.trim().is_empty() {
                return Err("control point ids must not be empty".to_string());
            }
            if !point.position[0].is_finite() || !point.position[1].is_finite() {
                return Err(format!(
                    "control point '{}' has a non-finite position",
                    point.id
                ));
            }
        }

        validate_splines(&self.settings.splines)?;
        validate_profile(
            &Profile::default(),
            &self.settings.splines,
            "venue default profile",
        )?;

        {
            let mut incident_edges = BTreeMap::<&str, BTreeSet<&str>>::new();
            for edge in &self.edges {
                incident_edges
                    .entry(edge.from.as_str())
                    .or_default()
                    .insert(edge.id.as_str());
                incident_edges
                    .entry(edge.to.as_str())
                    .or_default()
                    .insert(edge.id.as_str());
            }
            for point in &mut self.control_points {
                let point_id = point.id.clone();
                for (edge_id, tangent) in &mut point.branch_tangents {
                    let incident = incident_edges
                        .get(point_id.as_str())
                        .map(|edges| edges.contains(edge_id.as_str()))
                        .unwrap_or(false);
                    if !incident {
                        return Err(format!(
                            "control point '{point_id}' branch tangent references non-incident edge '{edge_id}'"
                        ));
                    }
                    if !tangent[0].is_finite() || !tangent[1].is_finite() {
                        return Err(format!(
                            "control point '{point_id}' branch tangent for edge '{edge_id}' must be finite"
                        ));
                    }
                    let length = tangent[0].hypot(tangent[1]);
                    if length <= 1e-6 {
                        return Err(format!(
                            "control point '{point_id}' branch tangent for edge '{edge_id}' must be non-zero"
                        ));
                    }
                    tangent[0] /= length;
                    tangent[1] /= length;
                }
            }
        }

        for edge in &mut self.edges {
            for point in &edge.points {
                if !point[0].is_finite() || !point[1].is_finite() {
                    return Err(format!(
                        "edge '{}' has a non-finite interior point",
                        edge.id
                    ));
                }
            }
            for (label, tangent) in [
                ("from", &mut edge.from_tangent),
                ("to", &mut edge.to_tangent),
            ] {
                let Some(tangent) = tangent else {
                    continue;
                };
                if !tangent[0].is_finite() || !tangent[1].is_finite() {
                    return Err(format!("edge '{}' {label} tangent must be finite", edge.id));
                }
                let length = tangent[0].hypot(tangent[1]);
                if length <= 1e-6 {
                    return Err(format!(
                        "edge '{}' {label} tangent must be non-zero",
                        edge.id
                    ));
                }
                tangent[0] /= length;
                tangent[1] /= length;
            }
        }

        let control_points = self
            .control_points
            .iter()
            .map(|point| (point.id.as_str(), point))
            .collect::<HashMap<_, _>>();
        let edges = self
            .edges
            .iter()
            .map(|edge| (edge.id.as_str(), edge))
            .collect::<HashMap<_, _>>();

        for edge in &self.edges {
            if edge.id.trim().is_empty() {
                return Err("edge ids must not be empty".to_string());
            }
            if edge.from == edge.to {
                return Err(format!("edge '{}' must not self-loop", edge.id));
            }
            for endpoint in [&edge.from, &edge.to] {
                if !control_points.contains_key(endpoint.as_str()) {
                    return Err(format!(
                        "edge '{}' references unknown control point '{endpoint}'",
                        edge.id
                    ));
                }
            }
            if let Some(profile) = &edge.start_profile {
                validate_profile(
                    profile,
                    &self.settings.splines,
                    &format!("edge '{}' start profile", edge.id),
                )?;
            }
            if let Some(profile) = &edge.end_profile {
                validate_profile(
                    profile,
                    &self.settings.splines,
                    &format!("edge '{}' end profile", edge.id),
                )?;
            }
            for span in &edge.kerbs {
                validate_span(
                    span.start,
                    span.end,
                    &format!("edge '{}' kerb span", edge.id),
                )?;
            }
            for span in &edge.prop_spans {
                validate_span(
                    span.start,
                    span.end,
                    &format!("edge '{}' prop span", edge.id),
                )?;
                let spline = self
                    .settings
                    .splines
                    .iter()
                    .find(|spline| spline.id == span.spline)
                    .ok_or_else(|| {
                        format!(
                            "edge '{}' prop span references unknown spline '{}'",
                            edge.id, span.spline
                        )
                    })?;
                if spline.role != SplineRole::Prop {
                    return Err(format!(
                        "edge '{}' prop span references non-prop spline '{}'",
                        edge.id, span.spline
                    ));
                }
            }
        }

        for point in &self.control_points {
            if let Some(profile) = &point.profile {
                validate_profile(
                    profile,
                    &self.settings.splines,
                    &format!("control point '{}' profile", point.id),
                )?;
            }
        }

        if self.layouts.is_empty() {
            return Err("track project needs at least one layout".to_string());
        }
        let require_layout_start_finish = self.schema == TRACK_PROJECT_SCHEMA_V4;
        for layout in &self.layouts {
            if layout.id.trim().is_empty() {
                return Err("layout ids must not be empty".to_string());
            }
            if layout.route.is_empty() {
                return Err(format!("layout '{}' route must not be empty", layout.id));
            }
            let mut seen = BTreeSet::new();
            for (index, edge_id) in layout.route.iter().enumerate() {
                let edge = edges.get(edge_id.as_str()).ok_or_else(|| {
                    format!(
                        "layout '{}' route references unknown edge '{edge_id}'",
                        layout.id
                    )
                })?;
                if !seen.insert(edge_id.as_str()) {
                    return Err(format!(
                        "layout '{}' route repeats edge '{edge_id}'",
                        layout.id
                    ));
                }
                let next_id = &layout.route[(index + 1) % layout.route.len()];
                let next = edges.get(next_id.as_str()).ok_or_else(|| {
                    format!(
                        "layout '{}' route references unknown edge '{next_id}'",
                        layout.id
                    )
                })?;
                if edge.to != next.from {
                    return Err(format!(
                        "layout '{}' route is not a closed canonical chain: edge '{}' ends at '{}' but next edge '{}' starts at '{}'",
                        layout.id, edge.id, edge.to, next.id, next.from
                    ));
                }
            }
            if require_layout_start_finish {
                if !layout.start_finish.fraction.is_finite()
                    || layout.start_finish.fraction < 0.0
                    || layout.start_finish.fraction >= 1.0
                {
                    return Err(format!(
                        "layout '{}' start/finish fraction must lie in [0, 1)",
                        layout.id
                    ));
                }
                let start_edge = edges
                    .get(layout.start_finish.edge.as_str())
                    .ok_or_else(|| {
                        format!(
                            "layout '{}' start/finish references unknown edge '{}'",
                            layout.id, layout.start_finish.edge
                        )
                    })?;
                if !layout.route.iter().any(|edge| edge == &start_edge.id) {
                    return Err(format!(
                        "layout '{}' start/finish edge '{}' is missing from its route",
                        layout.id, start_edge.id
                    ));
                }
            }
        }

        if self.schema != TRACK_PROJECT_SCHEMA_V4 {
            let start_edge = edges.get(self.start_finish.edge.as_str()).ok_or_else(|| {
                format!(
                    "start/finish references unknown edge '{}'",
                    self.start_finish.edge
                )
            })?;
            for layout in &self.layouts {
                if !layout.route.iter().any(|edge| edge == &start_edge.id) {
                    return Err(format!(
                        "start/finish edge '{}' is missing from layout '{}'",
                        start_edge.id, layout.id
                    ));
                }
            }
            // Legacy `@2` projects only carry the global start/finish, so mirror
            // it onto every route before the route is assembled.
            let start_finish = self.start_finish.clone();
            for layout in &mut self.layouts {
                layout.start_finish = start_finish.clone();
            }
        }

        let mut degrees = BTreeMap::<&str, usize>::new();
        for edge in &self.edges {
            *degrees.entry(edge.from.as_str()).or_default() += 1;
            *degrees.entry(edge.to.as_str()).or_default() += 1;
        }
        for point in &self.control_points {
            let degree = degrees.get(point.id.as_str()).copied().unwrap_or(0);
            match point.kind {
                ControlPointKind::Normal => {
                    // Degree 0 is an unreferenced draft junction created while
                    // authoring a branch; it is valid until it is connected or
                    // deleted.
                    if degree != 0 && degree != 2 {
                        return Err(format!(
                            "control point '{}' is normal but has degree {degree}; expected 0 or 2",
                            point.id
                        ));
                    }
                }
                ControlPointKind::StartFinish => {
                    if degree != 2 {
                        return Err(format!(
                            "start/finish control point '{}' has degree {degree}; expected 2",
                            point.id
                        ));
                    }
                }
                ControlPointKind::Fork | ControlPointKind::PitEntry | ControlPointKind::PitExit => {
                    if degree < 3 {
                        return Err(format!(
                            "control point '{}' is a fork/pit node but has degree {degree}; expected at least 3",
                            point.id
                        ));
                    }
                }
            }
        }

        if let Some(pit_lane) = &self.pit_lane {
            if pit_lane.route.is_empty() {
                return Err(format!(
                    "pit lane '{}' route must not be empty",
                    pit_lane.id
                ));
            }
            let mut seen = BTreeSet::new();
            for (index, edge_id) in pit_lane.route.iter().enumerate() {
                let edge = edges.get(edge_id.as_str()).ok_or_else(|| {
                    format!(
                        "pit lane '{}' route references unknown edge '{edge_id}'",
                        pit_lane.id
                    )
                })?;
                if !seen.insert(edge_id.as_str()) {
                    return Err(format!(
                        "pit lane '{}' route repeats edge '{edge_id}'",
                        pit_lane.id
                    ));
                }
                if let Some(next_id) = pit_lane.route.get(index + 1) {
                    let next = edges.get(next_id.as_str()).ok_or_else(|| {
                        format!(
                            "pit lane '{}' route references unknown edge '{next_id}'",
                            pit_lane.id
                        )
                    })?;
                    if edge.to != next.from {
                        return Err(format!(
                            "pit lane '{}' route is not an open chain: edge '{}' ends at '{}' but next edge '{}' starts at '{}'",
                            pit_lane.id, edge.id, edge.to, next.id, next.from
                        ));
                    }
                }
            }
            let first = edges
                .get(pit_lane.route[0].as_str())
                .expect("pit lane edge was validated");
            let last = edges
                .get(
                    pit_lane
                        .route
                        .last()
                        .expect("pit lane route is non-empty")
                        .as_str(),
                )
                .expect("pit lane edge was validated");
            let entry = control_points
                .get(first.from.as_str())
                .expect("pit lane control point was validated");
            let exit = control_points
                .get(last.to.as_str())
                .expect("pit lane control point was validated");
            if entry.kind != ControlPointKind::PitEntry {
                return Err(format!(
                    "pit lane '{}' must start at a pit_entry control point, but '{}' is {:?}",
                    pit_lane.id, entry.id, entry.kind
                ));
            }
            if exit.kind != ControlPointKind::PitExit {
                return Err(format!(
                    "pit lane '{}' must end at a pit_exit control point, but '{}' is {:?}",
                    pit_lane.id, exit.id, exit.kind
                ));
            }
        }

        for path in &self.paths {
            if path.id.trim().is_empty() {
                return Err("path ids must not be empty".to_string());
            }
            if path.kind.trim().is_empty() {
                return Err(format!("path '{}' kind must not be empty", path.id));
            }
            let mut previous: Option<&EdgeProject> = None;
            for edge_id in &path.route {
                let edge = edges.get(edge_id.as_str()).ok_or_else(|| {
                    format!(
                        "path '{}' route references unknown edge '{edge_id}'",
                        path.id
                    )
                })?;
                if edge.from == edge.to {
                    return Err(format!(
                        "path '{}' route must not reference self-loop edge '{}'",
                        path.id, edge.id
                    ));
                }
                if let Some(previous) = previous {
                    if previous.to != edge.from {
                        return Err(format!(
                            "path '{}' route is not a canonical chain: edge '{}' ends at '{}' but next edge '{}' starts at '{}'",
                            path.id, previous.id, previous.to, edge.id, edge.from
                        ));
                    }
                }
                previous = Some(edge);
            }
        }

        Ok(())
    }

    pub fn layout_by_id(&self, id: &str) -> Option<&LayoutProject> {
        self.layouts.iter().find(|layout| layout.id == id)
    }

    pub fn layout_index(&self, id: &str) -> Option<usize> {
        self.layouts.iter().position(|layout| layout.id == id)
    }

    pub fn default_layout_id(&self) -> &str {
        self.layouts
            .first()
            .map(|layout| layout.id.as_str())
            .unwrap_or_default()
    }

    fn control_point_by_id(&self, id: &str) -> Option<&ControlPoint> {
        self.control_points.iter().find(|point| point.id == id)
    }

    fn edge_by_id(&self, id: &str) -> Option<&EdgeProject> {
        self.edges.iter().find(|edge| edge.id == id)
    }

    fn unique_layout_id(&self, base: &str) -> String {
        unique_id(self.layouts.iter().map(|layout| layout.id.as_str()), base)
    }

    fn unique_control_point_id(&self, base: &str) -> String {
        unique_id(
            self.control_points.iter().map(|point| point.id.as_str()),
            base,
        )
    }

    fn unique_edge_id(&self, base: &str) -> String {
        unique_id(self.edges.iter().map(|edge| edge.id.as_str()), base)
    }

    /// Adds a new closed layout that shares the venue start/finish edge and
    /// returns its id. The rest of the route is built from new control points
    /// and edges; shared sections beyond start/finish are deferred to B2.
    pub fn add_layout(&mut self, name: &str) -> Result<String, String> {
        self.normalize_in_place()?;
        let start_edge = self
            .edge_by_id(&self.start_finish.edge)
            .cloned()
            .ok_or_else(|| "start/finish edge is missing".to_string())?;
        let from_point = self
            .control_point_by_id(&start_edge.from)
            .cloned()
            .ok_or_else(|| "start/finish edge start point is missing".to_string())?;
        let to_point = self
            .control_point_by_id(&start_edge.to)
            .cloned()
            .ok_or_else(|| "start/finish edge end point is missing".to_string())?;

        let offset = 80.0 * (self.layouts.len() as f32 + 1.0);
        let center = [
            (from_point.position[0] + to_point.position[0]) * 0.5,
            (from_point.position[1] + to_point.position[1]) * 0.5 + offset,
        ];
        let dx = to_point.position[0] - center[0];
        let dz = to_point.position[1] - center[1];
        let radius = (dx * dx + dz * dz).sqrt().max(1.0);
        let start_angle = dz.atan2(dx);
        let arc_segments = 12usize;

        let mut stations = Vec::with_capacity(arc_segments + 1);
        stations.push((to_point.id.clone(), to_point.position));
        for index in 1..arc_segments {
            let angle = start_angle + PI * index as f32 / arc_segments as f32;
            let id = self.unique_control_point_id(&format!(
                "layout-{}-control-{}",
                self.layouts.len() + 1,
                index
            ));
            self.control_points.push(ControlPoint {
                id: id.clone(),
                position: [
                    center[0] + radius * angle.cos(),
                    center[1] + radius * angle.sin(),
                ],
                kind: ControlPointKind::Normal,
                profile: None,
                branch_tangents: BTreeMap::new(),
            });
            stations.push((
                id,
                [
                    center[0] + radius * angle.cos(),
                    center[1] + radius * angle.sin(),
                ],
            ));
        }
        stations.push((from_point.id.clone(), from_point.position));

        let mut route = vec![start_edge.id.clone()];
        for index in 0..arc_segments {
            let id =
                self.unique_edge_id(&format!("layout-{}-edge-{}", self.layouts.len() + 1, index));
            self.edges.push(EdgeProject {
                id: id.clone(),
                from: stations[index].0.clone(),
                to: stations[index + 1].0.clone(),
                start_profile: None,
                end_profile: None,
                kerbs: Vec::new(),
                prop_spans: Vec::new(),
                points: Vec::new(),
                from_tangent: None,
                to_tangent: None,
            });
            route.push(id);
        }

        for point_id in [from_point.id.as_str(), to_point.id.as_str()] {
            if let Some(point) = self
                .control_points
                .iter_mut()
                .find(|point| point.id == point_id)
            {
                if point.kind == ControlPointKind::Normal
                    || point.kind == ControlPointKind::StartFinish
                {
                    point.kind = ControlPointKind::Fork;
                }
            }
        }

        let id = self.unique_layout_id(&format!("layout-{}", self.layouts.len() + 1));
        let layout_name = if name.trim().is_empty() {
            format!("Layout {}", self.layouts.len() + 1)
        } else {
            name.trim().to_string()
        };
        let start_finish = StartFinish {
            edge: start_edge.id.clone(),
            fraction: self.start_finish.fraction,
        };
        self.layouts.push(LayoutProject {
            id: id.clone(),
            name: layout_name,
            route,
            sectors: Vec::new(),
            cameras: Vec::new(),
            spawns: Vec::new(),
            start_finish,
        });
        Ok(id)
    }

    /// Clones a layout, sharing its route and canonical edges with the source.
    /// No new control points or edges are created.
    pub fn duplicate_layout(&mut self, id: &str) -> Result<String, String> {
        let source = self
            .layout_by_id(id)
            .cloned()
            .ok_or_else(|| format!("unknown layout id '{id}'"))?;
        let new_id = self.unique_layout_id(&format!("{}-copy", source.id));
        let mut duplicate = source.clone();
        duplicate.id = new_id.clone();
        duplicate.name = format!("{} Copy", source.name);
        self.layouts.push(duplicate);
        self.normalize_in_place()?;
        Ok(new_id)
    }

    /// Removes a layout. The last remaining layout cannot be deleted.
    pub fn delete_layout(&mut self, id: &str) -> Result<(), String> {
        if self.layouts.len() <= 1 {
            return Err("cannot delete the last layout".to_string());
        }
        let index = self
            .layout_index(id)
            .ok_or_else(|| format!("unknown layout id '{id}'"))?;
        self.layouts.remove(index);

        let mut used_edges = BTreeSet::new();
        for layout in &self.layouts {
            for edge in &layout.route {
                used_edges.insert(edge.clone());
            }
        }
        if let Some(pit_lane) = &self.pit_lane {
            for edge in &pit_lane.route {
                used_edges.insert(edge.clone());
            }
        }
        used_edges.insert(self.start_finish.edge.clone());
        self.edges.retain(|edge| used_edges.contains(&edge.id));

        let mut used_points = BTreeSet::new();
        for edge in &self.edges {
            used_points.insert(edge.from.clone());
            used_points.insert(edge.to.clone());
        }
        self.control_points
            .retain(|point| used_points.contains(&point.id));

        if !self
            .edges
            .iter()
            .any(|edge| edge.id == self.start_finish.edge)
        {
            self.start_finish.edge = self
                .layouts
                .first()
                .and_then(|layout| layout.route.first().cloned())
                .unwrap_or_default();
            self.start_finish.fraction = 0.0;
        }

        let mut degrees = BTreeMap::<String, usize>::new();
        for edge in &self.edges {
            *degrees.entry(edge.from.clone()).or_default() += 1;
            *degrees.entry(edge.to.clone()).or_default() += 1;
        }
        for point in &mut self.control_points {
            if point.kind == ControlPointKind::Fork
                && degrees.get(&point.id).copied().unwrap_or(0) < 3
            {
                point.kind = ControlPointKind::Normal;
            }
        }

        self.normalize_in_place()
    }

    /// Updates the global asphalt profile symmetrically for every segment. The
    /// argument `id` identifies the layout whose width control is being driven;
    /// the venue has one shared surface profile in the graph model.
    pub fn set_layout_width(&mut self, id: &str, width: f32) -> Result<(), String> {
        if self.layout_by_id(id).is_none() {
            return Err(format!("unknown layout id '{id}'"));
        }
        let width = if width.is_finite() {
            width.clamp(MIN_LAYOUT_WIDTH, MAX_LAYOUT_WIDTH)
        } else {
            DEFAULT_LAYOUT_WIDTH
        };
        self.apply_global_width(width);
        self.normalize_in_place()
    }

    fn apply_global_width(&mut self, width: f32) {
        let half = width * 0.5;
        for spline in &mut self.settings.splines {
            if spline.role != SplineRole::Surface {
                continue;
            }
            match spline.id.as_str() {
                "asphalt-left" => spline.offset = -half,
                "asphalt-right" => spline.offset = half,
                "runoff-left" => spline.offset = -width,
                "runoff-right" => spline.offset = width,
                _ => {}
            }
        }
        for point in &mut self.control_points {
            if let Some(profile) = &mut point.profile {
                apply_width_to_profile(profile, width);
            }
        }
        for edge in &mut self.edges {
            if let Some(profile) = &mut edge.start_profile {
                apply_width_to_profile(profile, width);
            }
            if let Some(profile) = &mut edge.end_profile {
                apply_width_to_profile(profile, width);
            }
        }
    }

    pub fn rename_layout(&mut self, id: &str, name: &str) -> Result<(), String> {
        let layout = self
            .layouts
            .iter_mut()
            .find(|layout| layout.id == id)
            .ok_or_else(|| format!("unknown layout id '{id}'"))?;
        layout.name = name.to_string();
        self.normalize_in_place()
    }

    /// Moves the control points of the active layout ring. Shared control
    /// points are moved too; B1 edits one ring at a time.
    pub fn set_layout_control_points(
        &mut self,
        id: &str,
        points: &[[f32; 2]],
    ) -> Result<(), String> {
        self.normalize_in_place()?;
        let route = self
            .layout_by_id(id)
            .ok_or_else(|| format!("unknown layout id '{id}'"))?
            .route
            .clone();
        if points.len() < route.len() {
            return Err(format!(
                "layout '{id}' needs at least {} control points",
                route.len()
            ));
        }
        for (index, edge_id) in route.iter().enumerate() {
            let from = self
                .edge_by_id(edge_id)
                .ok_or_else(|| format!("unknown edge id '{edge_id}'"))?
                .from
                .clone();
            if let Some(point) = self
                .control_points
                .iter_mut()
                .find(|point| point.id == from)
            {
                point.position = points[index];
            }
        }
        self.normalize_in_place()
    }

    /// Stores legacy kerb spans across every edge of the active layout route as
    /// edge-local fractions. Input spans use route sample indices.
    pub fn set_layout_kerb_spans(&mut self, id: &str, spans: &[KerbSpan]) -> Result<(), String> {
        self.normalize_in_place()?;
        let layout = self
            .layout_by_id(id)
            .ok_or_else(|| format!("unknown layout id '{id}'"))?
            .clone();
        let count = layout_centerline(self, &layout)?.len().max(1) as f32;
        let (ranges, route_len) = layout_route_ranges(self, &layout)?;
        let last = route_len.saturating_sub(1).max(1) as f32;
        let edge_ids = layout.route.iter().cloned().collect::<BTreeSet<_>>();
        let mut spans_by_edge = HashMap::<String, Vec<KerbSpan>>::new();
        for span in spans {
            let route_start = (span.start / count * last).clamp(0.0, last);
            let route_end = (span.end / count * last).clamp(0.0, last);
            if route_end - route_start <= 1e-4 {
                continue;
            }
            for (edge_id, range_start, range_end) in &ranges {
                let start = (*range_start as f32).max(route_start);
                let end = (*range_end as f32).min(route_end);
                let range_len = *range_end as f32 - *range_start as f32;
                if range_len <= 1e-6 || end - start <= 1e-4 {
                    continue;
                }
                spans_by_edge
                    .entry(edge_id.clone())
                    .or_default()
                    .push(KerbSpan {
                        start: ((start - *range_start as f32) / range_len).clamp(0.0, 1.0),
                        end: ((end - *range_start as f32) / range_len).clamp(0.0, 1.0),
                        side: span.side,
                    });
            }
        }
        for edge in &mut self.edges {
            if edge_ids.contains(&edge.id) {
                edge.kerbs = spans_by_edge.remove(&edge.id).unwrap_or_default();
            }
        }
        self.normalize_in_place()
    }

    /// Stores legacy prop lines across every edge of the active layout route as
    /// prop spans when a prop spline exists. Input lines use route sample
    /// indices.
    pub fn set_layout_prop_lines(&mut self, id: &str, lines: &[PropLine]) -> Result<(), String> {
        self.normalize_in_place()?;
        let layout = self
            .layout_by_id(id)
            .ok_or_else(|| format!("unknown layout id '{id}'"))?
            .clone();
        let width = global_asphalt_width(self);
        let count = layout_centerline(self, &layout)?.len().max(1) as f32;
        let (ranges, route_len) = layout_route_ranges(self, &layout)?;
        let last = route_len.saturating_sub(1).max(1) as f32;
        let edge_ids = layout.route.iter().cloned().collect::<BTreeSet<_>>();
        let mut spans_by_edge = HashMap::<String, Vec<PropSpan>>::new();
        for line in lines {
            let Some(spline) = select_prop_spline_id(&self.settings.splines, line, width) else {
                continue;
            };
            let route_start = (line.start / count * last).clamp(0.0, last);
            let route_end = (line.end / count * last).clamp(0.0, last);
            if route_end - route_start <= 1e-4 {
                continue;
            }
            for (edge_id, range_start, range_end) in &ranges {
                let start = (*range_start as f32).max(route_start);
                let end = (*range_end as f32).min(route_end);
                let range_len = *range_end as f32 - *range_start as f32;
                if range_len <= 1e-6 || end - start <= 1e-4 {
                    continue;
                }
                spans_by_edge
                    .entry(edge_id.clone())
                    .or_default()
                    .push(PropSpan {
                        spline: spline.clone(),
                        start: ((start - *range_start as f32) / range_len).clamp(0.0, 1.0),
                        end: ((end - *range_start as f32) / range_len).clamp(0.0, 1.0),
                        kind: line.kind,
                    });
            }
        }
        for edge in &mut self.edges {
            if edge_ids.contains(&edge.id) {
                edge.prop_spans = spans_by_edge.remove(&edge.id).unwrap_or_default();
            }
        }
        self.normalize_in_place()
    }
}

impl TrackProject {
    /// Converts the internal venue graph into the public `@3` editor document.
    pub fn to_editor_project(&self) -> Result<EditorProject, String> {
        let width = global_asphalt_width(self);
        let mut control_points = self
            .control_points
            .iter()
            .map(|point| EditorControlPoint {
                id: point.id.clone(),
                position: point.position,
                kind: point.kind,
                tangent: None,
            })
            .collect::<Vec<_>>();
        if let Some(start) = self.edge_by_id(&self.start_finish.edge) {
            if let Some(point) = control_points
                .iter_mut()
                .find(|point| point.id == start.from)
            {
                point.kind = ControlPointKind::StartFinish;
            }
        }

        let mut loops = Vec::with_capacity(self.layouts.len());
        for layout in &self.layouts {
            let mut points = Vec::with_capacity(layout.route.len());
            for edge_id in &layout.route {
                let edge = self.edge_by_id(edge_id).ok_or_else(|| {
                    format!(
                        "layout '{}' route references unknown edge '{edge_id}'",
                        layout.id
                    )
                })?;
                points.push(edge.from.clone());
            }
            loops.push(EditorLoop {
                id: layout.id.clone(),
                name: layout.name.clone(),
                control_points: points,
                sectors: layout.sectors.clone(),
                cameras: layout.cameras.clone(),
                spawns: layout.spawns.clone(),
            });
        }

        let mut segments = Vec::<EditorSegment>::new();
        let mut segment_index = BTreeMap::<(String, String), usize>::new();
        for edge in &self.edges {
            let key = (edge.from.clone(), edge.to.clone());
            if let Some(index) = segment_index.get(&key).copied() {
                let segment = &mut segments[index];
                segment.kerbs.extend(edge.kerbs.iter().copied());
                segment.prop_spans.extend(edge.prop_spans.iter().cloned());
            } else {
                segment_index.insert(key, segments.len());
                segments.push(EditorSegment {
                    from: edge.from.clone(),
                    to: edge.to.clone(),
                    role: EditorSegmentRole::Main,
                    kerbs: edge.kerbs.clone(),
                    prop_spans: edge.prop_spans.clone(),
                });
            }
        }

        let pit_lane = self.pit_lane.as_ref().map(|pit| {
            let mut control_points = Vec::new();
            for edge_id in &pit.route {
                let Some(edge) = self.edge_by_id(edge_id) else {
                    continue;
                };
                if control_points.is_empty() {
                    control_points.push(edge.from.clone());
                }
                control_points.push(edge.to.clone());
            }
            EditorPitLane {
                control_points,
                boxes: pit.boxes.clone(),
            }
        });

        EditorProject {
            schema: EDITOR_PROJECT_SCHEMA.to_string(),
            name: self.name.clone(),
            settings: EditorSettings {
                width,
                kerb: self.settings.kerb,
                pit: Some(self.settings.pit),
            },
            loops,
            control_points,
            segments,
            pit_lane,
        }
        .normalize()
    }
}

/// Route/segment/junction editing operations for the `@4` editor. These use
/// the graph's canonical names: a segment is an [`EdgeProject`], a junction is
/// a [`ControlPoint`], and a route is a [`LayoutProject`].
#[allow(dead_code)]
impl TrackProject {
    pub fn segment_by_id(&self, id: &str) -> Option<&EdgeProject> {
        self.edges.iter().find(|segment| segment.id == id)
    }

    pub fn segment_index(&self, id: &str) -> Option<usize> {
        self.edges.iter().position(|segment| segment.id == id)
    }

    pub fn junction_by_id(&self, id: &str) -> Option<&ControlPoint> {
        self.control_points
            .iter()
            .find(|junction| junction.id == id)
    }

    pub fn junction_index(&self, id: &str) -> Option<usize> {
        self.control_points
            .iter()
            .position(|junction| junction.id == id)
    }

    pub fn route_segments(&self, route_id: &str) -> Option<&[String]> {
        self.layout_by_id(route_id)
            .map(|route| route.route.as_slice())
    }

    /// Moves a junction's control point. Incident segments follow because they
    /// sample from the junction position.
    pub fn move_junction(&mut self, id: &str, x: f32, z: f32) -> Result<(), String> {
        let index = self
            .junction_index(id)
            .ok_or_else(|| format!("unknown junction id '{id}'"))?;
        if !x.is_finite() || !z.is_finite() {
            return Err(format!("junction '{id}' position must be finite"));
        }
        let mut candidate = self.clone();
        candidate.control_points[index].position = [x, z];
        *self = candidate.normalize()?;
        Ok(())
    }

    /// Inserts a detached junction and returns its id. A detached junction is
    /// only valid once an incident segment references it; connect or delete it
    /// before normalizing the project.
    pub fn insert_junction(&mut self, x: f32, z: f32) -> String {
        let id = unique_id(
            self.control_points.iter().map(|point| point.id.as_str()),
            "junction",
        );
        let position = [
            if x.is_finite() { x } else { 0.0 },
            if z.is_finite() { z } else { 0.0 },
        ];
        self.control_points.push(ControlPoint {
            id: id.clone(),
            position,
            kind: ControlPointKind::Normal,
            profile: None,
            branch_tangents: BTreeMap::new(),
        });
        id
    }

    /// Deletes an unreferenced junction. Referenced junctions must be rewired
    /// or their segments removed first.
    pub fn delete_junction(&mut self, id: &str) -> Result<(), String> {
        let index = self
            .junction_index(id)
            .ok_or_else(|| format!("unknown junction id '{id}'"))?;
        let referenced = self
            .edges
            .iter()
            .filter(|segment| segment.from == id || segment.to == id)
            .map(|segment| segment.id.as_str())
            .collect::<Vec<_>>();
        if !referenced.is_empty() {
            return Err(format!(
                "junction '{id}' is referenced by segment(s) {}; rewire or delete them first",
                referenced.join(", ")
            ));
        }
        let mut candidate = self.clone();
        candidate.control_points.remove(index);
        *self = candidate.normalize()?;
        Ok(())
    }

    /// Inserts an interior control point after `after_index` and returns the
    /// inserted index. Indices are 0-based into the segment's interior points
    /// vector; an empty vector accepts `after_index == 0`.
    pub fn insert_segment_point(
        &mut self,
        segment_id: &str,
        after_index: usize,
        x: f32,
        z: f32,
    ) -> Result<usize, String> {
        let index = self
            .segment_index(segment_id)
            .ok_or_else(|| format!("unknown segment id '{segment_id}'"))?;
        if !x.is_finite() || !z.is_finite() {
            return Err(format!(
                "segment '{segment_id}' interior point must be finite"
            ));
        }
        let mut candidate = self.clone();
        let points = &mut candidate.edges[index].points;
        if after_index > points.len() {
            return Err(format!(
                "segment '{segment_id}' interior point index {after_index} is out of range for {} point(s)",
                points.len()
            ));
        }
        let insert_at = if points.is_empty() {
            0
        } else {
            (after_index + 1).min(points.len())
        };
        points.insert(insert_at, [x, z]);
        *self = candidate.normalize()?;
        Ok(insert_at)
    }

    /// Moves an interior control point at a 0-based index.
    pub fn move_segment_point(
        &mut self,
        segment_id: &str,
        index: usize,
        x: f32,
        z: f32,
    ) -> Result<(), String> {
        let segment_index = self
            .segment_index(segment_id)
            .ok_or_else(|| format!("unknown segment id '{segment_id}'"))?;
        if !x.is_finite() || !z.is_finite() {
            return Err(format!(
                "segment '{segment_id}' interior point must be finite"
            ));
        }
        let mut candidate = self.clone();
        let points = &mut candidate.edges[segment_index].points;
        if index >= points.len() {
            return Err(format!(
                "segment '{segment_id}' interior point index {index} is out of range for {} point(s)",
                points.len()
            ));
        }
        points[index] = [x, z];
        *self = candidate.normalize()?;
        Ok(())
    }

    /// Deletes an interior control point at a 0-based index.
    pub fn delete_segment_point(&mut self, segment_id: &str, index: usize) -> Result<(), String> {
        let segment_index = self
            .segment_index(segment_id)
            .ok_or_else(|| format!("unknown segment id '{segment_id}'"))?;
        let mut candidate = self.clone();
        let points = &mut candidate.edges[segment_index].points;
        if index >= points.len() {
            return Err(format!(
                "segment '{segment_id}' interior point index {index} is out of range for {} point(s)",
                points.len()
            ));
        }
        points.remove(index);
        *self = candidate.normalize()?;
        Ok(())
    }

    /// Clones a segment for one route and repoints only that route at the
    /// clone. Other routes keep the original segment.
    pub fn make_segment_unique(
        &mut self,
        route_id: &str,
        segment_id: &str,
    ) -> Result<String, String> {
        let route_index = self
            .layout_index(route_id)
            .ok_or_else(|| format!("unknown route id '{route_id}'"))?;
        let route_position = self.layouts[route_index]
            .route
            .iter()
            .position(|id| id == segment_id)
            .ok_or_else(|| {
                format!("route '{route_id}' does not reference segment '{segment_id}'")
            })?;
        let segment_index = self
            .segment_index(segment_id)
            .ok_or_else(|| format!("unknown segment id '{segment_id}'"))?;
        if self.start_finish.edge == segment_id {
            return Err(format!(
                "cannot make segment '{segment_id}' unique because it is the venue start/finish segment"
            ));
        }

        let mut candidate = self.clone();
        let mut unique = candidate.edges[segment_index].clone();
        unique.id = unique_id(
            candidate.edges.iter().map(|segment| segment.id.as_str()),
            segment_id,
        );
        let unique_id = unique.id.clone();
        let endpoints = [unique.from.clone(), unique.to.clone()];
        candidate.edges.push(unique);
        candidate.layouts[route_index].route[route_position] = unique_id.clone();
        if candidate.layouts[route_index].start_finish.edge == segment_id {
            candidate.layouts[route_index].start_finish.edge = unique_id.clone();
        }
        // A parallel segment raises the degree of both endpoints. Promote the
        // degree-two junction kinds so the cloned segment can share them.
        let mut degrees = BTreeMap::<&str, usize>::new();
        for segment in &candidate.edges {
            *degrees.entry(segment.from.as_str()).or_default() += 1;
            *degrees.entry(segment.to.as_str()).or_default() += 1;
        }
        for junction in &mut candidate.control_points {
            if endpoints.iter().any(|id| id == &junction.id)
                && matches!(
                    junction.kind,
                    ControlPointKind::Normal | ControlPointKind::StartFinish
                )
                && degrees.get(junction.id.as_str()).copied().unwrap_or(0) >= 3
            {
                junction.kind = ControlPointKind::Fork;
            }
        }
        *self = candidate.normalize()?;
        Ok(unique_id)
    }

    pub fn set_segment_kerb_spans(
        &mut self,
        segment_id: &str,
        spans: &[KerbSpan],
    ) -> Result<(), String> {
        let index = self
            .segment_index(segment_id)
            .ok_or_else(|| format!("unknown segment id '{segment_id}'"))?;
        let mut candidate = self.clone();
        candidate.edges[index].kerbs = spans.to_vec();
        *self = candidate.normalize()?;
        Ok(())
    }

    pub fn set_segment_prop_spans(
        &mut self,
        segment_id: &str,
        spans: &[PropSpan],
    ) -> Result<(), String> {
        let index = self
            .segment_index(segment_id)
            .ok_or_else(|| format!("unknown segment id '{segment_id}'"))?;
        let mut candidate = self.clone();
        candidate.edges[index].prop_spans = spans.to_vec();
        *self = candidate.normalize()?;
        Ok(())
    }

    /// Sets both endpoint tangents. `None` clears a tangent and falls back to
    /// the sampler's derived direction. Non-zero tangents are normalized by
    /// [`Self::normalize`].
    pub fn set_segment_tangents(
        &mut self,
        segment_id: &str,
        from: Option<[f32; 2]>,
        to: Option<[f32; 2]>,
    ) -> Result<(), String> {
        let index = self
            .segment_index(segment_id)
            .ok_or_else(|| format!("unknown segment id '{segment_id}'"))?;
        let mut candidate = self.clone();
        candidate.edges[index].from_tangent = from;
        candidate.edges[index].to_tangent = to;
        *self = candidate.normalize()?;
        Ok(())
    }

    pub fn set_segment_from_tangent(
        &mut self,
        segment_id: &str,
        tangent: Option<[f32; 2]>,
    ) -> Result<(), String> {
        let to = self
            .segment_by_id(segment_id)
            .ok_or_else(|| format!("unknown segment id '{segment_id}'"))?
            .to_tangent;
        self.set_segment_tangents(segment_id, tangent, to)
    }

    pub fn set_segment_to_tangent(
        &mut self,
        segment_id: &str,
        tangent: Option<[f32; 2]>,
    ) -> Result<(), String> {
        let from = self
            .segment_by_id(segment_id)
            .ok_or_else(|| format!("unknown segment id '{segment_id}'"))?
            .from_tangent;
        self.set_segment_tangents(segment_id, from, tangent)
    }

    /// Samples one segment through the existing edge sampler.
    pub fn segment_centerline(&self, segment_id: &str) -> Result<Vec<[f32; 3]>, String> {
        let segment = self
            .segment_by_id(segment_id)
            .ok_or_else(|| format!("unknown segment id '{segment_id}'"))?;
        let points = sample_edge_points(self, segment)?;
        Ok(points.into_iter().map(|point| point.to_array()).collect())
    }

    /// Assembles a route's centreline from its canonical segments.
    pub fn route_centerline(&self, route_id: &str) -> Result<Vec<[f32; 3]>, String> {
        let route = self
            .layout_by_id(route_id)
            .ok_or_else(|| format!("unknown route id '{route_id}'"))?;
        let segment_ids = route.route.iter().cloned().collect::<BTreeSet<_>>();
        let built = build_unique_edges(self, &segment_ids)?;
        let assembled = assemble_route(self, route, &built, &HashMap::new())?;
        Ok(assembled
            .points
            .into_iter()
            .map(|point| point.to_array())
            .collect())
    }
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            width: DEFAULT_LAYOUT_WIDTH,
            kerb: VenueSettings::default().kerb,
            pit: None,
        }
    }
}

#[allow(dead_code)]
impl EditorProject {
    /// The built-in single-loop venue; matches the original editor loop.
    pub fn default_project() -> Self {
        let control_points = CONTROL_POINTS
            .iter()
            .enumerate()
            .map(|(index, point)| EditorControlPoint {
                id: format!("cp-{index}"),
                position: [point[0], point[2]],
                kind: if index == 0 {
                    ControlPointKind::StartFinish
                } else {
                    ControlPointKind::Normal
                },
                tangent: None,
            })
            .collect::<Vec<_>>();
        let control_point_ids = control_points
            .iter()
            .map(|point| point.id.clone())
            .collect::<Vec<_>>();
        Self {
            schema: EDITOR_PROJECT_SCHEMA.to_string(),
            name: "Procedural Drift Loop".to_string(),
            settings: EditorSettings::default(),
            loops: vec![EditorLoop {
                id: "procedural-drift-loop".to_string(),
                name: "Procedural Drift Loop".to_string(),
                control_points: control_point_ids,
                sectors: Vec::new(),
                cameras: Vec::new(),
                spawns: Vec::new(),
            }],
            control_points,
            segments: Vec::new(),
            pit_lane: None,
        }
    }

    /// Parses an `@3` project, or migrates an `@2`/`@1` project into `@3`.
    pub fn from_json(json: &str) -> Result<Self, String> {
        let value = serde_json::from_str::<serde_json::Value>(json)
            .map_err(|error| format!("invalid track project JSON: {error}"))?;
        let schema = value
            .get("schema")
            .and_then(|schema| schema.as_str())
            .unwrap_or_default()
            .to_string();
        match schema.as_str() {
            EDITOR_PROJECT_SCHEMA => serde_json::from_value::<EditorProject>(value)
                .map_err(|error| format!("invalid editor project: {error}"))?
                .normalize(),
            TRACK_PROJECT_SCHEMA | TRACK_PROJECT_SCHEMA_V1 => {
                let project = TrackProject::from_json(json)?;
                project.to_editor_project()?.normalize()
            }
            other => Err(format!(
                "unsupported track project schema '{other}'; expected {EDITOR_PROJECT_SCHEMA}, {TRACK_PROJECT_SCHEMA}, or {TRACK_PROJECT_SCHEMA_V1}"
            )),
        }
    }

    pub fn normalize(mut self) -> Result<Self, String> {
        self.normalize_in_place()?;
        Ok(self)
    }

    fn normalize_in_place(&mut self) -> Result<(), String> {
        if self.schema != EDITOR_PROJECT_SCHEMA {
            return Err(format!(
                "unsupported editor project schema '{}'; expected {EDITOR_PROJECT_SCHEMA}",
                self.schema
            ));
        }
        self.name = self.name.trim().to_string();
        self.settings.width = clamp_layout_width(self.settings.width);
        validate_unique_ids(
            self.control_points.iter().map(|point| point.id.as_str()),
            "control point",
        )?;
        validate_unique_ids(self.loops.iter().map(|loop_| loop_.id.as_str()), "loop")?;

        if self.loops.is_empty() {
            return Err("track project needs at least one loop".to_string());
        }

        let mut positions = HashMap::<String, [f32; 2]>::new();
        for point in &self.control_points {
            if point.id.trim().is_empty() {
                return Err("control point ids must not be empty".to_string());
            }
            if !point.position[0].is_finite() || !point.position[1].is_finite() {
                return Err(format!(
                    "control point '{}' has a non-finite position",
                    point.id
                ));
            }
            positions.insert(point.id.clone(), point.position);
        }

        for loop_ in &self.loops {
            if loop_.id.trim().is_empty() {
                return Err("loop ids must not be empty".to_string());
            }
            if loop_.control_points.len() < 3 {
                return Err(format!(
                    "loop '{}' needs at least three control points",
                    loop_.id
                ));
            }
            let mut seen = BTreeSet::new();
            for (index, point_id) in loop_.control_points.iter().enumerate() {
                if !positions.contains_key(point_id) {
                    return Err(format!(
                        "loop '{}' references unknown control point '{point_id}'",
                        loop_.id
                    ));
                }
                if !seen.insert(point_id.as_str()) {
                    return Err(format!(
                        "loop '{}' repeats control point '{point_id}'",
                        loop_.id
                    ));
                }
                let next_id = &loop_.control_points[(index + 1) % loop_.control_points.len()];
                if point_id == next_id {
                    return Err(format!(
                        "loop '{}' has consecutive duplicate control point '{point_id}'",
                        loop_.id
                    ));
                }
            }
        }

        if !self
            .control_points
            .iter()
            .any(|point| point.kind == ControlPointKind::StartFinish)
        {
            return Err("track project needs at least one start_finish control point".to_string());
        }

        let mut segment_pairs = BTreeSet::<(String, String)>::new();
        for segment in &self.segments {
            if !positions.contains_key(&segment.from) {
                return Err(format!(
                    "segment references unknown control point '{}'",
                    segment.from
                ));
            }
            if !positions.contains_key(&segment.to) {
                return Err(format!(
                    "segment references unknown control point '{}'",
                    segment.to
                ));
            }
            if segment.from == segment.to {
                return Err(format!(
                    "segment '{0}->{1}' must not self-loop",
                    segment.from, segment.to
                ));
            }
            let key = (segment.from.clone(), segment.to.clone());
            if !segment_pairs.insert(key) {
                return Err(format!(
                    "duplicate segment pair '{}->{}'",
                    segment.from, segment.to
                ));
            }
            for span in &segment.kerbs {
                validate_span(
                    span.start,
                    span.end,
                    &format!("segment '{}->{}' kerb span", segment.from, segment.to),
                )?;
            }
            for span in &segment.prop_spans {
                validate_span(
                    span.start,
                    span.end,
                    &format!("segment '{}->{}' prop span", segment.from, segment.to),
                )?;
            }
        }

        let mut additions = Vec::new();
        for loop_ in &self.loops {
            for index in 0..loop_.control_points.len() {
                let from = loop_.control_points[index].clone();
                let to = loop_.control_points[(index + 1) % loop_.control_points.len()].clone();
                let key = (from.clone(), to.clone());
                if segment_pairs.insert(key) {
                    additions.push(EditorSegment {
                        from,
                        to,
                        role: EditorSegmentRole::Main,
                        kerbs: Vec::new(),
                        prop_spans: Vec::new(),
                    });
                }
            }
        }
        self.segments.extend(additions);

        if let Some(pit_lane) = &self.pit_lane {
            if pit_lane.control_points.len() < 2 {
                return Err("pit lane needs at least two control points".to_string());
            }
            for point_id in &pit_lane.control_points {
                if !positions.contains_key(point_id) {
                    return Err(format!(
                        "pit lane references unknown control point '{point_id}'"
                    ));
                }
            }
        }

        Ok(())
    }

    pub fn loop_by_id(&self, id: &str) -> Option<&EditorLoop> {
        self.loops.iter().find(|loop_| loop_.id == id)
    }

    pub fn loop_index(&self, id: &str) -> Option<usize> {
        self.loops.iter().position(|loop_| loop_.id == id)
    }

    pub fn default_loop_id(&self) -> &str {
        self.loops
            .first()
            .map(|loop_| loop_.id.as_str())
            .unwrap_or_default()
    }

    fn control_point_by_id(&self, id: &str) -> Option<&EditorControlPoint> {
        self.control_points.iter().find(|point| point.id == id)
    }

    fn unique_loop_id(&self, base: &str) -> String {
        unique_id(self.loops.iter().map(|loop_| loop_.id.as_str()), base)
    }

    fn unique_control_point_id(&self, base: &str) -> String {
        unique_id(
            self.control_points.iter().map(|point| point.id.as_str()),
            base,
        )
    }

    /// Adds a loop with its own new ring of control points.
    pub fn add_loop(&mut self, name: &str) -> Result<String, String> {
        self.normalize_in_place()?;
        let index = self.loops.len() + 1;
        let offset = 80.0 * index as f32;
        let mut control_points = Vec::with_capacity(CONTROL_POINTS.len());
        for (point_index, point) in CONTROL_POINTS.iter().enumerate() {
            let id = self.unique_control_point_id(&format!("loop-{index}-cp-{point_index}"));
            self.control_points.push(EditorControlPoint {
                id: id.clone(),
                position: [point[0], point[2] + offset],
                kind: if point_index == 0 {
                    ControlPointKind::StartFinish
                } else {
                    ControlPointKind::Normal
                },
                tangent: None,
            });
            control_points.push(id);
        }
        let id = self.unique_loop_id(&format!("loop-{index}"));
        let loop_name = if name.trim().is_empty() {
            format!("Loop {index}")
        } else {
            name.trim().to_string()
        };
        self.loops.push(EditorLoop {
            id: id.clone(),
            name: loop_name,
            control_points,
            sectors: Vec::new(),
            cameras: Vec::new(),
            spawns: Vec::new(),
        });
        self.normalize_in_place()?;
        Ok(id)
    }

    /// Duplicates a loop, sharing all of its control-point ids.
    pub fn duplicate_loop(&mut self, id: &str) -> Result<String, String> {
        self.normalize_in_place()?;
        let source = self
            .loop_by_id(id)
            .cloned()
            .ok_or_else(|| format!("unknown loop id '{id}'"))?;
        let new_id = self.unique_loop_id(&format!("{}-copy", source.id));
        let mut duplicate = source;
        duplicate.id = new_id.clone();
        duplicate.name = format!("{} Copy", duplicate.name);
        self.loops.push(duplicate);
        self.normalize_in_place()?;
        Ok(new_id)
    }

    /// Removes a loop, pruning control points and segments that no longer
    /// belong to any loop. The last loop cannot be deleted.
    pub fn delete_loop(&mut self, id: &str) -> Result<(), String> {
        self.normalize_in_place()?;
        if self.loops.len() <= 1 {
            return Err("cannot delete the last loop".to_string());
        }
        let index = self
            .loop_index(id)
            .ok_or_else(|| format!("unknown loop id '{id}'"))?;
        self.loops.remove(index);

        let mut used_points = BTreeSet::new();
        let mut used_pairs = BTreeSet::new();
        for loop_ in &self.loops {
            for point_id in &loop_.control_points {
                used_points.insert(point_id.clone());
            }
            for point_index in 0..loop_.control_points.len() {
                used_pairs.insert((
                    loop_.control_points[point_index].clone(),
                    loop_.control_points[(point_index + 1) % loop_.control_points.len()].clone(),
                ));
            }
        }
        self.control_points
            .retain(|point| used_points.contains(&point.id));
        self.segments
            .retain(|segment| used_pairs.contains(&(segment.from.clone(), segment.to.clone())));
        if let Some(pit_lane) = &self.pit_lane {
            if pit_lane
                .control_points
                .iter()
                .any(|point_id| !used_points.contains(point_id))
            {
                self.pit_lane = None;
            }
        }
        if !self
            .control_points
            .iter()
            .any(|point| point.kind == ControlPointKind::StartFinish)
        {
            if let Some(point) = self.control_points.first_mut() {
                point.kind = ControlPointKind::StartFinish;
            }
        }
        self.normalize_in_place()
    }

    pub fn rename_loop(&mut self, id: &str, name: &str) -> Result<(), String> {
        let loop_ = self
            .loops
            .iter_mut()
            .find(|loop_| loop_.id == id)
            .ok_or_else(|| format!("unknown loop id '{id}'"))?;
        loop_.name = name.trim().to_string();
        self.normalize_in_place()
    }

    /// Sets the global venue width.
    pub fn set_width(&mut self, width: f32) -> Result<(), String> {
        self.settings.width = clamp_layout_width(width);
        self.normalize_in_place()
    }

    pub fn loop_control_points(&self, loop_: &EditorLoop) -> Result<Vec<[f32; 2]>, String> {
        let mut points = Vec::with_capacity(loop_.control_points.len());
        for point_id in &loop_.control_points {
            let point = self.control_point_by_id(point_id).ok_or_else(|| {
                format!(
                    "loop '{}' references unknown control point '{point_id}'",
                    loop_.id
                )
            })?;
            points.push(point.position);
        }
        Ok(points)
    }

    pub fn loop_centerline(&self, loop_: &EditorLoop) -> Result<Vec<[f32; 3]>, String> {
        let control_points = self.loop_control_points(loop_)?;
        let points = layout_control_points_with_heights(&control_points);
        let width = clamp_layout_width(self.settings.width);
        let samples = sample_track_with_min_radius(&points, min_turn_radius(width));
        if samples.len() < 3 {
            return Err(format!("loop '{}' needs at least three samples", loop_.id));
        }
        Ok(samples.iter().map(|point| point.to_array()).collect())
    }

    /// Returns the centerline used for the editor overlay. Single-loop
    /// projects without a pit lane keep the continuous fast path; any other
    /// project resolves the loop through the generated venue graph so the
    /// overlay matches the built geometry.
    pub fn loop_visible_centerline(&self, loop_: &EditorLoop) -> Result<Vec<[f32; 3]>, String> {
        if self.loops.len() == 1 && self.pit_lane.is_none() {
            return self.loop_centerline(loop_);
        }
        let graph = self.to_graph()?;
        let layout = graph
            .layout_by_id(&loop_.id)
            .ok_or_else(|| format!("unknown loop id '{}'", loop_.id))?;
        let edge_ids = layout.route.iter().cloned().collect::<BTreeSet<_>>();
        let built = build_unique_edges(&graph, &edge_ids)?;
        let route = assemble_route(&graph, layout, &built, &HashMap::new())?;
        Ok(route
            .points
            .into_iter()
            .map(|point| point.to_array())
            .collect())
    }

    pub fn loop_kerb_spans(&self, loop_: &EditorLoop) -> Result<Vec<KerbSpan>, String> {
        let graph = self.to_graph()?;
        let layout = graph
            .layout_by_id(&loop_.id)
            .ok_or_else(|| format!("unknown loop id '{}'", loop_.id))?;
        layout_kerb_spans(&graph, layout)
    }

    pub fn loop_prop_lines(&self, loop_: &EditorLoop) -> Result<Vec<PropLine>, String> {
        let graph = self.to_graph()?;
        let layout = graph
            .layout_by_id(&loop_.id)
            .ok_or_else(|| format!("unknown loop id '{}'", loop_.id))?;
        layout_prop_lines(&graph, layout)
    }

    pub fn set_loop_control_points(&mut self, id: &str, points: &[[f32; 2]]) -> Result<(), String> {
        self.normalize_in_place()?;
        let point_ids = self
            .loop_by_id(id)
            .ok_or_else(|| format!("unknown loop id '{id}'"))?
            .control_points
            .clone();
        if points.len() < point_ids.len() {
            return Err(format!(
                "loop '{id}' needs at least {} control points",
                point_ids.len()
            ));
        }
        for (point_id, position) in point_ids.iter().zip(points.iter()) {
            if let Some(point) = self
                .control_points
                .iter_mut()
                .find(|point| point.id == *point_id)
            {
                point.position = *position;
            }
        }
        self.normalize_in_place()
    }

    /// Lists the loop's control points with sharing metadata for the editor UI.
    pub fn loop_point_info(&self, loop_id: &str) -> Result<Vec<EditorPointInfo>, String> {
        let loop_ = self
            .loop_by_id(loop_id)
            .ok_or_else(|| format!("unknown loop id '{loop_id}'"))?;
        let mut info = Vec::with_capacity(loop_.control_points.len());
        for point_id in &loop_.control_points {
            let point = self.control_point_by_id(point_id).ok_or_else(|| {
                format!("loop '{loop_id}' references unknown control point '{point_id}'")
            })?;
            let shared_count = self
                .loops
                .iter()
                .filter(|candidate| {
                    candidate
                        .control_points
                        .iter()
                        .any(|candidate_id| candidate_id == point_id)
                })
                .count();
            info.push(EditorPointInfo {
                id: point.id.clone(),
                position: point.position,
                kind: point.kind,
                shared: shared_count > 1,
                shared_count,
            });
        }
        Ok(info)
    }

    /// Moves a shared control point, affecting every loop that references it.
    pub fn move_control_point(&mut self, point_id: &str, x: f32, z: f32) -> Result<(), String> {
        self.normalize_in_place()?;
        let point = self
            .control_points
            .iter_mut()
            .find(|point| point.id == point_id)
            .ok_or_else(|| format!("unknown control point '{point_id}'"))?;
        point.position = [x, z];
        self.normalize_in_place()
    }

    /// Clones a loop control point so the loop no longer shares it, returning
    /// the new point id.
    pub fn make_control_point_unique(
        &mut self,
        loop_id: &str,
        point_id: &str,
    ) -> Result<String, String> {
        self.normalize_in_place()?;
        let loop_index = self
            .loop_index(loop_id)
            .ok_or_else(|| format!("unknown loop id '{loop_id}'"))?;
        if !self.loops[loop_index]
            .control_points
            .iter()
            .any(|id| id == point_id)
        {
            return Err(format!(
                "loop '{loop_id}' does not reference control point '{point_id}'"
            ));
        }
        let source = self
            .control_point_by_id(point_id)
            .cloned()
            .ok_or_else(|| format!("unknown control point '{point_id}'"))?;
        let new_id = self.unique_control_point_id(&format!("{point_id}-unique"));
        let mut unique = source;
        unique.id = new_id.clone();
        self.control_points.push(unique);
        for id in &mut self.loops[loop_index].control_points {
            if id == point_id {
                *id = new_id.clone();
            }
        }
        self.normalize_in_place()?;
        Ok(new_id)
    }

    /// Inserts a new control point after `after_point_id` in this loop only,
    /// returning the new point id.
    pub fn insert_control_point(
        &mut self,
        loop_id: &str,
        after_point_id: &str,
        x: f32,
        z: f32,
    ) -> Result<String, String> {
        self.normalize_in_place()?;
        let loop_index = self
            .loop_index(loop_id)
            .ok_or_else(|| format!("unknown loop id '{loop_id}'"))?;
        let after_index = self.loops[loop_index]
            .control_points
            .iter()
            .position(|id| id == after_point_id)
            .ok_or_else(|| {
                format!("loop '{loop_id}' does not reference control point '{after_point_id}'")
            })?;
        let new_id = self.unique_control_point_id(&format!("{after_point_id}-insert"));
        self.control_points.push(EditorControlPoint {
            id: new_id.clone(),
            position: [x, z],
            kind: ControlPointKind::Normal,
            tangent: None,
        });
        self.loops[loop_index]
            .control_points
            .insert(after_index + 1, new_id.clone());
        self.normalize_in_place()?;
        Ok(new_id)
    }

    /// Removes a point from one loop only. A point that no other loop uses is
    /// pruned along with any segments that referenced it.
    pub fn delete_control_point(&mut self, loop_id: &str, point_id: &str) -> Result<(), String> {
        self.normalize_in_place()?;
        let loop_index = self
            .loop_index(loop_id)
            .ok_or_else(|| format!("unknown loop id '{loop_id}'"))?;
        let reference_index = self.loops[loop_index]
            .control_points
            .iter()
            .position(|id| id == point_id)
            .ok_or_else(|| {
                format!("loop '{loop_id}' does not reference control point '{point_id}'")
            })?;
        if self.loops[loop_index].control_points.len() <= 3 {
            return Err(format!(
                "loop '{loop_id}' needs at least three control points"
            ));
        }
        self.loops[loop_index]
            .control_points
            .remove(reference_index);

        let still_referenced = self
            .loops
            .iter()
            .any(|candidate| candidate.control_points.iter().any(|id| id == point_id));
        if !still_referenced {
            self.control_points.retain(|point| point.id != point_id);
            self.segments
                .retain(|segment| segment.from != point_id && segment.to != point_id);
            let mut clear_pit_lane = false;
            if let Some(pit_lane) = &mut self.pit_lane {
                pit_lane
                    .control_points
                    .retain(|pit_point_id| pit_point_id != point_id);
                clear_pit_lane = pit_lane.control_points.len() < 2;
            }
            if clear_pit_lane {
                self.pit_lane = None;
            }
            if !self
                .control_points
                .iter()
                .any(|point| point.kind == ControlPointKind::StartFinish)
            {
                if let Some(point) = self.control_points.first_mut() {
                    point.kind = ControlPointKind::StartFinish;
                }
            }
        }
        self.normalize_in_place()
    }

    /// Stores legacy route-sample kerb spans on the loop's segment entries.
    pub fn set_loop_kerb_spans(&mut self, id: &str, spans: &[KerbSpan]) -> Result<(), String> {
        self.normalize_in_place()?;
        let loop_ = self
            .loop_by_id(id)
            .cloned()
            .ok_or_else(|| format!("unknown loop id '{id}'"))?;
        let graph = self.to_graph()?;
        let layout = graph
            .layout_by_id(id)
            .ok_or_else(|| format!("unknown loop id '{id}'"))?
            .clone();
        let count = layout_centerline(&graph, &layout)?.len().max(1) as f32;
        let (ranges, route_len) = layout_route_ranges(&graph, &layout)?;
        let last = route_len.saturating_sub(1).max(1) as f32;
        let mut spans_by_edge = HashMap::<String, Vec<KerbSpan>>::new();
        for span in spans {
            let route_start = (span.start / count * last).clamp(0.0, last);
            let route_end = (span.end / count * last).clamp(0.0, last);
            if route_end - route_start <= 1e-4 {
                continue;
            }
            for (edge_id, range_start, range_end) in &ranges {
                let start = (*range_start as f32).max(route_start);
                let end = (*range_end as f32).min(route_end);
                let range_len = *range_end as f32 - *range_start as f32;
                if range_len <= 1e-6 || end - start <= 1e-4 {
                    continue;
                }
                spans_by_edge
                    .entry(edge_id.clone())
                    .or_default()
                    .push(KerbSpan {
                        start: ((start - *range_start as f32) / range_len).clamp(0.0, 1.0),
                        end: ((end - *range_start as f32) / range_len).clamp(0.0, 1.0),
                        side: span.side,
                    });
            }
        }
        let loop_pairs = loop_pair_set(&loop_);
        for segment in &mut self.segments {
            if loop_pairs.contains(&(segment.from.clone(), segment.to.clone())) {
                segment.kerbs.clear();
            }
        }
        for (edge_id, spans) in spans_by_edge {
            let Some(edge) = graph.edge_by_id(&edge_id) else {
                continue;
            };
            let key = (edge.from.clone(), edge.to.clone());
            if let Some(segment) = self
                .segments
                .iter_mut()
                .find(|segment| (segment.from.clone(), segment.to.clone()) == key)
            {
                segment.kerbs = spans;
            }
        }
        self.normalize_in_place()
    }

    /// Stores legacy route-sample prop lines on the loop's segment entries.
    pub fn set_loop_prop_lines(&mut self, id: &str, lines: &[PropLine]) -> Result<(), String> {
        self.normalize_in_place()?;
        let loop_ = self
            .loop_by_id(id)
            .cloned()
            .ok_or_else(|| format!("unknown loop id '{id}'"))?;
        let graph = self.to_graph()?;
        let layout = graph
            .layout_by_id(id)
            .ok_or_else(|| format!("unknown loop id '{id}'"))?
            .clone();
        let width = global_asphalt_width(&graph);
        let count = layout_centerline(&graph, &layout)?.len().max(1) as f32;
        let (ranges, route_len) = layout_route_ranges(&graph, &layout)?;
        let last = route_len.saturating_sub(1).max(1) as f32;
        let mut spans_by_edge = HashMap::<String, Vec<PropSpan>>::new();
        for line in lines {
            let Some(spline) = select_prop_spline_id(&graph.settings.splines, line, width) else {
                continue;
            };
            let route_start = (line.start / count * last).clamp(0.0, last);
            let route_end = (line.end / count * last).clamp(0.0, last);
            if route_end - route_start <= 1e-4 {
                continue;
            }
            for (edge_id, range_start, range_end) in &ranges {
                let start = (*range_start as f32).max(route_start);
                let end = (*range_end as f32).min(route_end);
                let range_len = *range_end as f32 - *range_start as f32;
                if range_len <= 1e-6 || end - start <= 1e-4 {
                    continue;
                }
                spans_by_edge
                    .entry(edge_id.clone())
                    .or_default()
                    .push(PropSpan {
                        spline: spline.clone(),
                        start: ((start - *range_start as f32) / range_len).clamp(0.0, 1.0),
                        end: ((end - *range_start as f32) / range_len).clamp(0.0, 1.0),
                        kind: line.kind,
                    });
            }
        }
        let loop_pairs = loop_pair_set(&loop_);
        for segment in &mut self.segments {
            if loop_pairs.contains(&(segment.from.clone(), segment.to.clone())) {
                segment.prop_spans.clear();
            }
        }
        for (edge_id, spans) in spans_by_edge {
            let Some(edge) = graph.edge_by_id(&edge_id) else {
                continue;
            };
            let key = (edge.from.clone(), edge.to.clone());
            if let Some(segment) = self
                .segments
                .iter_mut()
                .find(|segment| (segment.from.clone(), segment.to.clone()) == key)
            {
                segment.prop_spans = spans;
            }
        }
        self.normalize_in_place()
    }

    /// Converts the loop/shared-control-point model into the internal `@2`
    /// venue graph used by the geometry builder.
    pub fn to_graph(&self) -> Result<TrackProject, String> {
        let project = self.clone().normalize()?;
        let width = clamp_layout_width(project.settings.width);
        let positions = project
            .control_points
            .iter()
            .map(|point| (point.id.clone(), point.position))
            .collect::<HashMap<_, _>>();

        let mut explicit_tangents = HashMap::<String, [f32; 2]>::new();
        for point in &project.control_points {
            let Some(tangent) = point.tangent else {
                continue;
            };
            if !tangent[0].is_finite() || !tangent[1].is_finite() {
                continue;
            }
            let length = tangent[0].hypot(tangent[1]);
            if length > 1e-6 {
                explicit_tangents
                    .insert(point.id.clone(), [tangent[0] / length, tangent[1] / length]);
            }
        }

        let segments = project
            .segments
            .iter()
            .map(|segment| ((segment.from.clone(), segment.to.clone()), segment.clone()))
            .collect::<HashMap<_, _>>();
        let segment_roles = project
            .segments
            .iter()
            .map(|segment| ((segment.from.clone(), segment.to.clone()), segment.role))
            .collect::<HashMap<_, _>>();

        // The union of all loops defines the venue graph. Average the main-loop
        // travel directions at each control point to derive its point tangent.
        let mut tangent_sums = HashMap::<String, (f32, f32)>::new();
        let mut tangent_pairs = BTreeSet::<(String, String)>::new();
        for loop_ in &project.loops {
            for index in 0..loop_.control_points.len() {
                let from_id = &loop_.control_points[index];
                let to_id = &loop_.control_points[(index + 1) % loop_.control_points.len()];
                let key = (from_id.clone(), to_id.clone());
                if !tangent_pairs.insert(key.clone()) {
                    continue;
                }
                let role = segment_roles.get(&key).copied().unwrap_or_default();
                // Branch edges never contribute to the main-loop tangent
                // average, even at a junction.
                if role != EditorSegmentRole::Main {
                    continue;
                }
                let (Some(from), Some(to)) = (positions.get(from_id), positions.get(to_id)) else {
                    continue;
                };
                let dx = to[0] - from[0];
                let dz = to[1] - from[1];
                let length = dx.hypot(dz);
                if length > 1e-6 {
                    let direction = (dx / length, dz / length);
                    // Both endpoints share the same travel direction: `from`
                    // sees it as outgoing, `to` as incoming. Averaging both at
                    // each point yields the true point tangent.
                    for point_id in [from_id, to_id] {
                        let entry = tangent_sums.entry(point_id.clone()).or_insert((0.0, 0.0));
                        entry.0 += direction.0;
                        entry.1 += direction.1;
                    }
                }
            }
        }

        let mut tangents = HashMap::<String, [f32; 2]>::new();
        for point in &project.control_points {
            if let Some(tangent) = explicit_tangents.get(&point.id) {
                tangents.insert(point.id.clone(), *tangent);
            } else if let Some((x, z)) = tangent_sums.get(&point.id) {
                let length = x.hypot(*z);
                if length > 1e-6 {
                    tangents.insert(point.id.clone(), [x / length, z / length]);
                }
            }
        }

        let mut edges = Vec::new();
        let mut edge_pairs = BTreeSet::<(String, String)>::new();
        for loop_ in &project.loops {
            for index in 0..loop_.control_points.len() {
                let from = loop_.control_points[index].clone();
                let to = loop_.control_points[(index + 1) % loop_.control_points.len()].clone();
                let key = (from.clone(), to.clone());
                if !edge_pairs.insert(key.clone()) {
                    continue;
                }
                let segment = segments.get(&key);
                let role = segment_roles.get(&key).copied().unwrap_or_default();
                let branch_tangent = match (positions.get(&from), positions.get(&to)) {
                    (Some(from_position), Some(to_position)) => {
                        let dx = to_position[0] - from_position[0];
                        let dz = to_position[1] - from_position[1];
                        let length = dx.hypot(dz);
                        (length > 1e-6).then(|| [dx / length, dz / length])
                    }
                    _ => None,
                };
                // Main edges follow the averaged main-loop point tangent. Branch
                // edges seed both endpoints with their own chord direction so
                // they can meet a junction at a distinct angle.
                let (from_tangent, to_tangent) = match (role, branch_tangent) {
                    (EditorSegmentRole::Branch, Some(tangent)) => (Some(tangent), Some(tangent)),
                    _ => (tangents.get(&from).copied(), tangents.get(&to).copied()),
                };
                edges.push(EdgeProject {
                    id: format!("{from}->{to}"),
                    from,
                    to,
                    start_profile: None,
                    end_profile: None,
                    kerbs: segment
                        .map(|segment| segment.kerbs.clone())
                        .unwrap_or_default(),
                    prop_spans: segment
                        .map(|segment| segment.prop_spans.clone())
                        .unwrap_or_default(),
                    points: Vec::new(),
                    from_tangent,
                    to_tangent,
                });
            }
        }

        if let Some(pit_lane) = &project.pit_lane {
            for pair in pit_lane.control_points.windows(2) {
                let key = (pair[0].clone(), pair[1].clone());
                if !edge_pairs.insert(key.clone()) {
                    continue;
                }
                let from_tangent = tangents.get(&pair[0]).copied();
                let to_tangent = tangents.get(&pair[1]).copied();
                edges.push(EdgeProject {
                    id: format!("{}->{}", pair[0], pair[1]),
                    from: pair[0].clone(),
                    to: pair[1].clone(),
                    start_profile: None,
                    end_profile: None,
                    kerbs: Vec::new(),
                    prop_spans: Vec::new(),
                    points: Vec::new(),
                    from_tangent,
                    to_tangent,
                });
            }
        }

        let mut branch_tangents = BTreeMap::<String, BTreeMap<String, [f32; 2]>>::new();
        for edge in &edges {
            // Mirror the canonical edge endpoint tangents into the legacy
            // per-control-point map so both views agree at every junction.
            if let Some(tangent) = edge.from_tangent {
                branch_tangents
                    .entry(edge.from.clone())
                    .or_default()
                    .insert(edge.id.clone(), tangent);
            }
            if let Some(tangent) = edge.to_tangent {
                branch_tangents
                    .entry(edge.to.clone())
                    .or_default()
                    .insert(edge.id.clone(), tangent);
            }
        }

        let control_points = project
            .control_points
            .iter()
            .map(|point| ControlPoint {
                id: point.id.clone(),
                position: point.position,
                kind: point.kind,
                profile: None,
                branch_tangents: branch_tangents.remove(&point.id).unwrap_or_default(),
            })
            .collect::<Vec<_>>();

        let mut layouts = project
            .loops
            .iter()
            .map(|loop_| LayoutProject {
                id: loop_.id.clone(),
                name: loop_.name.clone(),
                route: (0..loop_.control_points.len())
                    .map(|index| {
                        format!(
                            "{}->{}",
                            loop_.control_points[index],
                            loop_.control_points[(index + 1) % loop_.control_points.len()]
                        )
                    })
                    .collect(),
                sectors: loop_.sectors.clone(),
                cameras: loop_.cameras.clone(),
                spawns: loop_.spawns.clone(),
                start_finish: StartFinish::default(),
            })
            .collect::<Vec<_>>();

        let default_loop = project
            .loops
            .first()
            .ok_or_else(|| "track project needs at least one loop".to_string())?;
        let start_index = default_loop
            .control_points
            .iter()
            .position(|point_id| {
                project
                    .control_points
                    .iter()
                    .find(|point| point.id == *point_id)
                    .map(|point| point.kind == ControlPointKind::StartFinish)
                    .unwrap_or(false)
            })
            .unwrap_or(0);
        let start_from = &default_loop.control_points[start_index];
        let start_to =
            &default_loop.control_points[(start_index + 1) % default_loop.control_points.len()];
        let start_edge = format!("{start_from}->{start_to}");
        let start_finish = StartFinish {
            edge: if edges.iter().any(|edge| edge.id == start_edge) {
                start_edge
            } else {
                edges
                    .first()
                    .map(|edge| edge.id.clone())
                    .unwrap_or_default()
            },
            fraction: 0.0,
        };

        for layout in &mut layouts {
            layout.start_finish = if layout.route.iter().any(|edge| edge == &start_finish.edge) {
                start_finish.clone()
            } else {
                StartFinish {
                    edge: layout.route.first().cloned().unwrap_or_default(),
                    fraction: 0.0,
                }
            };
        }

        let pit_lane = project.pit_lane.as_ref().map(|pit_lane| PitLaneProject {
            id: "pit".to_string(),
            route: pit_lane
                .control_points
                .windows(2)
                .map(|pair| format!("{}->{}", pair[0], pair[1]))
                .collect(),
            boxes: pit_lane.boxes.clone(),
        });
        let paths = project
            .pit_lane
            .as_ref()
            .map(|pit_lane| PathProject {
                id: "pit".to_string(),
                kind: "pit_lane".to_string(),
                route: pit_lane
                    .control_points
                    .windows(2)
                    .map(|pair| format!("{}->{}", pair[0], pair[1]))
                    .collect(),
                boxes: pit_lane.boxes.clone(),
            })
            .into_iter()
            .collect::<Vec<_>>();

        Ok(TrackProject {
            schema: TRACK_PROJECT_SCHEMA_V4.to_string(),
            name: project.name.clone(),
            settings: VenueSettings {
                splines: default_splines(width),
                kerb: project.settings.kerb,
                pit: project.settings.pit.unwrap_or_default(),
            },
            start_finish,
            pit_lane,
            control_points,
            edges,
            segments: project.segments.clone(),
            layouts,
            paths,
        })
    }
}

#[allow(dead_code)]
fn loop_pair_set(loop_: &EditorLoop) -> BTreeSet<(String, String)> {
    (0..loop_.control_points.len())
        .map(|index| {
            (
                loop_.control_points[index].clone(),
                loop_.control_points[(index + 1) % loop_.control_points.len()].clone(),
            )
        })
        .collect()
}

#[allow(dead_code)]
fn apply_width_to_profile(profile: &mut Profile, width: f32) {
    let half = width * 0.5;
    for (id, value) in &mut profile.values {
        match id.as_str() {
            "asphalt-left" => *value = -half,
            "asphalt-right" => *value = half,
            "runoff-left" => *value = -width,
            "runoff-right" => *value = width,
            _ => {}
        }
    }
}

fn validate_unique_ids<'a>(ids: impl Iterator<Item = &'a str>, label: &str) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for id in ids {
        if id.trim().is_empty() {
            return Err(format!("{label} ids must not be empty"));
        }
        if !seen.insert(id) {
            return Err(format!("duplicate {label} id '{id}'"));
        }
    }
    Ok(())
}

fn validate_splines(splines: &[SplineSpec]) -> Result<(), String> {
    let surface_ids = splines
        .iter()
        .filter(|spline| spline.role == SplineRole::Surface)
        .map(|spline| spline.id.as_str())
        .collect::<BTreeSet<_>>();
    if surface_ids.len() < 2 {
        return Err("venue needs at least two surface splines".to_string());
    }
    for spline in splines {
        if !spline.offset.is_finite() {
            return Err(format!("spline '{}' has a non-finite offset", spline.id));
        }
        if let Some(base) = &spline.base {
            if !surface_ids.contains(base.as_str()) {
                return Err(format!(
                    "spline '{}' references unknown base surface spline '{base}'",
                    spline.id
                ));
            }
        }
    }
    Ok(())
}

fn effective_profile_offsets(
    splines: &[SplineSpec],
    profile: &Profile,
    label: &str,
) -> Result<BTreeMap<String, f32>, String> {
    let surface_ids = splines
        .iter()
        .filter(|spline| spline.role == SplineRole::Surface)
        .map(|spline| spline.id.as_str())
        .collect::<BTreeSet<_>>();
    for (id, value) in &profile.values {
        if !surface_ids.contains(id.as_str()) {
            return Err(format!("{label} references unknown surface spline '{id}'"));
        }
        if !value.is_finite() {
            return Err(format!("{label} value for '{id}' is not finite"));
        }
    }

    let mut offsets = BTreeMap::new();
    let mut previous: Option<(f32, &str)> = None;
    for spline in splines
        .iter()
        .filter(|spline| spline.role == SplineRole::Surface)
    {
        let offset = profile
            .values
            .get(&spline.id)
            .copied()
            .unwrap_or(spline.offset);
        if !offset.is_finite() {
            return Err(format!("{label} value for '{}' is not finite", spline.id));
        }
        if let Some((previous_offset, previous_id)) = previous {
            if offset <= previous_offset {
                return Err(format!(
                    "{label} spline offsets must increase in declared order: '{}' ({previous_offset}) then '{}' ({offset})",
                    previous_id, spline.id
                ));
            }
        }
        previous = Some((offset, spline.id.as_str()));
        offsets.insert(spline.id.clone(), offset);
    }
    Ok(offsets)
}

fn validate_profile(profile: &Profile, splines: &[SplineSpec], label: &str) -> Result<(), String> {
    let offsets = effective_profile_offsets(splines, profile, label)?;
    let asphalt_left = offsets.get("asphalt-left").copied();
    let asphalt_right = offsets.get("asphalt-right").copied();
    if let (Some(left), Some(right)) = (asphalt_left, asphalt_right) {
        if right - left <= 1e-3 {
            return Err(format!(
                "{label} has a degenerate asphalt band: {left}..{right}"
            ));
        }
        if let Some(runoff_left) = offsets.get("runoff-left").copied() {
            if runoff_left >= left {
                return Err(format!(
                    "{label} runoff-left ({runoff_left}) must lie outside asphalt-left ({left})"
                ));
            }
        }
        if let Some(runoff_right) = offsets.get("runoff-right").copied() {
            if runoff_right <= right {
                return Err(format!(
                    "{label} runoff-right ({runoff_right}) must lie outside asphalt-right ({right})"
                ));
            }
        }
    }
    Ok(())
}

fn validate_span(start: f32, end: f32, label: &str) -> Result<(), String> {
    if !start.is_finite() || !end.is_finite() {
        return Err(format!("{label} must be finite"));
    }
    if !(0.0..=1.0).contains(&start) || !(0.0..=1.0).contains(&end) {
        return Err(format!("{label} must lie in [0, 1]"));
    }
    if start >= end {
        return Err(format!("{label} start must be less than end"));
    }
    Ok(())
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyTrackProject {
    schema: String,
    name: String,
    #[serde(default)]
    default_layout: String,
    #[serde(default)]
    layouts: Vec<LegacyLayoutProject>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyLayoutProject {
    id: String,
    name: String,
    #[serde(default = "default_layout_width")]
    width: f32,
    #[serde(default)]
    control_points: Vec<[f32; 2]>,
    #[serde(default)]
    kerbs: Vec<KerbSpan>,
    #[serde(default)]
    prop_lines: Vec<PropLine>,
    #[serde(default)]
    start_finish: f32,
    #[serde(default)]
    spawns: Vec<SpawnPoint>,
    #[serde(default)]
    pit_boxes: Vec<SpawnPoint>,
    #[serde(default)]
    cameras: Vec<CameraSpec>,
    #[serde(default)]
    sectors: Vec<SectorSpec>,
}

struct MigratedLayout {
    id: String,
    name: String,
    route: Vec<String>,
    control_points: Vec<ControlPoint>,
    edges: Vec<EdgeProject>,
    sectors: Vec<SectorSpec>,
    cameras: Vec<CameraSpec>,
    spawns: Vec<SpawnPoint>,
}

fn profile_for_width(width: f32) -> Profile {
    let half = width * 0.5;
    let mut values = BTreeMap::new();
    values.insert("asphalt-left".to_string(), -half);
    values.insert("asphalt-right".to_string(), half);
    values.insert("runoff-left".to_string(), -width);
    values.insert("runoff-right".to_string(), width);
    Profile { values }
}

impl LegacyTrackProject {
    fn migrate(self) -> Result<TrackProject, String> {
        if self.schema != TRACK_PROJECT_SCHEMA_V1 {
            return Err(format!(
                "unsupported legacy track project schema '{}'",
                self.schema
            ));
        }
        if self.layouts.is_empty() {
            return Err("legacy track project needs at least one layout".to_string());
        }
        let default_index = self
            .layouts
            .iter()
            .position(|layout| layout.id == self.default_layout)
            .unwrap_or(0);
        let default_width = clamp_layout_width(self.layouts[default_index].width);
        let settings = VenueSettings {
            splines: default_splines(default_width),
            ..VenueSettings::default()
        };

        let mut used_layout_ids = BTreeSet::new();
        let mut migrated = Vec::with_capacity(self.layouts.len());
        for (layout_index, legacy) in self.layouts.into_iter().enumerate() {
            if legacy.control_points.len() < 3 {
                return Err(format!(
                    "layout '{}' needs at least three control points",
                    legacy.id
                ));
            }
            let width = clamp_layout_width(legacy.width);
            let profile = profile_for_width(width);
            let mut control_points = Vec::with_capacity(legacy.control_points.len());
            for (point_index, position) in legacy.control_points.iter().enumerate() {
                control_points.push(ControlPoint {
                    id: format!("layout-{layout_index}-control-{point_index}"),
                    position: *position,
                    kind: ControlPointKind::Normal,
                    profile: Some(profile.clone()),
                    branch_tangents: BTreeMap::new(),
                });
            }
            let mut edges = Vec::with_capacity(control_points.len());
            for (point_index, point) in control_points.iter().enumerate() {
                let next = &control_points[(point_index + 1) % control_points.len()];
                edges.push(EdgeProject {
                    id: format!("layout-{layout_index}-edge-{point_index}"),
                    from: point.id.clone(),
                    to: next.id.clone(),
                    start_profile: None,
                    end_profile: None,
                    kerbs: Vec::new(),
                    prop_spans: Vec::new(),
                    points: Vec::new(),
                    from_tangent: None,
                    to_tangent: None,
                });
            }
            let route = edges.iter().map(|edge| edge.id.clone()).collect();
            let id = unique_id(
                used_layout_ids.iter().map(|id: &String| id.as_str()),
                &legacy.id,
            );
            used_layout_ids.insert(id.clone());
            migrated.push(MigratedLayout {
                id,
                name: legacy.name,
                route,
                control_points,
                edges,
                sectors: legacy.sectors,
                cameras: legacy.cameras,
                spawns: legacy.spawns,
            });
        }

        let start_finish_edge = migrated[default_index]
            .route
            .first()
            .cloned()
            .ok_or_else(|| "legacy default layout route is empty".to_string())?;

        let mut control_points = Vec::new();
        let mut edges = Vec::new();
        let mut layouts = Vec::with_capacity(migrated.len());
        for (layout_index, layout) in migrated.into_iter().enumerate() {
            let mut route = layout.route;
            control_points.extend(layout.control_points.iter().cloned());
            edges.extend(layout.edges.iter().cloned());
            if layout_index != default_index {
                let start_edge = migrated_edge(edges.as_slice(), &start_finish_edge)
                    .ok_or_else(|| "legacy start/finish edge is missing".to_string())?;
                let (start_from, start_to) = (start_edge.from.clone(), start_edge.to.clone());
                let first_point = layout
                    .control_points
                    .first()
                    .ok_or_else(|| "legacy layout has no control points".to_string())?;
                let attach_id = first_point.id.clone();
                let bridge_in = EdgeProject {
                    id: format!("layout-{layout_index}-bridge-in"),
                    from: start_to.clone(),
                    to: attach_id.clone(),
                    start_profile: None,
                    end_profile: None,
                    kerbs: Vec::new(),
                    prop_spans: Vec::new(),
                    points: Vec::new(),
                    from_tangent: None,
                    to_tangent: None,
                };
                let bridge_out = EdgeProject {
                    id: format!("layout-{layout_index}-bridge-out"),
                    from: attach_id.clone(),
                    to: start_from.clone(),
                    start_profile: None,
                    end_profile: None,
                    kerbs: Vec::new(),
                    prop_spans: Vec::new(),
                    points: Vec::new(),
                    from_tangent: None,
                    to_tangent: None,
                };
                edges.push(bridge_in.clone());
                edges.push(bridge_out.clone());
                let mut attached = vec![start_finish_edge.clone(), bridge_in.id];
                attached.append(&mut route);
                attached.push(bridge_out.id);
                route = attached;

                for point_id in [start_from.as_str(), start_to.as_str(), attach_id.as_str()] {
                    if let Some(point) =
                        control_points.iter_mut().find(|point| point.id == point_id)
                    {
                        if point.kind == ControlPointKind::Normal
                            || point.kind == ControlPointKind::StartFinish
                        {
                            point.kind = ControlPointKind::Fork;
                        }
                    }
                }
            }
            layouts.push(LayoutProject {
                id: layout.id,
                name: layout.name,
                route,
                sectors: layout.sectors,
                cameras: layout.cameras,
                spawns: layout.spawns,
                start_finish: StartFinish {
                    edge: start_finish_edge.clone(),
                    fraction: 0.0,
                },
            });
        }

        Ok(TrackProject {
            schema: TRACK_PROJECT_SCHEMA_V4.to_string(),
            name: self.name,
            settings,
            start_finish: StartFinish {
                edge: start_finish_edge,
                fraction: 0.0,
            },
            pit_lane: None,
            control_points,
            edges,
            segments: Vec::new(),
            layouts,
            paths: Vec::new(),
        })
    }
}

fn migrated_edge<'a>(edges: &'a [EdgeProject], id: &str) -> Option<&'a EdgeProject> {
    edges.iter().find(|edge| edge.id == id)
}

fn clamp_layout_width(width: f32) -> f32 {
    if width.is_finite() {
        width.clamp(MIN_LAYOUT_WIDTH, MAX_LAYOUT_WIDTH)
    } else {
        DEFAULT_LAYOUT_WIDTH
    }
}

#[allow(dead_code)]
pub struct EditorTrack {
    pub visual: TrackVisual,
    pub metadata: TrackMetadata,
    pub collision: TrackCollision,
    pub manifest: AssetManifest,
    pub kerb_spans: Vec<KerbSpan>,
    pub prop_lines: Vec<PropLine>,
    pub project: TrackProject,
}

#[allow(dead_code)]
pub fn default_control_points() -> Vec<[f32; 3]> {
    CONTROL_POINTS
        .iter()
        .map(|point| [point[0], terrain_height(point[0], point[2]), point[2]])
        .collect()
}

#[allow(dead_code)]
pub fn control_points_with_heights(points: &[[f32; 3]]) -> Vec<[f32; 3]> {
    points
        .iter()
        .map(|point| [point[0], terrain_height(point[0], point[2]), point[2]])
        .collect()
}

pub fn layout_control_points_with_heights(points: &[[f32; 2]]) -> Vec<[f32; 3]> {
    points
        .iter()
        .map(|point| [point[0], terrain_height(point[0], point[1]), point[1]])
        .collect()
}

pub fn layout_control_points(
    project: &TrackProject,
    layout: &LayoutProject,
) -> Result<Vec<[f32; 2]>, String> {
    if layout.route.is_empty() {
        return Err(format!("layout '{}' route must not be empty", layout.id));
    }
    let mut points = Vec::with_capacity(layout.route.len());
    for edge_id in &layout.route {
        let edge = project
            .edges
            .iter()
            .find(|edge| edge.id == *edge_id)
            .ok_or_else(|| {
                format!(
                    "layout '{}' route references unknown edge '{edge_id}'",
                    layout.id
                )
            })?;
        let point = project
            .control_points
            .iter()
            .find(|point| point.id == edge.from)
            .ok_or_else(|| {
                format!(
                    "edge '{}' references unknown control point '{}'",
                    edge.id, edge.from
                )
            })?;
        points.push(point.position);
    }
    Ok(points)
}

fn global_asphalt_width(project: &TrackProject) -> f32 {
    let left = project
        .settings
        .splines
        .iter()
        .find(|spline| spline.id == "asphalt-left")
        .map(|spline| spline.offset);
    let right = project
        .settings
        .splines
        .iter()
        .find(|spline| spline.id == "asphalt-right")
        .map(|spline| spline.offset);
    match (left, right) {
        (Some(left), Some(right)) if right.is_finite() && left.is_finite() && right > left => {
            (right - left).clamp(MIN_LAYOUT_WIDTH, MAX_LAYOUT_WIDTH)
        }
        _ => DEFAULT_LAYOUT_WIDTH,
    }
}

pub fn layout_centerline(
    project: &TrackProject,
    layout: &LayoutProject,
) -> Result<Vec<[f32; 3]>, String> {
    let control_points = layout_control_points(project, layout)?;
    let points = layout_control_points_with_heights(&control_points);
    let width = global_asphalt_width(project);
    let samples = sample_track_with_min_radius(&points, min_turn_radius(width));
    if samples.len() < 3 {
        return Err(format!(
            "layout '{}' needs at least three samples",
            layout.id
        ));
    }
    Ok(samples.iter().map(|point| point.to_array()).collect())
}

fn layout_route_points(project: &TrackProject, layout: &LayoutProject) -> Vec<Vec3> {
    layout_centerline(project, layout)
        .map(|points| {
            points
                .iter()
                .map(|point| Vec3::from_array(*point))
                .collect()
        })
        .unwrap_or_default()
}

pub fn layout_kerb_spans(
    project: &TrackProject,
    layout: &LayoutProject,
) -> Result<Vec<KerbSpan>, String> {
    let points = layout_route_points(project, layout);
    let count = points.len();
    if count < 2 {
        return Ok(Vec::new());
    }
    let (ranges, route_len) = layout_route_ranges(project, layout)?;
    let last = route_len.saturating_sub(1).max(1) as f32;
    let count = count as f32;
    let mut spans = Vec::new();
    for (edge_id, range_start, range_end) in &ranges {
        let Some(edge) = project.edges.iter().find(|edge| edge.id == *edge_id) else {
            continue;
        };
        let range_start = *range_start as f32;
        let range_len = *range_end as f32 - range_start;
        for span in &edge.kerbs {
            spans.push(KerbSpan {
                start: (range_start + span.start * range_len) / last * count,
                end: (range_start + span.end * range_len) / last * count,
                side: span.side,
            });
        }
    }
    if spans.is_empty() {
        Ok(auto_kerb_spans(&points))
    } else {
        Ok(sanitize_kerb_spans(&spans, count as usize))
    }
}

pub fn layout_prop_lines(
    project: &TrackProject,
    layout: &LayoutProject,
) -> Result<Vec<PropLine>, String> {
    let points = layout_route_points(project, layout);
    let count = points.len();
    if count < 2 {
        return Ok(Vec::new());
    }
    let (ranges, route_len) = layout_route_ranges(project, layout)?;
    let last = route_len.saturating_sub(1).max(1) as f32;
    let count_f = count as f32;
    let mut lines = Vec::new();
    for (edge_id, range_start, range_end) in &ranges {
        let Some(edge) = project.edges.iter().find(|edge| edge.id == *edge_id) else {
            continue;
        };
        let range_start = *range_start as f32;
        let range_len = *range_end as f32 - range_start;
        for span in &edge.prop_spans {
            let Some(spline) = project
                .settings
                .splines
                .iter()
                .find(|spline| spline.id == span.spline)
            else {
                continue;
            };
            let lateral = prop_spline_lateral(&project.settings.splines, spline);
            lines.push(PropLine {
                start: (range_start + span.start * range_len) / last * count_f,
                end: (range_start + span.end * range_len) / last * count_f,
                side: if lateral >= 0.0 { 1 } else { -1 },
                offset: lateral.abs().max(MIN_PROP_OFFSET),
                kind: span.kind,
            });
        }
    }
    if lines.is_empty() {
        let width = global_asphalt_width(project);
        Ok(default_prop_lines_with_offsets(
            &points,
            default_prop_offset(width),
            default_prop_far_offset(width),
        ))
    } else {
        Ok(sanitize_prop_lines_with_limit(
            &lines,
            count,
            default_prop_far_offset(global_asphalt_width(project)),
        ))
    }
}

fn segmented_ring(count: usize, segments: usize, gap: f32) -> Vec<(f32, f32)> {
    if count < 2 || segments == 0 {
        return Vec::new();
    }
    let length = count as f32 / segments as f32;
    let gap = gap.clamp(0.0, length * 0.5);
    (0..segments)
        .filter_map(|index| {
            let start = index as f32 * length + gap * 0.5;
            let end = (index + 1) as f32 * length - gap * 0.5;
            (end - start > 1e-4).then_some((start, end))
        })
        .collect()
}

#[allow(dead_code)]
fn default_prop_lines(points: &[Vec3]) -> Vec<PropLine> {
    default_prop_lines_with_offsets(points, PROP_OFFSET, PROP_FAR_OFFSET)
}

/// Like `default_prop_lines_with_offsets`, but keeps only the outside lane so
/// the loop's interior stays free of the default tyre ring.
fn default_prop_lines_outside_only(
    points: &[Vec3],
    near_offset: f32,
    far_offset: f32,
) -> Vec<PropLine> {
    let outside_side = route_outside_side(points);
    default_prop_lines_with_offsets(points, near_offset, far_offset)
        .into_iter()
        .filter(|line| line.side == outside_side)
        .collect()
}

/// Returns the route side that points away from the loop's interior, matching
/// the signed-area convention used by `default_prop_lines_with_offsets`.
fn route_outside_side(points: &[Vec3]) -> i8 {
    let area = points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            let next = points[(index + 1) % points.len()];
            point.x * next.z - next.x * point.z
        })
        .sum::<f32>();
    if area > 0.0 {
        -1
    } else {
        1
    }
}

fn default_prop_lines_with_offsets(
    points: &[Vec3],
    near_offset: f32,
    far_offset: f32,
) -> Vec<PropLine> {
    let area = points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            let next = points[(index + 1) % points.len()];
            point.x * next.z - next.x * point.z
        })
        .sum::<f32>();
    let outside_side = if area > 0.0 { -1 } else { 1 };
    let mut lines = Vec::new();
    for (start, end) in segmented_ring(
        points.len(),
        DEFAULT_NEAR_PROP_SEGMENTS,
        DEFAULT_NEAR_PROP_GAP,
    ) {
        lines.push(PropLine {
            start,
            end,
            side: outside_side,
            offset: near_offset,
            kind: PropKind::Blocks,
        });
        lines.push(PropLine {
            start,
            end,
            side: -outside_side,
            offset: near_offset,
            kind: PropKind::Tyres,
        });
    }
    for (start, end) in segmented_ring(
        points.len(),
        DEFAULT_TREE_RING_SEGMENTS,
        DEFAULT_TREE_RING_GAP,
    ) {
        for side in [outside_side, -outside_side] {
            lines.push(PropLine {
                start,
                end,
                side,
                offset: far_offset,
                kind: PropKind::Trees,
            });
        }
    }
    lines
}

#[allow(dead_code)]
fn sanitize_prop_lines(lines: &[PropLine], count: usize) -> Vec<PropLine> {
    sanitize_prop_lines_with_limit(lines, count, MAX_PROP_OFFSET)
}

fn sanitize_prop_lines_with_limit(
    lines: &[PropLine],
    count: usize,
    max_offset: f32,
) -> Vec<PropLine> {
    if count < 2 {
        return Vec::new();
    }
    let limit = count as f32;
    let max_offset = max_offset.max(MIN_PROP_OFFSET);
    let mut lines = lines
        .iter()
        .filter_map(|line| {
            let start = line.start.clamp(0.0, limit);
            let end = line.end.clamp(0.0, limit);
            let offset = if line.offset.is_finite() {
                line.offset.clamp(MIN_PROP_OFFSET, max_offset)
            } else {
                PROP_OFFSET.clamp(MIN_PROP_OFFSET, max_offset)
            };
            (end - start > 1e-4).then_some(PropLine {
                start,
                end,
                side: if line.side >= 0 { 1 } else { -1 },
                offset,
                kind: line.kind,
            })
        })
        .collect::<Vec<_>>();
    lines.sort_by(|a, b| {
        a.side
            .cmp(&b.side)
            .then_with(|| a.offset.total_cmp(&b.offset))
            .then_with(|| a.start.total_cmp(&b.start))
    });

    let mut lanes: Vec<(i8, f32, f32)> = Vec::new();
    let mut sanitized = Vec::with_capacity(lines.len());
    for line in lines {
        let lane = lanes.iter().position(|(side, offset, _)| {
            *side == line.side && (*offset - line.offset).abs() < 1e-4
        });
        let start = line
            .start
            .max(lane.map(|index| lanes[index].2).unwrap_or(0.0));
        if start < line.end {
            match lane {
                Some(index) => lanes[index].2 = line.end,
                None => lanes.push((line.side, line.offset, line.end)),
            }
            sanitized.push(PropLine { start, ..line });
        }
    }
    sanitized
}

#[allow(dead_code)]
pub fn centerline_points(control_points: &[[f32; 3]]) -> Vec<[f32; 3]> {
    sample_track(control_points)
        .iter()
        .map(|point| point.to_array())
        .collect()
}

fn auto_kerb_spans(points: &[Vec3]) -> Vec<KerbSpan> {
    let count = points.len();
    if count < 4 {
        return Vec::new();
    }
    let mut candidates = Vec::new();
    for index in 0..count {
        let previous = points[(index + count - 1) % count];
        let current = points[index];
        let next = points[(index + 1) % count];
        let radius = curvature_radius(previous, current, next);
        if !radius.is_finite() || radius <= 0.0 {
            continue;
        }
        let curvature = 1.0 / radius;
        if curvature < KERB_MIN_CURVATURE {
            continue;
        }
        let tangent = (next - current).normalize_or_zero();
        let future = points[(index + 2) % count];
        let future_tangent = (future - next).normalize_or_zero();
        let side = if tangent.cross(future_tangent).y > 0.0 {
            1
        } else {
            -1
        };
        candidates.push((curvature, index, side));
    }
    candidates.sort_by(|a, b| b.0.total_cmp(&a.0));

    let mut peaks: Vec<(f32, usize, i8)> = Vec::new();
    for candidate in candidates {
        if peaks.len() >= DEFAULT_KERB_BENDS {
            break;
        }
        let crowded = peaks.iter().any(|(_, index, side)| {
            *side == candidate.2
                && circular_distance(candidate.1, *index, count) <= KERB_PEAK_SPACING
        });
        if !crowded {
            peaks.push(candidate);
        }
    }

    let mut spans = Vec::new();
    for (_, index, side) in peaks {
        let center = index as f32;
        let start = center - DEFAULT_KERB_BEND_HALF_LENGTH;
        let end = center + DEFAULT_KERB_BEND_HALF_LENGTH;
        for (piece_start, piece_end) in split_kerb_span(
            start,
            end,
            DEFAULT_KERB_SEGMENTS_PER_BEND,
            DEFAULT_KERB_SEGMENT_GAP,
        ) {
            push_wrapped_kerb_span(&mut spans, piece_start, piece_end, side, count as f32);
        }
    }
    sanitize_kerb_spans(&spans, count)
}

fn circular_distance(a: usize, b: usize, count: usize) -> f32 {
    let direct = (a as f32 - b as f32).abs();
    direct.min(count as f32 - direct)
}

fn split_kerb_span(start: f32, end: f32, pieces: usize, gap: f32) -> Vec<(f32, f32)> {
    if pieces == 0 || end <= start {
        return Vec::new();
    }
    let piece = (end - start) / pieces as f32;
    (0..pieces)
        .filter_map(|index| {
            let piece_start = start + index as f32 * piece + gap * 0.5;
            let piece_end = start + (index + 1) as f32 * piece - gap * 0.5;
            (piece_end - piece_start > 0.1).then_some((piece_start, piece_end))
        })
        .collect()
}

fn push_wrapped_kerb_span(spans: &mut Vec<KerbSpan>, start: f32, end: f32, side: i8, count: f32) {
    if end <= start {
        return;
    }
    if start < 0.0 {
        let wrapped_start = start + count;
        if wrapped_start < count {
            spans.push(KerbSpan {
                start: wrapped_start,
                end: count,
                side,
            });
        }
        if end > 0.0 {
            spans.push(KerbSpan {
                start: 0.0,
                end,
                side,
            });
        }
    } else if end > count {
        spans.push(KerbSpan {
            start,
            end: count,
            side,
        });
        let wrapped_end = end - count;
        if wrapped_end > 0.0 {
            spans.push(KerbSpan {
                start: 0.0,
                end: wrapped_end,
                side,
            });
        }
    } else {
        spans.push(KerbSpan { start, end, side });
    }
}

fn sanitize_kerb_spans(spans: &[KerbSpan], count: usize) -> Vec<KerbSpan> {
    if count < 2 {
        return Vec::new();
    }
    let limit = count as f32;
    let mut spans = spans
        .iter()
        .filter_map(|span| {
            let start = span.start.clamp(0.0, limit);
            let end = span.end.clamp(0.0, limit);
            (end - start > 1e-4).then_some(KerbSpan {
                start,
                end,
                side: if span.side >= 0 { 1 } else { -1 },
            })
        })
        .collect::<Vec<_>>();
    spans.sort_by(|a, b| {
        a.side
            .cmp(&b.side)
            .then_with(|| a.start.total_cmp(&b.start))
    });

    let mut last_end = [0.0f32; 2];
    let mut sanitized: Vec<KerbSpan> = Vec::with_capacity(spans.len());
    for span in spans {
        let side_index = usize::from(span.side > 0);
        let start = span.start.max(last_end[side_index]);
        if start < span.end {
            last_end[side_index] = span.end;
            sanitized.push(KerbSpan { start, ..span });
        }
    }
    sanitized
}

struct LegacyLayout {
    id: String,
    name: String,
    width: f32,
    control_points: Vec<[f32; 2]>,
    kerbs: Option<Vec<KerbSpan>>,
    prop_lines: Option<Vec<PropLine>>,
    start_finish: f32,
    spawns: Vec<SpawnPoint>,
    pit_boxes: Vec<SpawnPoint>,
    cameras: Vec<CameraSpec>,
    sectors: Vec<SectorSpec>,
}

struct LegacyProject {
    #[allow(dead_code)]
    name: String,
    layouts: Vec<LegacyLayout>,
    default_index: usize,
}

fn shared_start_finish_fraction(project: &TrackProject) -> f32 {
    let fraction = project.start_finish.fraction;
    if fraction.is_finite() {
        fraction.clamp(0.0, 0.999_999)
    } else {
        0.0
    }
}

/// Temporary B1 adapter from the venue graph to the legacy single-loop builder.
#[allow(dead_code)]
fn legacy_from_project(
    project: &TrackProject,
    default_layout: Option<&str>,
) -> Result<LegacyProject, String> {
    if project.layouts.is_empty() {
        return Err("track project needs at least one layout".to_string());
    }
    let width = global_asphalt_width(project);
    let start_finish = shared_start_finish_fraction(project);
    let pit_boxes = project
        .pit_lane
        .as_ref()
        .map(|pit_lane| pit_lane.boxes.clone())
        .unwrap_or_default();
    let mut layouts = Vec::with_capacity(project.layouts.len());
    for layout in &project.layouts {
        let control_points = layout_control_points(project, layout)?;
        let has_authored_props = layout.route.iter().any(|edge_id| {
            project
                .edges
                .iter()
                .any(|edge| edge.id == *edge_id && !edge.prop_spans.is_empty())
        });
        let prop_lines = if has_authored_props {
            layout_prop_lines(project, layout).ok()
        } else {
            let points = layout_control_points_with_heights(&control_points);
            let samples = sample_track_with_min_radius(&points, min_turn_radius(width));
            Some(default_prop_lines_outside_only(
                &samples,
                default_prop_offset(width),
                default_prop_far_offset(width),
            ))
        };
        layouts.push(LegacyLayout {
            id: layout.id.clone(),
            name: layout.name.clone(),
            width,
            control_points,
            kerbs: layout_kerb_spans(project, layout).ok(),
            prop_lines,
            start_finish,
            spawns: layout.spawns.clone(),
            pit_boxes: pit_boxes.clone(),
            cameras: layout.cameras.clone(),
            sectors: layout.sectors.clone(),
        });
    }
    let default_index = default_layout
        .and_then(|id| project.layout_index(id))
        .unwrap_or(0);
    Ok(LegacyProject {
        name: project.name.clone(),
        layouts,
        default_index,
    })
}

#[derive(Clone, Debug)]
struct BuiltEdge {
    id: String,
    points: Vec<Vec3>,
    length: f32,
    half_widths: Vec<f32>,
    outer_radii: Vec<f32>,
    surface_offsets: Vec<Vec<f32>>,
}

fn surface_splines(project: &TrackProject) -> Vec<SplineSpec> {
    project
        .settings
        .splines
        .iter()
        .filter(|spline| spline.role == SplineRole::Surface)
        .cloned()
        .collect()
}

fn surface_column_index(splines: &[SplineSpec], id: &str) -> Option<usize> {
    splines.iter().position(|spline| spline.id == id)
}

fn control_point_position(project: &TrackProject, id: &str) -> Result<Vec3, String> {
    let point = project
        .control_points
        .iter()
        .find(|point| point.id == id)
        .ok_or_else(|| format!("edge references unknown control point '{id}'"))?;
    Ok(Vec3::new(point.position[0], 0.0, point.position[1]))
}

/// Finds the control point on the far side of a node so Catmull-Rom can keep
/// tangent continuity. Fork nodes use a stable reflection of the edge chord;
/// junction pads and proper branch tangents are deferred to phase C.
fn edge_neighbor_position(
    project: &TrackProject,
    edge: &EdgeProject,
    node_id: &str,
    node_position: Vec3,
    edge_other: Vec3,
) -> Vec3 {
    let degree = project
        .edges
        .iter()
        .filter(|other| other.from == node_id || other.to == node_id)
        .count();
    if degree == 2 {
        if let Some(other) = project
            .edges
            .iter()
            .find(|other| other.id != edge.id && (other.from == node_id || other.to == node_id))
        {
            let far_id = if other.from == node_id {
                other.to.as_str()
            } else {
                other.from.as_str()
            };
            if let Ok(position) = control_point_position(project, far_id) {
                return position;
            }
        }
    }
    node_position + (node_position - edge_other)
}

fn resolved_edge_offsets(
    project: &TrackProject,
    edge: &EdgeProject,
    splines: &[SplineSpec],
) -> Result<(Vec<f32>, Vec<f32>), String> {
    let default_profile = Profile::default();
    let from_profile = edge
        .start_profile
        .as_ref()
        .or_else(|| {
            project
                .control_points
                .iter()
                .find(|point| point.id == edge.from)
                .and_then(|point| point.profile.as_ref())
        })
        .unwrap_or(&default_profile);
    let to_profile = edge
        .end_profile
        .as_ref()
        .or_else(|| {
            project
                .control_points
                .iter()
                .find(|point| point.id == edge.to)
                .and_then(|point| point.profile.as_ref())
        })
        .unwrap_or(&default_profile);
    let start_offsets = effective_profile_offsets(
        &project.settings.splines,
        from_profile,
        &format!("edge '{}' start profile", edge.id),
    )?;
    let end_offsets = effective_profile_offsets(
        &project.settings.splines,
        to_profile,
        &format!("edge '{}' end profile", edge.id),
    )?;
    Ok((
        splines
            .iter()
            .map(|spline| {
                start_offsets
                    .get(&spline.id)
                    .copied()
                    .unwrap_or(spline.offset)
            })
            .collect(),
        splines
            .iter()
            .map(|spline| {
                end_offsets
                    .get(&spline.id)
                    .copied()
                    .unwrap_or(spline.offset)
            })
            .collect(),
    ))
}

fn cubic_hermite(p0: Vec3, p1: Vec3, m0: Vec3, m1: Vec3, t: f32) -> Vec3 {
    let t2 = t * t;
    let t3 = t2 * t;
    let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
    let h10 = t3 - 2.0 * t2 + t;
    let h01 = -2.0 * t3 + 3.0 * t2;
    let h11 = t3 - t2;
    p0 * h00 + m0 * h10 + p1 * h01 + m1 * h11
}

fn tangent_vector(tangent: Option<[f32; 2]>) -> Option<Vec3> {
    let tangent = tangent?;
    let direction = Vec3::new(tangent[0], 0.0, tangent[1]);
    (direction.length_squared() > 1e-8).then(|| direction.normalize())
}

fn direction_between(from: Vec3, to: Vec3) -> Option<Vec3> {
    let direction = to - from;
    (direction.length_squared() > 1e-8).then(|| direction.normalize())
}

/// Samples one canonical segment from its own junction endpoints, interior
/// points, and endpoint tangents. The result never depends on which route owns
/// the segment.
fn sample_edge_points(project: &TrackProject, edge: &EdgeProject) -> Result<Vec<Vec3>, String> {
    let from_position = control_point_position(project, &edge.from)?;
    let to_position = control_point_position(project, &edge.to)?;
    let explicit_from = tangent_vector(edge.from_tangent);
    let explicit_to = tangent_vector(edge.to_tangent);

    if edge.points.is_empty() {
        // No interior points: one span between the junctions. Authored edge
        // tangents win; otherwise keep Catmull-Rom continuity with the
        // neighbouring edges.
        let previous =
            edge_neighbor_position(project, edge, &edge.from, from_position, to_position);
        let next = edge_neighbor_position(project, edge, &edge.to, to_position, from_position);
        let chord = from_position.distance(to_position).max(1.0);
        let from_tangent =
            explicit_from.unwrap_or_else(|| (to_position - previous).normalize_or_zero());
        let to_tangent = explicit_to.unwrap_or_else(|| (next - from_position).normalize_or_zero());
        let use_tangents = explicit_from.is_some() || explicit_to.is_some();
        let start_derivative = from_tangent * chord;
        let end_derivative = to_tangent * chord;

        let segments = ((chord / 3.0).ceil() as usize).clamp(4, 64);
        let mut points = Vec::with_capacity(segments + 1);
        for index in 0..=segments {
            let t = index as f32 / segments as f32;
            let mut point = if use_tangents {
                cubic_hermite(
                    from_position,
                    to_position,
                    start_derivative,
                    end_derivative,
                    t,
                )
            } else {
                catmull_rom(previous, from_position, to_position, next, t)
            };
            point.y = terrain_height(point.x, point.z);
            points.push(point);
        }
        return Ok(points);
    }

    // Interior points: build the edge's own control chain and derive a knot
    // tangent at every control point. Endpoint tangents come from the edge or
    // fall back to the nearest chain direction; interior knots use Catmull-Rom
    // continuity through their neighbours.
    let mut controls = Vec::with_capacity(edge.points.len() + 2);
    controls.push(from_position);
    controls.extend(
        edge.points
            .iter()
            .map(|point| Vec3::new(point[0], 0.0, point[1])),
    );
    controls.push(to_position);

    let last = controls.len() - 1;
    let mut tangents = Vec::with_capacity(controls.len());
    for index in 0..controls.len() {
        let tangent = if index == 0 {
            explicit_from.or_else(|| direction_between(controls[0], controls[1]))
        } else if index == last {
            explicit_to.or_else(|| direction_between(controls[last - 1], controls[last]))
        } else {
            direction_between(controls[index - 1], controls[index + 1])
        };
        let fallback = direction_between(
            controls[index.saturating_sub(1)],
            controls[(index + 1).min(last)],
        )
        .unwrap_or(Vec3::Z);
        tangents.push(tangent.unwrap_or(fallback));
    }

    let mut points = Vec::new();
    for span in 0..last {
        let p0 = controls[span];
        let p1 = controls[span + 1];
        let chord = p0.distance(p1).max(1.0);
        let m0 = tangents[span] * chord;
        let m1 = tangents[span + 1] * chord;
        let segments = ((chord / 3.0).ceil() as usize).clamp(2, 32);
        let first_step = usize::from(span > 0);
        for step in first_step..=segments {
            let t = step as f32 / segments as f32;
            let mut point = cubic_hermite(p0, p1, m0, m1, t);
            point.y = terrain_height(point.x, point.z);
            points.push(point);
        }
    }
    Ok(points)
}

fn sample_built_edge(project: &TrackProject, edge: &EdgeProject) -> Result<BuiltEdge, String> {
    let splines = surface_splines(project);
    let points = sample_edge_points(project, edge)?;
    let segments = points.len().saturating_sub(1);
    let length = points
        .windows(2)
        .map(|pair| pair[0].distance(pair[1]))
        .sum::<f32>();

    let (start_offsets, end_offsets) = resolved_edge_offsets(project, edge, &splines)?;
    let asphalt_left = surface_column_index(&splines, "asphalt-left");
    let asphalt_right = surface_column_index(&splines, "asphalt-right");
    let mut surface_offsets = Vec::with_capacity(points.len());
    let mut half_widths = Vec::with_capacity(points.len());
    let mut outer_radii = Vec::with_capacity(points.len());
    for index in 0..=segments {
        let t = index as f32 / segments as f32;
        let offsets = start_offsets
            .iter()
            .zip(end_offsets.iter())
            .map(|(start, end)| start + (end - start) * t)
            .collect::<Vec<_>>();
        let half_width = match (asphalt_left, asphalt_right) {
            (Some(left), Some(right)) => (offsets[right] - offsets[left]) * 0.5,
            _ => {
                (offsets.last().copied().unwrap_or(0.0) - offsets.first().copied().unwrap_or(0.0))
                    * 0.5
            }
        }
        .max(0.0);
        let outer = offsets
            .iter()
            .fold(0.0_f32, |outer, offset| outer.max(offset.abs()));
        half_widths.push(half_width);
        outer_radii.push(outer);
        surface_offsets.push(offsets);
    }

    Ok(BuiltEdge {
        id: edge.id.clone(),
        points,
        length,
        half_widths,
        outer_radii,
        surface_offsets,
    })
}

fn build_unique_edges(
    project: &TrackProject,
    ids: &BTreeSet<String>,
) -> Result<HashMap<String, BuiltEdge>, String> {
    let mut built = HashMap::with_capacity(ids.len());
    for id in ids {
        if built.contains_key(id) {
            continue;
        }
        let edge = project
            .edge_by_id(id)
            .ok_or_else(|| format!("unknown edge id '{id}'"))?;
        built.insert(id.clone(), sample_built_edge(project, edge)?);
    }
    Ok(built)
}

fn surface_id_for_name(name: &str) -> u16 {
    match name {
        "asphalt" => TRACK_SURFACE,
        "grass" => GRASS_SURFACE,
        "dirt" => DIRT_SURFACE,
        "kerb" => KERB_SURFACE,
        "wall" => WALL_SURFACE,
        _ => DIRT_SURFACE,
    }
}

fn band_surface_ids(splines: &[SplineSpec], offsets: &[f32]) -> Vec<u16> {
    (0..offsets.len().saturating_sub(1))
        .map(|band| {
            let left = &splines[band];
            let right = &splines[band + 1];
            let left_surface = left.surface.as_deref().unwrap_or("dirt");
            let right_surface = right.surface.as_deref().unwrap_or("dirt");
            let left_abs = offsets[band].abs();
            let right_abs = offsets[band + 1].abs();
            let surface = if (left_abs - right_abs).abs() <= 1e-5 {
                if left_surface == right_surface {
                    left_surface
                } else {
                    right_surface
                }
            } else if left_abs > right_abs {
                left_surface
            } else {
                right_surface
            };
            surface_id_for_name(surface)
        })
        .collect()
}

fn edge_ribbon(edge: &BuiltEdge, splines: &[SplineSpec]) -> (MeshBuffers, Vec<u16>) {
    let mut buffers = empty_buffers();
    let mut surface_ids = Vec::new();
    let count = edge.points.len();
    if count < 2 || splines.len() < 2 {
        return (buffers, surface_ids);
    }
    let segments = count - 1;
    let tiles_along = (edge.length / TARMAC_TILE_METERS).round().max(1.0);
    let mut distance = 0.0;
    for index in 0..=segments {
        let point = edge.points[index];
        let binormal = edge_binormal(&edge.points, index);
        let offsets = &edge.surface_offsets[index];
        let outer_left = offsets.first().copied().unwrap_or(0.0);
        let outer_right = offsets.last().copied().unwrap_or(0.0);
        let span = (outer_right - outer_left).max(1e-4);
        let u = if edge.length > 1e-4 {
            (distance / edge.length) * tiles_along
        } else {
            index as f32
        };
        for &offset in offsets {
            let lateral = point + binormal * offset;
            let y = terrain_height(lateral.x, lateral.z) + SURFACE_HEIGHT_OFFSET;
            buffers.positions.push([lateral.x, y, lateral.z]);
            buffers.normals.push(terrain_normal(lateral.x, lateral.z));
            let v = ((offset - outer_left) / span) * TARMAC_UV_ACROSS;
            buffers.uvs.push([u, v]);
        }
        if index < segments {
            let next_offsets = &edge.surface_offsets[index + 1];
            let midpoint = offsets
                .iter()
                .zip(next_offsets.iter())
                .map(|(start, end)| (start + end) * 0.5)
                .collect::<Vec<_>>();
            let ids = band_surface_ids(splines, &midpoint);
            for (band, surface) in ids.iter().enumerate() {
                let base = (index * offsets.len() + band) as u32;
                let next_base = ((index + 1) * offsets.len() + band) as u32;
                buffers.indices.extend_from_slice(&[
                    base,
                    base + 1,
                    next_base,
                    base + 1,
                    next_base + 1,
                    next_base,
                ]);
                surface_ids.extend_from_slice(&[*surface, *surface]);
            }
            distance += edge.points[index].distance(edge.points[index + 1]);
        }
    }
    (buffers, surface_ids)
}

fn edge_binormal(points: &[Vec3], index: usize) -> Vec3 {
    let segments = points.len().saturating_sub(1);
    if segments == 0 {
        return Vec3::Z;
    }
    let index = index.min(segments);
    let previous = points[index.saturating_sub(1)];
    let next = points[(index + 1).min(segments)];
    let tangent = (next - previous).normalize_or_zero();
    Vec3::new(-tangent.z, 0.0, tangent.x).normalize_or_zero()
}

fn edge_frame_at(points: &[Vec3], station: f32) -> (Vec3, Vec3) {
    let segments = points.len().saturating_sub(1);
    if segments == 0 {
        return (points.first().copied().unwrap_or(Vec3::ZERO), Vec3::Z);
    }
    let clamped = station.clamp(0.0, segments as f32);
    let base = clamped.floor() as usize;
    let t = clamped - base as f32;
    let sample = base.min(segments);
    let next = (sample + 1).min(segments);
    let point = points[sample].lerp(points[next], t);
    let binormal = edge_binormal(points, sample).lerp(edge_binormal(points, next), t);
    (point, binormal.normalize_or_zero())
}

fn station_value(values: &[f32], station: f32) -> f32 {
    let segments = values.len().saturating_sub(1);
    if segments == 0 {
        return values.first().copied().unwrap_or(0.0);
    }
    let clamped = station.clamp(0.0, segments as f32);
    let base = clamped.floor() as usize;
    let t = clamped - base as f32;
    let sample = base.min(segments);
    let next = (sample + 1).min(segments);
    values[sample] + (values[next] - values[sample]) * t
}

struct JunctionCrossSection {
    angle: f32,
    left: Vec3,
    right: Vec3,
}

fn node_angle(center: Vec3, point: Vec3) -> f32 {
    (point.z - center.z).atan2(point.x - center.x)
}

fn offset_asphalt_point(base: Vec3, binormal: Vec3, offset: f32) -> Vec3 {
    let lateral = base + binormal * offset;
    Vec3::new(
        lateral.x,
        terrain_height(lateral.x, lateral.z) + SURFACE_HEIGHT_OFFSET,
        lateral.z,
    )
}

fn push_junction_triangle(buffers: &mut MeshBuffers, center: Vec3, a: Vec3, b: Vec3) {
    let cross = (a - center).cross(b - center);
    if cross.length_squared() < 1e-8 {
        return;
    }
    let (a, b) = if cross.y < 0.0 { (b, a) } else { (a, b) };
    let base = buffers.positions.len() as u32;
    for corner in [center, a, b] {
        buffers.positions.push(corner.to_array());
        buffers.normals.push(terrain_normal(corner.x, corner.z));
        buffers
            .uvs
            .push([corner.x / TARMAC_TILE_METERS, corner.z / TARMAC_TILE_METERS]);
    }
    buffers
        .indices
        .extend_from_slice(&[base, base + 1, base + 2]);
}

/// Adds a small asphalt patch bridging two degree-2 endpoint cross-sections.
/// Cross-sections that already coincide (or coincide after swapping sides) are
/// skipped so smooth single-loop nodes do not gain redundant geometry.
fn push_degree_two_join(
    buffers: &mut MeshBuffers,
    first: &JunctionCrossSection,
    second: &JunctionCrossSection,
) {
    const JOIN_EPSILON: f32 = 0.05;
    let direct = first.left.distance(second.left) + first.right.distance(second.right);
    let swapped = first.left.distance(second.right) + first.right.distance(second.left);
    let (second_left, second_right) = if swapped < direct {
        (second.right, second.left)
    } else {
        (second.left, second.right)
    };
    if first.left.distance(second_left) <= JOIN_EPSILON
        && first.right.distance(second_right) <= JOIN_EPSILON
    {
        return;
    }
    let center = (first.left + first.right + second_left + second_right) * 0.25;
    let mut boundary = [first.left, first.right, second_left, second_right];
    boundary.sort_by(|a, b| node_angle(center, *a).total_cmp(&node_angle(center, *b)));
    for index in 0..boundary.len() {
        push_junction_triangle(
            buffers,
            center,
            boundary[index],
            boundary[(index + 1) % boundary.len()],
        );
    }
}

/// Builds a basic asphalt pad for every junction node. Each incident edge
/// contributes its asphalt cross-section at the node; the ordered boundary is
/// fanned around the node centre. Degree-2 nodes with distinct cross-sections
/// get a small join strip so shared-loop joins stay closed.
fn build_junction_pads(
    project: &TrackProject,
    built: &HashMap<String, BuiltEdge>,
    splines: &[SplineSpec],
) -> MeshBuffers {
    let mut buffers = empty_buffers();
    let (Some(asphalt_left), Some(asphalt_right)) = (
        surface_column_index(splines, "asphalt-left"),
        surface_column_index(splines, "asphalt-right"),
    ) else {
        return buffers;
    };
    let mut incident = BTreeMap::<&str, Vec<&EdgeProject>>::new();
    for edge in &project.edges {
        incident.entry(edge.from.as_str()).or_default().push(edge);
        incident.entry(edge.to.as_str()).or_default().push(edge);
    }
    for point in &project.control_points {
        let Some(edges) = incident.get(point.id.as_str()) else {
            continue;
        };
        if edges.len() < 2 {
            continue;
        }
        let center = Vec3::new(
            point.position[0],
            terrain_height(point.position[0], point.position[1]) + SURFACE_HEIGHT_OFFSET,
            point.position[1],
        );
        let mut sections = Vec::with_capacity(edges.len());
        for edge in edges {
            let Some(built_edge) = built.get(&edge.id) else {
                continue;
            };
            let (station, away) = if edge.from == point.id {
                if built_edge.points.len() < 2 {
                    continue;
                }
                (0, built_edge.points[1] - built_edge.points[0])
            } else if edge.to == point.id {
                let last = built_edge.points.len().saturating_sub(1);
                if last == 0 {
                    continue;
                }
                (last, built_edge.points[last - 1] - built_edge.points[last])
            } else {
                continue;
            };
            let offsets = &built_edge.surface_offsets[station];
            let (Some(left_offset), Some(right_offset)) =
                (offsets.get(asphalt_left), offsets.get(asphalt_right))
            else {
                continue;
            };
            let base = built_edge.points[station];
            let binormal = edge_binormal(&built_edge.points, station);
            sections.push(JunctionCrossSection {
                angle: away.z.atan2(away.x),
                left: offset_asphalt_point(base, binormal, *left_offset),
                right: offset_asphalt_point(base, binormal, *right_offset),
            });
        }
        if edges.len() == 2 {
            if let [first, second] = sections.as_slice() {
                push_degree_two_join(&mut buffers, first, second);
            }
            continue;
        }
        if sections.len() < 3 {
            continue;
        }
        sections.sort_by(|first, second| first.angle.total_cmp(&second.angle));
        let mut boundary = sections
            .iter()
            .flat_map(|section| [section.left, section.right])
            .collect::<Vec<_>>();
        boundary.sort_by(|first, second| {
            node_angle(center, *first).total_cmp(&node_angle(center, *second))
        });
        let count = boundary.len();
        if count < 3 {
            continue;
        }
        for index in 0..count {
            push_junction_triangle(
                &mut buffers,
                center,
                boundary[index],
                boundary[(index + 1) % count],
            );
        }
    }
    buffers
}

#[allow(clippy::too_many_arguments)]
fn append_kerb_with_widths(
    buffers: &mut MeshBuffers,
    start: Vec3,
    end: Vec3,
    start_binormal: Vec3,
    end_binormal: Vec3,
    side: f32,
    inner_start: f32,
    inner_end: f32,
    width: f32,
    depth: f32,
) -> ([Vec3; 2], [Vec3; 2]) {
    let outer_start_offset = inner_start + width.max(0.0);
    let outer_end_offset = inner_end + width.max(0.0);
    let inner_start = start + start_binormal * (inner_start * side);
    let inner_end = end + end_binormal * (inner_end * side);
    let outer_start = start + start_binormal * (outer_start_offset * side);
    let outer_end = end + end_binormal * (outer_end_offset * side);
    let inner_base_start = terrain_height(inner_start.x, inner_start.z);
    let inner_base_end = terrain_height(inner_end.x, inner_end.z);
    let outer_base_start = terrain_height(outer_start.x, outer_start.z);
    let outer_base_end = terrain_height(outer_end.x, outer_end.z);
    let inner_top_start = Vec3::new(
        inner_start.x,
        inner_base_start + KERB_SURFACE_OFFSET,
        inner_start.z,
    );
    let inner_top_end = Vec3::new(
        inner_end.x,
        inner_base_end + KERB_SURFACE_OFFSET,
        inner_end.z,
    );
    let top_offset = KERB_BOTTOM_OFFSET + depth.max(0.01);
    let outer_top_start = Vec3::new(outer_start.x, outer_base_start + top_offset, outer_start.z);
    let outer_top_end = Vec3::new(outer_end.x, outer_base_end + top_offset, outer_end.z);
    let outer_bottom_start = Vec3::new(
        outer_start.x,
        outer_base_start + KERB_BOTTOM_OFFSET,
        outer_start.z,
    );
    let outer_bottom_end = Vec3::new(
        outer_end.x,
        outer_base_end + KERB_BOTTOM_OFFSET,
        outer_end.z,
    );
    let side_normal = (start_binormal + end_binormal).normalize_or_zero() * side;
    let along = (end - start).normalize_or_zero();
    let across = (outer_top_start - inner_top_start).normalize_or_zero();
    let mut top_normal = across.cross(along).normalize_or_zero();
    if top_normal.y < 0.0 {
        top_normal = -top_normal;
    }
    push_quad(
        buffers,
        [
            inner_top_start.to_array(),
            outer_top_start.to_array(),
            outer_top_end.to_array(),
            inner_top_end.to_array(),
        ],
        top_normal.to_array(),
    );
    push_quad(
        buffers,
        [
            outer_top_start.to_array(),
            outer_top_end.to_array(),
            outer_bottom_end.to_array(),
            outer_bottom_start.to_array(),
        ],
        side_normal.to_array(),
    );
    (
        [inner_top_start, outer_top_start],
        [inner_top_end, outer_top_end],
    )
}

fn build_edge_kerbs(
    edge: &BuiltEdge,
    kerbs: &[KerbSpan],
    settings: &KerbSettings,
    red: &mut MeshBuffers,
    white: &mut MeshBuffers,
) {
    let segments = edge.points.len().saturating_sub(1);
    if segments == 0 {
        return;
    }
    for span in kerbs {
        let start = span.start.clamp(0.0, 1.0) * segments as f32;
        let end = span.end.clamp(0.0, 1.0) * segments as f32;
        if end - start <= 1e-4 {
            continue;
        }
        let mut previous_end = None;
        for index in start.floor() as usize..end.ceil() as usize {
            let segment_start = (index as f32).max(start);
            let segment_end = ((index + 1) as f32).min(end);
            if segment_end - segment_start <= 1e-4 {
                continue;
            }
            let (point, binormal) = edge_frame_at(&edge.points, segment_start);
            let (next, next_binormal) = edge_frame_at(&edge.points, segment_end);
            let buffers = if index % 2 == 0 {
                &mut *red
            } else {
                &mut *white
            };
            let (start_edge, end_edge) = append_kerb_with_widths(
                buffers,
                point,
                next,
                binormal,
                next_binormal,
                span.side as f32,
                station_value(&edge.half_widths, segment_start),
                station_value(&edge.half_widths, segment_end),
                settings.width,
                settings.depth,
            );
            if let Some(previous_end) = previous_end {
                append_kerb_riser(buffers, previous_end, start_edge, next - point);
            }
            previous_end = Some(end_edge);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn build_edge_props(
    project: &TrackProject,
    edge: &BuiltEdge,
    splines: &[SplineSpec],
    barriers: &mut MeshBuffers,
    tires_blue: &mut MeshBuffers,
    tires_white: &mut MeshBuffers,
    tree_trunks_high: &mut MeshBuffers,
    tree_foliage_high: &mut MeshBuffers,
    tree_trunks_low: &mut MeshBuffers,
    tree_foliage_low: &mut MeshBuffers,
    tree_cards: &mut MeshBuffers,
    prop_walls: &mut MeshBuffers,
    prop_spans: &[PropSpan],
) -> Vec<PropLine> {
    let segments = edge.points.len().saturating_sub(1) as f32;
    let mut lines = Vec::new();
    for span in prop_spans {
        let Some(spline) = project
            .settings
            .splines
            .iter()
            .find(|spline| spline.id == span.spline && spline.role == SplineRole::Prop)
        else {
            continue;
        };
        let base_id = spline.base.as_deref().unwrap_or("asphalt-right");
        let Some(base_column) = surface_column_index(splines, base_id) else {
            continue;
        };
        let mid_station = (span.start + span.end) * 0.5 * segments;
        let base_offset = edge
            .surface_offsets
            .get(mid_station.round() as usize)
            .and_then(|offsets| offsets.get(base_column))
            .copied()
            .unwrap_or(0.0);
        let lateral = base_offset + spline.offset;
        lines.push(PropLine {
            start: span.start * segments,
            end: span.end * segments,
            side: if lateral >= 0.0 { 1 } else { -1 },
            offset: lateral.abs().max(MIN_PROP_OFFSET),
            kind: span.kind,
        });
    }
    append_prop_lines(
        &edge.points,
        &lines,
        barriers,
        tires_blue,
        tires_white,
        tree_trunks_high,
        tree_foliage_high,
        tree_trunks_low,
        tree_foliage_low,
        tree_cards,
    );
    append_prop_walls(&edge.points, &lines, prop_walls);
    lines
}

struct RouteBuild {
    points: Vec<Vec3>,
    half_widths: Vec<f32>,
    length: f32,
    start_finish: f32,
    kerb_spans: Vec<KerbSpan>,
    prop_lines: Vec<PropLine>,
    /// For each route edge, the inclusive sample range it occupies in
    /// `points`. The end may equal `points.len()` for the final edge whose
    /// duplicate closing sample was removed.
    edge_ranges: Vec<(String, usize, usize)>,
}

fn assemble_route(
    project: &TrackProject,
    layout: &LayoutProject,
    built: &HashMap<String, BuiltEdge>,
    edge_prop_lines: &HashMap<String, Vec<PropLine>>,
) -> Result<RouteBuild, String> {
    let mut points = Vec::new();
    let mut half_widths = Vec::new();
    let mut kerb_spans = Vec::new();
    let mut prop_lines = Vec::new();
    let mut edge_ranges = Vec::new();
    let mut length = 0.0_f32;
    let mut preceding = 0.0_f32;
    let mut start_finish_distance = None;
    for edge_id in &layout.route {
        let edge = built
            .get(edge_id)
            .ok_or_else(|| format!("layout '{}' route has no edge '{edge_id}'", layout.id))?;
        debug_assert_eq!(edge.id, *edge_id);
        let segments = edge.points.len().saturating_sub(1) as f32;
        let station_offset = (points.len() as f32) - if points.is_empty() { 0.0 } else { 1.0 };
        if edge_id == &layout.start_finish.edge {
            let fraction = layout.start_finish.fraction.clamp(0.0, 0.999_999);
            start_finish_distance = Some(preceding + edge.length * fraction);
        }
        let route_edge = project
            .edge_by_id(edge_id)
            .ok_or_else(|| format!("layout '{}' route has no edge '{edge_id}'", layout.id))?;
        for span in &route_edge.kerbs {
            kerb_spans.push(KerbSpan {
                start: station_offset + span.start * segments,
                end: station_offset + span.end * segments,
                side: span.side,
            });
        }
        if let Some(lines) = edge_prop_lines.get(edge_id) {
            for line in lines {
                prop_lines.push(PropLine {
                    start: station_offset + line.start,
                    end: station_offset + line.end,
                    ..*line
                });
            }
        }
        let range_start = if points.is_empty() {
            0
        } else {
            points.len() - 1
        };
        if points.is_empty() {
            points.extend_from_slice(&edge.points);
            half_widths.extend_from_slice(&edge.half_widths);
        } else {
            points.extend_from_slice(&edge.points[1..]);
            half_widths.extend_from_slice(&edge.half_widths[1..]);
        }
        edge_ranges.push((edge_id.clone(), range_start, points.len().saturating_sub(1)));
        length += edge.length;
        preceding += edge.length;
    }
    if points.len() > 1 {
        points.pop();
        half_widths.pop();
    }
    if points.len() < 3 {
        return Err(format!(
            "layout '{}' needs at least three route samples",
            layout.id
        ));
    }
    for (_, start, end) in &mut edge_ranges {
        *start = (*start).min(points.len());
        *end = (*end).min(points.len());
    }
    let start_finish = if length > 1e-4 {
        start_finish_distance.unwrap_or(0.0) / length
    } else {
        0.0
    }
    .clamp(0.0, 0.999_999);
    let count = points.len() as f32;
    Ok(RouteBuild {
        points,
        half_widths,
        length,
        start_finish,
        kerb_spans: sanitize_kerb_spans(&kerb_spans, count as usize),
        prop_lines: sanitize_prop_lines_with_limit(&prop_lines, count as usize, MAX_PROP_OFFSET),
        edge_ranges,
    })
}

/// Builds the layout route and returns its edge sample ranges plus its route
/// point count. Used to map between route-local sample indices and per-edge
/// fractions.
#[allow(clippy::type_complexity)]
fn layout_route_ranges(
    project: &TrackProject,
    layout: &LayoutProject,
) -> Result<(Vec<(String, usize, usize)>, usize), String> {
    let edge_ids = layout.route.iter().cloned().collect::<BTreeSet<_>>();
    let built = build_unique_edges(project, &edge_ids)?;
    let route = assemble_route(project, layout, &built, &HashMap::new())?;
    Ok((route.edge_ranges, route.points.len()))
}

/// How many route samples around an edge's own range to ignore when checking
/// synthesized props against the route centerline. This keeps the prop's own
/// edge and its shared junction samples from rejecting valid placements.
const PROP_ROUTE_CLEARANCE_MARGIN: usize = 2;

/// Computes world-space points at the start, midpoint, and end of a prop span
/// using the same lateral offset calculation as `build_edge_props`.
fn prop_span_world_points(
    edge: &BuiltEdge,
    splines: &[SplineSpec],
    spline: &SplineSpec,
    span: &PropSpan,
) -> Option<[Vec3; 3]> {
    let segments = edge.points.len().saturating_sub(1) as f32;
    if segments <= 1e-6 {
        return None;
    }
    let base_id = spline.base.as_deref().unwrap_or("asphalt-right");
    let base_column = surface_column_index(splines, base_id)?;
    let mid_station = (span.start + span.end) * 0.5 * segments;
    let base_offset = edge
        .surface_offsets
        .get(mid_station.round() as usize)
        .and_then(|offsets| offsets.get(base_column))
        .copied()
        .unwrap_or(0.0);
    let lateral = base_offset + spline.offset;
    let side = if lateral >= 0.0 { 1.0 } else { -1.0 };
    let offset = lateral.abs().max(MIN_PROP_OFFSET) * side;
    let mut points = [Vec3::ZERO; 3];
    for (point, fraction) in
        points
            .iter_mut()
            .zip([span.start, (span.start + span.end) * 0.5, span.end])
    {
        let (sample, binormal) = edge_frame_at(&edge.points, fraction * segments);
        *point = sample + binormal * offset;
    }
    Some(points)
}

/// Returns true when every prop point stays at least `clearance` away from
/// route centerline samples outside the owning edge's sample range.
fn prop_points_clear_of_route(
    points: &[Vec3; 3],
    route_points: &[Vec3],
    edge_start: usize,
    edge_end: usize,
    clearance: f32,
) -> bool {
    if route_points.is_empty() {
        return true;
    }
    let skip_start = edge_start.saturating_sub(PROP_ROUTE_CLEARANCE_MARGIN);
    let skip_end =
        (edge_end + PROP_ROUTE_CLEARANCE_MARGIN).min(route_points.len().saturating_sub(1));
    let clearance_sq = clearance * clearance;
    for (index, route_point) in route_points.iter().enumerate() {
        if index >= skip_start && index <= skip_end {
            continue;
        }
        for point in points {
            let delta = *point - *route_point;
            if delta.x * delta.x + delta.z * delta.z < clearance_sq {
                return false;
            }
        }
    }
    true
}

/// Synthesizes the default prop layout once over the assembled route, then
/// splits each route-level line into edge-local spans. This keeps the outside
/// side consistent around the whole loop instead of recalculating it per edge.
/// Spans whose world placement would cross the route centerline elsewhere on a
/// concave loop are rejected so barriers and trees stay clear of the tarmac.
fn synthesize_default_prop_spans(
    project: &TrackProject,
    route: &RouteBuild,
    built: &HashMap<String, BuiltEdge>,
    splines: &[SplineSpec],
    width: f32,
) -> HashMap<String, Vec<PropSpan>> {
    // Default synthesis only decorates the outside of the loop; the inside
    // stays clear so the infield does not get pinched by the tyre ring.
    let outside_side = route_outside_side(&route.points);
    let lines = default_prop_lines_with_offsets(
        &route.points,
        default_prop_offset(width),
        default_prop_far_offset(width),
    )
    .into_iter()
    .filter(|line| line.side == outside_side);
    let mut spans_by_edge = HashMap::<String, Vec<PropSpan>>::new();
    for line in lines {
        let Some(spline) = select_prop_spline_id(&project.settings.splines, &line, width) else {
            continue;
        };
        let Some(spline_spec) = project
            .settings
            .splines
            .iter()
            .find(|spec| spec.id == spline && spec.role == SplineRole::Prop)
        else {
            continue;
        };
        for (edge_id, range_start, range_end) in &route.edge_ranges {
            let range_start_f = *range_start as f32;
            let range_end_f = *range_end as f32;
            let range_length = range_end_f - range_start_f;
            if range_length <= 1e-6 {
                continue;
            }
            let overlap_start = line.start.max(range_start_f);
            let overlap_end = line.end.min(range_end_f);
            if overlap_end - overlap_start <= 1e-4 {
                continue;
            }
            let start = ((overlap_start - range_start_f) / range_length).clamp(0.0, 1.0);
            let end = ((overlap_end - range_start_f) / range_length).clamp(0.0, 1.0);
            if end - start <= 1e-4 {
                continue;
            }
            let span = PropSpan {
                spline: spline.clone(),
                start,
                end,
                kind: line.kind,
            };
            let Some(edge) = built.get(edge_id) else {
                continue;
            };
            let Some(points) = prop_span_world_points(edge, splines, spline_spec, &span) else {
                continue;
            };
            if !prop_points_clear_of_route(&points, &route.points, *range_start, *range_end, width)
            {
                continue;
            }
            spans_by_edge.entry(edge_id.clone()).or_default().push(span);
        }
    }
    spans_by_edge
}

fn terrain_grid_for_built_edges(edges: impl Iterator<Item = BuiltEdge>) -> MeshBuffers {
    let stations = edges
        .flat_map(|edge| {
            edge.points
                .into_iter()
                .zip(edge.outer_radii)
                .map(|(point, radius)| (point, (radius - 2.5).max(0.0)))
        })
        .collect::<Vec<_>>();
    let mut buffers = empty_buffers();
    let half = TERRAIN_SIZE * 0.5;
    let step = TERRAIN_SIZE / TERRAIN_CELLS as f32;
    let uv_tiles = TERRAIN_SIZE / TERRAIN_TILE_METERS;
    for iz in 0..=TERRAIN_CELLS {
        for ix in 0..=TERRAIN_CELLS {
            let x = -half + ix as f32 * step;
            let z = -half + iz as f32 * step;
            buffers.positions.push([x, terrain_height(x, z), z]);
            buffers.normals.push(terrain_normal(x, z));
            buffers.uvs.push([
                ix as f32 / TERRAIN_CELLS as f32 * uv_tiles,
                iz as f32 / TERRAIN_CELLS as f32 * uv_tiles,
            ]);
        }
    }
    let row = TERRAIN_CELLS + 1;
    for iz in 0..TERRAIN_CELLS {
        for ix in 0..TERRAIN_CELLS {
            let cell_x = -half + (ix as f32 + 0.5) * step;
            let cell_z = -half + (iz as f32 + 0.5) * step;
            let under_track = stations.iter().any(|(point, radius)| {
                let dx = point.x - cell_x;
                let dz = point.z - cell_z;
                dx * dx + dz * dz < radius * radius
            });
            if under_track {
                continue;
            }
            let base = (iz * row + ix) as u32;
            let next = base + row as u32;
            buffers
                .indices
                .extend_from_slice(&[base, next, next + 1, base, next + 1, base + 1]);
        }
    }
    buffers
}

fn track_from_graph(project: TrackProject, default_index: usize) -> Result<EditorTrack, String> {
    if project.layouts.is_empty() {
        return Err("track project needs at least one layout".to_string());
    }
    let default_index = default_index.min(project.layouts.len() - 1);
    let splines = surface_splines(&project);

    let mut layout_edge_ids = BTreeSet::new();
    for layout in &project.layouts {
        for edge_id in &layout.route {
            layout_edge_ids.insert(edge_id.clone());
        }
    }
    let mut terrain_edge_ids = layout_edge_ids.clone();
    if let Some(pit_lane) = &project.pit_lane {
        for edge_id in &pit_lane.route {
            terrain_edge_ids.insert(edge_id.clone());
        }
    }

    let built = build_unique_edges(&project, &terrain_edge_ids)?;

    let mut visual = TrackVisual {
        meshes: Vec::new(),
        materials: materials(),
        textures: vec![conifer_card_texture(), tarmac_texture(), grass_texture()],
        mesh_buffers: Vec::new(),
        instances: Vec::new(),
    };
    let mut collision = TrackCollision {
        vertices: Vec::new(),
        indices: Vec::new(),
        surface_ids: Vec::new(),
        bounds: Aabb {
            min: [0.0; 3],
            max: [0.0; 3],
        },
    };

    let mut asphalt = empty_buffers();
    let mut kerb_red = empty_buffers();
    let mut kerb_white = empty_buffers();
    let mut barriers = empty_buffers();
    let mut tires_blue = empty_buffers();
    let mut tires_white = empty_buffers();
    let mut tree_trunks_high = empty_buffers();
    let mut tree_foliage_high = empty_buffers();
    let mut tree_trunks_low = empty_buffers();
    let mut tree_foliage_low = empty_buffers();
    let mut tree_cards = empty_buffers();
    let mut prop_walls = empty_buffers();
    let mut edge_prop_lines = HashMap::<String, Vec<PropLine>>::new();
    let width = global_asphalt_width(&project);
    let has_kerbs = project
        .edges
        .iter()
        .any(|edge| layout_edge_ids.contains(&edge.id) && !edge.kerbs.is_empty());
    let has_props = project
        .edges
        .iter()
        .any(|edge| layout_edge_ids.contains(&edge.id) && !edge.prop_spans.is_empty());
    let synthesized_prop_spans = if has_props {
        HashMap::new()
    } else {
        let default_layout = &project.layouts[default_index];
        let default_route = assemble_route(&project, default_layout, &built, &HashMap::new())?;
        synthesize_default_prop_spans(&project, &default_route, &built, &splines, width)
    };

    for edge_id in &layout_edge_ids {
        let edge = built
            .get(edge_id)
            .ok_or_else(|| format!("missing built edge '{edge_id}'"))?;
        let project_edge = project
            .edge_by_id(edge_id)
            .ok_or_else(|| format!("unknown edge id '{edge_id}'"))?;
        let segment_count = edge.points.len().saturating_sub(1).max(1) as f32;
        let (buffers, surface_ids) = edge_ribbon(edge, &splines);
        append_collision_surfaces(&mut collision, &buffers, |triangle| {
            surface_ids.get(triangle).copied().unwrap_or(TRACK_SURFACE)
        });
        append_buffers(&mut asphalt, &buffers);
        let kerbs = if has_kerbs {
            project_edge.kerbs.clone()
        } else {
            auto_kerb_spans(&edge.points)
                .into_iter()
                .map(|span| KerbSpan {
                    start: span.start / segment_count,
                    end: span.end / segment_count,
                    side: span.side,
                })
                .collect()
        };
        build_edge_kerbs(
            edge,
            &kerbs,
            &project.settings.kerb,
            &mut kerb_red,
            &mut kerb_white,
        );
        let prop_spans = if has_props {
            project_edge.prop_spans.clone()
        } else {
            synthesized_prop_spans
                .get(edge_id)
                .cloned()
                .unwrap_or_default()
        };
        let lines = build_edge_props(
            &project,
            edge,
            &splines,
            &mut barriers,
            &mut tires_blue,
            &mut tires_white,
            &mut tree_trunks_high,
            &mut tree_foliage_high,
            &mut tree_trunks_low,
            &mut tree_foliage_low,
            &mut tree_cards,
            &mut prop_walls,
            &prop_spans,
        );
        edge_prop_lines.insert(edge_id.clone(), lines);
    }

    let junction_pads = build_junction_pads(&project, &built, &splines);
    if !junction_pads.positions.is_empty() {
        append_collision(&mut collision, &junction_pads, TRACK_SURFACE);
        append_buffers(&mut asphalt, &junction_pads);
    }

    let terrain = terrain_grid_for_built_edges(built.values().cloned());
    let mountains = mountain_grid();

    let default_layout = &project.layouts[default_index];
    let default_route = assemble_route(&project, default_layout, &built, &edge_prop_lines)?;
    let half_widths = &default_route.half_widths;
    let offsets = racing_line_offsets_with_half_widths(&default_route.points, half_widths);
    let outer = half_widths
        .iter()
        .copied()
        .fold(RACING_LINE_HALF_WIDTH, f32::max);
    let racing_line =
        racing_line_ribbon_with_half_widths(&default_route.points, &offsets, half_widths, outer);

    push_mesh(&mut visual, "asphalt_dither", 1, asphalt, false);
    push_mesh(
        &mut visual,
        "racing_line",
        RACING_LINE_MATERIAL,
        racing_line,
        false,
    );
    push_mesh(&mut visual, "terrain", 2, terrain, false);
    push_mesh(&mut visual, "mountains", 8, mountains, false);
    push_mesh(&mut visual, "kerb_red", 3, kerb_red, false);
    push_mesh(&mut visual, "kerb_white", 4, kerb_white, false);
    push_mesh(&mut visual, "barriers", 5, barriers, true);
    push_mesh(&mut visual, "tires_blue", 6, tires_blue, true);
    push_mesh(&mut visual, "tires_white", 7, tires_white, true);
    append_collision(&mut collision, &tree_trunks_low, WALL_SURFACE);
    push_mesh(&mut visual, "tree_trunks_high", 9, tree_trunks_high, true);
    push_mesh(
        &mut visual,
        "tree_foliage_high",
        10,
        tree_foliage_high,
        true,
    );
    push_mesh(&mut visual, "tree_trunks_low", 9, tree_trunks_low, true);
    push_mesh(&mut visual, "tree_foliage_low", 10, tree_foliage_low, true);
    push_mesh(&mut visual, "tree_cards", 11, tree_cards, false);

    for (index, mesh) in visual.meshes.iter().enumerate() {
        let surface = match mesh.name.as_str() {
            "terrain" => GRASS_SURFACE,
            "kerb_red" | "kerb_white" => KERB_SURFACE,
            _ => continue,
        };
        append_collision(&mut collision, &visual.mesh_buffers[index], surface);
    }
    append_collision(&mut collision, &prop_walls, WALL_SURFACE);
    collision.bounds = collision_bounds(&collision.vertices);

    let pit_boxes = project
        .pit_lane
        .as_ref()
        .map(|pit_lane| pit_lane.boxes.clone())
        .unwrap_or_default();
    let mut layout_specs = Vec::with_capacity(project.layouts.len());
    for layout in &project.layouts {
        let route = assemble_route(&project, layout, &built, &edge_prop_lines)?;
        let spawns = if layout.spawns.is_empty() {
            vec![spawn_for_layout(&route.points, route.start_finish)]
        } else {
            layout.spawns.clone()
        };
        let cameras = if layout.cameras.is_empty() {
            vec![overview_camera_for_layout(&route.points)]
        } else {
            layout.cameras.clone()
        };
        let sectors = if layout.sectors.is_empty() {
            vec![SectorSpec {
                name: "loop".to_string(),
                start_fraction: 0.0,
                end_fraction: 1.0,
            }]
        } else {
            layout.sectors.clone()
        };
        layout_specs.push(LayoutSpec {
            id: layout.id.clone(),
            name: layout.name.clone(),
            length: route.length,
            start_finish: route.start_finish,
            spawns,
            pit_boxes: pit_boxes.clone(),
            cameras,
            sectors,
        });
    }

    let manifest_id = project.layouts[default_index].id.clone();
    let metadata = TrackMetadata {
        name: String::new(),
        length: 0.0,
        spawns: Vec::new(),
        pit_boxes: Vec::new(),
        cameras: Vec::new(),
        sectors: Vec::new(),
        surfaces: track_surfaces(),
        layouts: layout_specs,
        default_layout: default_index as u32,
    }
    .normalize_layouts();

    let bounds = visual_bounds(&visual)?;
    let vertices = visual
        .mesh_buffers
        .iter()
        .map(|buffers| buffers.positions.len() as u32)
        .sum();
    let triangles = visual
        .mesh_buffers
        .iter()
        .map(|buffers| (buffers.indices.len() / 3) as u32)
        .sum();
    let manifest = AssetManifest {
        magic: MAGIC,
        schema_version: SCHEMA_VERSION,
        kind: AssetKind::Track,
        id: manifest_id,
        source_hash: [0; 32],
        units: ENGINE_UNITS,
        frame: ENGINE_FRAME,
        bounds,
        chunks: Vec::new(),
        entrypoints: EntryPoints {
            physics: None,
            visual: None,
            collision: None,
            metadata: None,
            audio: None,
            texture: None,
        },
        stats: AssetStats {
            vertices,
            triangles,
            materials: visual.materials.len() as u16,
            textures: visual.textures.len() as u16,
            draw_calls: visual.meshes.len() as u16,
        },
        warnings: vec!["procedural track editor preview".to_string()],
    };

    Ok(EditorTrack {
        visual,
        metadata,
        collision,
        manifest,
        kerb_spans: default_route.kerb_spans,
        prop_lines: default_route.prop_lines,
        project,
    })
}

#[allow(dead_code)]
pub fn track_from_control_points_with_spans(
    control_points: &[[f32; 3]],
    kerb_spans: Option<&[KerbSpan]>,
    prop_lines: Option<&[PropLine]>,
) -> Result<EditorTrack, String> {
    if control_points.len() < 3 {
        return Err("procedural track needs at least three control points".to_string());
    }
    let points = sample_track(control_points);
    if points.len() < 3 {
        return Err("procedural track needs at least three samples".to_string());
    }
    let flat_points = control_points
        .iter()
        .map(|point| [point[0], point[2]])
        .collect::<Vec<_>>();
    let mut project = TrackProject::ring_project(
        "Procedural Drift Loop",
        "procedural-drift-loop",
        &flat_points,
    );
    let count = points.len().max(1) as f32;
    if let Some(first_edge) = project.edges.first_mut() {
        if let Some(spans) = kerb_spans {
            first_edge.kerbs = spans
                .iter()
                .filter_map(|span| {
                    let start = (span.start / count).clamp(0.0, 1.0);
                    let end = (span.end / count).clamp(0.0, 1.0);
                    (end - start > 1e-4).then_some(KerbSpan {
                        start,
                        end,
                        side: span.side,
                    })
                })
                .collect();
        }
    }
    let legacy = LegacyProject {
        name: project.name.clone(),
        layouts: vec![LegacyLayout {
            id: project.layouts[0].id.clone(),
            name: project.layouts[0].name.clone(),
            width: global_asphalt_width(&project),
            control_points: flat_points,
            kerbs: kerb_spans.map(|spans| spans.to_vec()),
            prop_lines: prop_lines.map(|lines| lines.to_vec()),
            start_finish: 0.0,
            spawns: Vec::new(),
            pit_boxes: Vec::new(),
            cameras: Vec::new(),
            sectors: Vec::new(),
        }],
        default_index: 0,
    };
    track_from_legacy(project, legacy)
}

struct LayoutSamples {
    points: Vec<Vec3>,
    kerb_spans: Vec<KerbSpan>,
    prop_lines: Vec<PropLine>,
}

fn track_surfaces() -> Vec<SurfaceSpec> {
    vec![
        SurfaceSpec {
            id: TRACK_SURFACE,
            name: "asphalt".to_string(),
            friction: 0.95,
            roughness: 0.8,
        },
        SurfaceSpec {
            id: GRASS_SURFACE,
            name: "grass".to_string(),
            friction: 0.55,
            roughness: 1.0,
        },
        SurfaceSpec {
            id: DIRT_SURFACE,
            name: "dirt".to_string(),
            friction: 0.7,
            roughness: 1.0,
        },
        SurfaceSpec {
            id: KERB_SURFACE,
            name: "kerb".to_string(),
            friction: 0.9,
            roughness: 0.2,
        },
        SurfaceSpec {
            id: WALL_SURFACE,
            name: "wall".to_string(),
            friction: 0.8,
            roughness: 1.0,
        },
    ]
}

fn line_length(points: &[Vec3]) -> f32 {
    points
        .iter()
        .enumerate()
        .map(|(index, point)| point.distance(points[(index + 1) % points.len()]))
        .sum()
}

fn spawn_for_layout(points: &[Vec3], start_finish: f32) -> SpawnPoint {
    let count = points.len();
    let index = ((start_finish.clamp(0.0, 1.0) * count as f32).round() as usize) % count;
    let current = points[index];
    let next = points[(index + 1) % count];
    let tangent = (next - current).normalize_or_zero();
    let rotation = if tangent.length_squared() > 1e-6 {
        Quat::from_rotation_y(PI) * Quat::from_rotation_arc(Vec3::NEG_Z, tangent)
    } else {
        Quat::IDENTITY
    };
    SpawnPoint {
        name: "spawn".to_string(),
        position: current.to_array(),
        rotation_xyzw: rotation.to_array(),
    }
}

fn overview_camera_for_layout(points: &[Vec3]) -> CameraSpec {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for point in points {
        min = min.min(*point);
        max = max.max(*point);
    }
    let center = (min + max) * 0.5;
    let center_x = center.x;
    let center_z = center.z;
    let camera_y = terrain_height(center_x, center_z);
    CameraSpec {
        name: "overview".to_string(),
        position: [center_x, camera_y + 45.0, center_z + 65.0],
        target: [center_x, camera_y, center_z],
        fov_y: 45.0_f32.to_radians(),
    }
}

#[allow(dead_code)]
pub fn track_from_project(project: &TrackProject) -> Result<EditorTrack, String> {
    track_from_project_with_default(project, None)
}

pub fn track_from_project_with_default(
    project: &TrackProject,
    default_layout: Option<&str>,
) -> Result<EditorTrack, String> {
    let project = project.clone().normalize()?;
    let default_index = default_layout
        .and_then(|id| project.layout_index(id))
        .unwrap_or(0);
    track_from_graph(project, default_index)
}

#[allow(dead_code)]
pub fn track_from_editor_project(project: &EditorProject) -> Result<EditorTrack, String> {
    track_from_editor_project_with_default(project, None)
}

pub fn track_from_editor_project_with_default(
    project: &EditorProject,
    default_loop: Option<&str>,
) -> Result<EditorTrack, String> {
    let project = project.clone().normalize()?;
    let graph = project.to_graph()?;
    let default_index = default_loop
        .and_then(|id| graph.layout_index(id))
        .unwrap_or(0);
    track_from_graph(graph, default_index)
}

/// Temporary B1 adapter. It flattens each `@2` route into an independent old
/// single-loop mesh so packs keep rendering until B2 replaces this with shared
/// edge geometry, junction pads, and manual edge-span rendering.
fn track_from_legacy(project: TrackProject, legacy: LegacyProject) -> Result<EditorTrack, String> {
    let default_index = legacy.default_index;
    let mut layouts = Vec::with_capacity(legacy.layouts.len());
    for layout in &legacy.layouts {
        let control_points = layout_control_points_with_heights(&layout.control_points);
        let points = sample_track_with_min_radius(&control_points, min_turn_radius(layout.width));
        if points.len() < 3 {
            return Err(format!(
                "layout '{}' needs at least three samples",
                layout.id
            ));
        }
        let kerb_spans = match &layout.kerbs {
            Some(spans) => sanitize_kerb_spans(spans, points.len()),
            None => auto_kerb_spans(&points),
        };
        let prop_lines = match &layout.prop_lines {
            Some(lines) => sanitize_prop_lines_with_limit(
                lines,
                points.len(),
                default_prop_far_offset(layout.width),
            ),
            None => sanitize_prop_lines_with_limit(
                &default_prop_lines_with_offsets(
                    &points,
                    default_prop_offset(layout.width),
                    default_prop_far_offset(layout.width),
                ),
                points.len(),
                default_prop_far_offset(layout.width),
            ),
        };
        layouts.push(LayoutSamples {
            points,
            kerb_spans,
            prop_lines,
        });
    }

    let mut visual = TrackVisual {
        meshes: Vec::new(),
        materials: materials(),
        textures: vec![conifer_card_texture(), tarmac_texture(), grass_texture()],
        mesh_buffers: Vec::new(),
        instances: Vec::new(),
    };
    let mut collision = TrackCollision {
        vertices: Vec::new(),
        indices: Vec::new(),
        surface_ids: Vec::new(),
        bounds: Aabb {
            min: [0.0; 3],
            max: [0.0; 3],
        },
    };

    let mut asphalt = empty_buffers();
    let mut kerb_red = empty_buffers();
    let mut kerb_white = empty_buffers();
    let mut barriers = empty_buffers();
    let mut tires_blue = empty_buffers();
    let mut tires_white = empty_buffers();
    let mut tree_trunks_high = empty_buffers();
    let mut tree_foliage_high = empty_buffers();
    let mut tree_trunks_low = empty_buffers();
    let mut tree_foliage_low = empty_buffers();
    let mut tree_cards = empty_buffers();
    let mut prop_walls = empty_buffers();
    let mut racing_line = None;

    for (layout_index, (layout, samples)) in legacy.layouts.iter().zip(layouts.iter()).enumerate() {
        let TrackRibbon {
            buffers: track_buffers,
            surface_ids: track_surface_ids,
        } = track_ribbon_with_width(&samples.points, layout.width);
        append_collision_surfaces(&mut collision, &track_buffers, |index| {
            track_surface_ids
                .get(index)
                .copied()
                .unwrap_or(TRACK_SURFACE)
        });
        append_buffers(&mut asphalt, &track_buffers);

        let count = samples.points.len();
        for span in &samples.kerb_spans {
            let start = span.start.clamp(0.0, count as f32);
            let end = span.end.clamp(0.0, count as f32);
            if end - start <= 1e-4 {
                continue;
            }
            let mut previous_end = None;
            for index in start.floor() as usize..end.ceil() as usize {
                let segment_start = (index as f32).max(start);
                let segment_end = ((index + 1) as f32).min(end);
                if segment_end - segment_start <= 1e-4 {
                    continue;
                }
                let (point, binormal) = track_frame(&samples.points, segment_start);
                let (next, next_binormal) = track_frame(&samples.points, segment_end);
                let kerb = if index % 2 == 0 {
                    &mut kerb_red
                } else {
                    &mut kerb_white
                };
                let (start_edge, end_edge) = append_kerb_with_radii(
                    kerb,
                    point,
                    next,
                    binormal,
                    next_binormal,
                    span.side as f32,
                    kerb_inner_radius(layout.width),
                    kerb_outer_radius(layout.width),
                );
                if let Some(previous_end) = previous_end {
                    append_kerb_riser(kerb, previous_end, start_edge, next - point);
                }
                previous_end = Some(end_edge);
            }
        }

        append_prop_lines(
            &samples.points,
            &samples.prop_lines,
            &mut barriers,
            &mut tires_blue,
            &mut tires_white,
            &mut tree_trunks_high,
            &mut tree_foliage_high,
            &mut tree_trunks_low,
            &mut tree_foliage_low,
            &mut tree_cards,
        );
        append_prop_walls(&samples.points, &samples.prop_lines, &mut prop_walls);

        if layout_index == default_index {
            let half_width = asphalt_half_width(layout.width);
            let offsets = racing_line_offsets_with_half_width(&samples.points, half_width);
            racing_line = Some(racing_line_ribbon_with_width(
                &samples.points,
                &offsets,
                half_width,
                outer_edge_radius(layout.width),
            ));
        }
    }

    let terrain_layouts = layouts
        .iter()
        .zip(legacy.layouts.iter())
        .map(|(samples, layout)| (samples.points.clone(), layout.width))
        .collect::<Vec<_>>();
    let terrain = terrain_grid_for_layouts(&terrain_layouts);
    let mountains = mountain_grid();

    push_mesh(&mut visual, "asphalt_dither", 1, asphalt, false);
    if let Some(racing_line) = racing_line {
        push_mesh(
            &mut visual,
            "racing_line",
            RACING_LINE_MATERIAL,
            racing_line,
            false,
        );
    }
    push_mesh(&mut visual, "terrain", 2, terrain, false);
    push_mesh(&mut visual, "mountains", 8, mountains, false);
    push_mesh(&mut visual, "kerb_red", 3, kerb_red, false);
    push_mesh(&mut visual, "kerb_white", 4, kerb_white, false);
    push_mesh(&mut visual, "barriers", 5, barriers, true);
    push_mesh(&mut visual, "tires_blue", 6, tires_blue, true);
    push_mesh(&mut visual, "tires_white", 7, tires_white, true);
    append_collision(&mut collision, &tree_trunks_low, WALL_SURFACE);
    push_mesh(&mut visual, "tree_trunks_high", 9, tree_trunks_high, true);
    push_mesh(
        &mut visual,
        "tree_foliage_high",
        10,
        tree_foliage_high,
        true,
    );
    push_mesh(&mut visual, "tree_trunks_low", 9, tree_trunks_low, true);
    push_mesh(&mut visual, "tree_foliage_low", 10, tree_foliage_low, true);
    push_mesh(&mut visual, "tree_cards", 11, tree_cards, false);

    for (index, mesh) in visual.meshes.iter().enumerate() {
        let surface = match mesh.name.as_str() {
            "terrain" => GRASS_SURFACE,
            "kerb_red" | "kerb_white" => KERB_SURFACE,
            _ => continue,
        };
        append_collision(&mut collision, &visual.mesh_buffers[index], surface);
    }
    append_collision(&mut collision, &prop_walls, WALL_SURFACE);
    collision.bounds = collision_bounds(&collision.vertices);

    let mut layout_specs = Vec::with_capacity(legacy.layouts.len());
    for (layout, samples) in legacy.layouts.iter().zip(layouts.iter()) {
        let length = line_length(&samples.points);
        let spawns = if layout.spawns.is_empty() {
            vec![spawn_for_layout(&samples.points, layout.start_finish)]
        } else {
            layout.spawns.clone()
        };
        let cameras = if layout.cameras.is_empty() {
            vec![overview_camera_for_layout(&samples.points)]
        } else {
            layout.cameras.clone()
        };
        let sectors = if layout.sectors.is_empty() {
            vec![SectorSpec {
                name: "loop".to_string(),
                start_fraction: 0.0,
                end_fraction: 1.0,
            }]
        } else {
            layout.sectors.clone()
        };
        layout_specs.push(LayoutSpec {
            id: layout.id.clone(),
            name: layout.name.clone(),
            length,
            start_finish: layout.start_finish,
            spawns,
            pit_boxes: layout.pit_boxes.clone(),
            cameras,
            sectors,
        });
    }

    let default_layout = &legacy.layouts[default_index];
    let metadata = TrackMetadata {
        name: default_layout.name.clone(),
        length: 0.0,
        spawns: Vec::new(),
        pit_boxes: Vec::new(),
        cameras: Vec::new(),
        sectors: Vec::new(),
        surfaces: track_surfaces(),
        layouts: layout_specs,
        default_layout: default_index as u32,
    }
    .normalize_layouts();

    let bounds = visual_bounds(&visual)?;
    let vertices = visual
        .mesh_buffers
        .iter()
        .map(|buffers| buffers.positions.len() as u32)
        .sum();
    let triangles = visual
        .mesh_buffers
        .iter()
        .map(|buffers| (buffers.indices.len() / 3) as u32)
        .sum();
    let manifest = AssetManifest {
        magic: MAGIC,
        schema_version: SCHEMA_VERSION,
        kind: AssetKind::Track,
        id: default_layout.id.clone(),
        source_hash: [0; 32],
        units: ENGINE_UNITS,
        frame: ENGINE_FRAME,
        bounds,
        chunks: Vec::new(),
        entrypoints: EntryPoints {
            physics: None,
            visual: None,
            collision: None,
            metadata: None,
            audio: None,
            texture: None,
        },
        stats: AssetStats {
            vertices,
            triangles,
            materials: visual.materials.len() as u16,
            textures: visual.textures.len() as u16,
            draw_calls: visual.meshes.len() as u16,
        },
        warnings: vec!["procedural track editor preview".to_string()],
    };

    let default_samples = &layouts[default_index];
    Ok(EditorTrack {
        visual,
        metadata,
        collision,
        manifest,
        kerb_spans: default_samples.kerb_spans.clone(),
        prop_lines: default_samples.prop_lines.clone(),
        project,
    })
}

#[allow(dead_code)]
pub fn pack_from_project(project: &TrackProject) -> Result<Vec<u8>, String> {
    pack_from_project_with_default(project, None)
}

pub fn pack_from_project_with_default(
    project: &TrackProject,
    default_layout: Option<&str>,
) -> Result<Vec<u8>, String> {
    let track = track_from_project_with_default(project, default_layout)?;
    pack_from_track(&track)
}

#[allow(dead_code)]
pub fn pack_from_editor_project(project: &EditorProject) -> Result<Vec<u8>, String> {
    pack_from_editor_project_with_default(project, None)
}

pub fn pack_from_editor_project_with_default(
    project: &EditorProject,
    default_loop: Option<&str>,
) -> Result<Vec<u8>, String> {
    let track = track_from_editor_project_with_default(project, default_loop)?;
    pack_from_track(&track)
}

pub fn pack_from_track(track: &EditorTrack) -> Result<Vec<u8>, String> {
    let chunks = vec![
        sim_asset::ChunkSpec {
            kind: sim_types::ChunkKind::Visual,
            bytes: sim_asset::archive(&track.visual).map_err(|error| error.to_string())?,
        },
        sim_asset::ChunkSpec {
            kind: sim_types::ChunkKind::Collision,
            bytes: sim_asset::archive(&track.collision).map_err(|error| error.to_string())?,
        },
        sim_asset::ChunkSpec {
            kind: sim_types::ChunkKind::Metadata,
            bytes: sim_asset::archive(&track.metadata).map_err(|error| error.to_string())?,
        },
    ];
    let mut manifest = track.manifest.clone();
    manifest.entrypoints = sim_types::EntryPoints {
        physics: None,
        visual: Some(0),
        collision: Some(1),
        metadata: Some(2),
        audio: None,
        texture: None,
    };
    sim_asset::write_pack(manifest, chunks).map_err(|error| error.to_string())
}

fn terrain_height(x: f32, z: f32) -> f32 {
    2.4 * (x * 0.025).sin() * (z * 0.03 + 0.4).cos() + 0.8 * ((x + z) * 0.04).sin()
}

fn terrain_normal(x: f32, z: f32) -> [f32; 3] {
    let epsilon = 0.5;
    let dx = (terrain_height(x + epsilon, z) - terrain_height(x - epsilon, z)) / (epsilon * 2.0);
    let dz = (terrain_height(x, z + epsilon) - terrain_height(x, z - epsilon)) / (epsilon * 2.0);
    Vec3::new(-dx, 1.0, -dz).normalize_or_zero().to_array()
}

fn smoothstep(edge0: f32, edge1: f32, value: f32) -> f32 {
    let t = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn mountain_noise(x: f32, z: f32) -> f32 {
    let ridge = (x * 0.0031 + z * 0.0017).sin() * 0.45
        + (x * 0.0073 - z * 0.0051).sin() * 0.28
        + (x * 0.0157 + z * 0.0113).sin() * 0.17
        + (x * 0.0290 + z * 0.0230).sin() * 0.10;
    (ridge * 0.5 + 0.5).clamp(0.0, 1.0)
}

fn mountain_height(x: f32, z: f32) -> f32 {
    let radius = (x * x + z * z).sqrt();
    let t = ((radius - MOUNTAIN_RISE_START) / (MOUNTAIN_OUTER_RADIUS - MOUNTAIN_RISE_START))
        .clamp(0.0, 1.0);
    let envelope = smoothstep(0.0, 0.25, t) * (1.0 - smoothstep(0.88, 1.0, t));
    let ridge = mountain_noise(x, z);
    envelope * (35.0 + 165.0 * ridge.powf(1.35))
}

fn distant_terrain_height(x: f32, z: f32) -> f32 {
    terrain_height(x, z) - 0.15 + mountain_height(x, z)
}

fn distant_terrain_normal(x: f32, z: f32) -> [f32; 3] {
    let epsilon = 20.0;
    let dx = (distant_terrain_height(x + epsilon, z) - distant_terrain_height(x - epsilon, z))
        / (epsilon * 2.0);
    let dz = (distant_terrain_height(x, z + epsilon) - distant_terrain_height(x, z - epsilon))
        / (epsilon * 2.0);
    Vec3::new(-dx, 1.0, -dz).normalize_or_zero().to_array()
}

fn materials() -> Vec<MaterialRef> {
    vec![
        material("asphalt", [0.045, 0.045, 0.05, 1.0], false),
        MaterialRef {
            name: "dither_shoulder".to_string(),
            base_color: [0.48, 0.345, 0.208, 1.0],
            metallic: 0.0,
            roughness: 0.9,
            base_color_texture: Some(TARMAC_TEXTURE_INDEX),
            outline: false,
        },
        MaterialRef {
            name: "grass".to_string(),
            base_color: [0.302, 0.431, 0.208, 1.0],
            metallic: 0.0,
            roughness: 0.9,
            base_color_texture: Some(GRASS_TEXTURE_INDEX),
            outline: false,
        },
        material("kerb_red", [0.702, 0.141, 0.141, 1.0], false),
        material("kerb_white", [0.933, 0.933, 0.933, 1.0], false),
        material("concrete", [0.667, 0.667, 0.667, 1.0], true),
        material("tire_blue", [0.133, 0.333, 0.667, 1.0], true),
        material("tire_white", [0.867, 0.867, 0.867, 1.0], true),
        material("mountain", [0.12, 0.16, 0.22, 1.0], false),
        material("tree_trunk", [0.32, 0.23, 0.15, 1.0], true),
        material("tree_foliage", [0.34, 0.56, 0.21, 1.0], true),
        MaterialRef {
            name: "tree_card".to_string(),
            base_color: [1.0, 1.0, 1.0, 1.0],
            metallic: 0.0,
            roughness: 0.9,
            base_color_texture: Some(0),
            outline: false,
        },
        MaterialRef {
            name: "racing_line".to_string(),
            base_color: [0.02, 0.02, 0.025, 1.0],
            metallic: 0.0,
            roughness: 0.9,
            base_color_texture: Some(TARMAC_TEXTURE_INDEX),
            outline: false,
        },
    ]
}

fn material(name: &str, color: [f32; 4], outline: bool) -> MaterialRef {
    MaterialRef {
        name: name.to_string(),
        base_color: color,
        metallic: 0.0,
        roughness: 0.9,
        base_color_texture: None,
        outline,
    }
}

fn sample_track(control_points: &[[f32; 3]]) -> Vec<Vec3> {
    sample_track_with_min_radius(control_points, MIN_TURN_RADIUS)
}

fn sample_track_with_min_radius(control_points: &[[f32; 3]], min_turn_radius: f32) -> Vec<Vec3> {
    let controls = control_points
        .iter()
        .map(|point| Vec3::new(point[0], 0.0, point[2]))
        .collect::<Vec<_>>();
    let count = controls.len();
    let mut points = (0..SEGMENTS)
        .map(|index| {
            let global = index as f32 / SEGMENTS as f32 * count as f32;
            let segment = (global.floor() as usize) % count;
            let local = global - global.floor();
            catmull_rom(
                controls[(segment + count - 1) % count],
                controls[segment],
                controls[(segment + 1) % count],
                controls[(segment + 2) % count],
                local,
            )
        })
        .collect::<Vec<_>>();

    limit_centerline_curvature(&mut points, min_turn_radius);

    for point in &mut points {
        point.y = terrain_height(point.x, point.z);
    }
    points
}

fn limit_centerline_curvature(points: &mut [Vec3], min_turn_radius: f32) {
    let count = points.len();
    if count < 3 {
        return;
    }

    let mut radii = vec![0.0; count];
    let mut previous = points.to_vec();
    for _ in 0..CURVATURE_RELAXATION_ITERATIONS {
        let mut too_tight = false;
        for index in 0..count {
            radii[index] = curvature_radius(
                points[(index + count - 1) % count],
                points[index],
                points[(index + 1) % count],
            );
            too_tight |= radii[index] < min_turn_radius;
        }
        if !too_tight {
            return;
        }

        previous.copy_from_slice(points);
        for index in 0..count {
            if radii[index] >= min_turn_radius {
                continue;
            }
            let midpoint =
                (previous[(index + count - 1) % count] + previous[(index + 1) % count]) * 0.5;
            points[index] = previous[index].lerp(midpoint, CURVATURE_RELAXATION);
            points[index].y = 0.0;
        }
    }
}

fn curvature_radius(previous: Vec3, current: Vec3, next: Vec3) -> f32 {
    let incoming = current - previous;
    let outgoing = next - current;
    let turn = (incoming.x * outgoing.z - incoming.z * outgoing.x)
        .atan2(incoming.x * outgoing.x + incoming.z * outgoing.z)
        .abs();
    if turn <= 1e-5 {
        f32::INFINITY
    } else {
        ((incoming.length() + outgoing.length()) * 0.5) / turn
    }
}

fn catmull_rom(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let alpha = 0.5_f32;
    let d01 = p0.distance(p1).powf(alpha);
    let d12 = p1.distance(p2).powf(alpha);
    let d23 = p2.distance(p3).powf(alpha);
    let t0 = 0.0;
    let t1 = t0 + d01.max(1e-5);
    let t2 = t1 + d12.max(1e-5);
    let t3 = t2 + d23.max(1e-5);
    let current = t1 + (t2 - t1) * t.clamp(0.0, 1.0);
    let a1 = p0 * ((t1 - current) / (t1 - t0)) + p1 * ((current - t0) / (t1 - t0));
    let a2 = p1 * ((t2 - current) / (t2 - t1)) + p2 * ((current - t1) / (t2 - t1));
    let a3 = p2 * ((t3 - current) / (t3 - t2)) + p3 * ((current - t2) / (t3 - t2));
    let b1 = a1 * ((t2 - current) / (t2 - t0)) + a2 * ((current - t0) / (t2 - t0));
    let b2 = a2 * ((t3 - current) / (t3 - t1)) + a3 * ((current - t1) / (t3 - t1));
    b1 * ((t2 - current) / (t2 - t1)) + b2 * ((current - t1) / (t2 - t1))
}

fn asphalt_half_width(width: f32) -> f32 {
    width * 0.5
}

fn outer_edge_radius(width: f32) -> f32 {
    width
}

fn kerb_inner_radius(width: f32) -> f32 {
    width * 0.5
}

fn kerb_outer_radius(width: f32) -> f32 {
    width * 0.5 + 0.8
}

fn default_prop_offset(width: f32) -> f32 {
    width + 2.0
}

fn default_prop_far_offset(width: f32) -> f32 {
    width + 7.0
}

fn min_turn_radius(width: f32) -> f32 {
    width + 8.0
}

struct TrackRibbon {
    buffers: MeshBuffers,
    surface_ids: Vec<u16>,
}

#[allow(dead_code)]
fn track_ribbon(points: &[Vec3]) -> TrackRibbon {
    track_ribbon_with_width(points, OUTER_EDGE_RADIUS)
}

fn track_ribbon_with_width(points: &[Vec3], width: f32) -> TrackRibbon {
    let mut buffers = empty_buffers();
    let mut surface_ids = Vec::new();
    let count = points.len();
    let half = asphalt_half_width(width);
    let outer = outer_edge_radius(width);
    let radii = [-outer, -half, half, outer];
    let surfaces = [DIRT_SURFACE, TRACK_SURFACE, DIRT_SURFACE];
    let total_length = (0..count)
        .map(|index| points[index].distance(points[(index + 1) % count]))
        .sum::<f32>();
    let tiles_along = (total_length / TARMAC_TILE_METERS).round().max(1.0);
    let mut distance = 0.0;
    for index in 0..=count {
        let sample = index % count;
        let point = points[sample];
        let next = points[(sample + 1) % count];
        let tangent = (next - point).normalize_or_zero();
        let binormal = Vec3::new(-tangent.z, 0.0, tangent.x).normalize_or_zero();
        let u = if total_length > 1e-4 {
            (distance / total_length) * tiles_along
        } else {
            index as f32
        };
        for radius in radii {
            let lateral = point + binormal * radius;
            let y = terrain_height(lateral.x, lateral.z) + SURFACE_HEIGHT_OFFSET;
            buffers.positions.push([lateral.x, y, lateral.z]);
            buffers.normals.push(terrain_normal(lateral.x, lateral.z));
            let v = ((radius - radii[0]) / (radii[radii.len() - 1] - radii[0])) * TARMAC_UV_ACROSS;
            buffers.uvs.push([u, v]);
        }
        if index < count {
            for (band, surface) in surfaces.iter().enumerate() {
                let base = (index * radii.len() + band) as u32;
                let next_base = ((index + 1) * radii.len() + band) as u32;
                buffers.indices.extend_from_slice(&[
                    base,
                    base + 1,
                    next_base,
                    base + 1,
                    next_base + 1,
                    next_base,
                ]);
                surface_ids.extend_from_slice(&[*surface, *surface]);
            }
        }
        if index < count {
            distance += point.distance(next);
        }
    }
    TrackRibbon {
        buffers,
        surface_ids,
    }
}

#[allow(dead_code)]
fn racing_line_offsets(points: &[Vec3]) -> Vec<f32> {
    racing_line_offsets_with_half_width(points, ASPHALT_HALF_WIDTH)
}

fn racing_line_offsets_with_half_width(points: &[Vec3], half_width: f32) -> Vec<f32> {
    let half_widths = vec![half_width; points.len()];
    racing_line_offsets_with_half_widths(points, &half_widths)
}

fn racing_line_offsets_with_half_widths(points: &[Vec3], half_widths: &[f32]) -> Vec<f32> {
    let count = points.len();
    if count < 3 {
        return vec![0.0; count];
    }
    let max_offsets = (0..count)
        .map(|index| {
            let half_width = half_widths
                .get(index)
                .copied()
                .unwrap_or(ASPHALT_HALF_WIDTH);
            (half_width - RACING_LINE_HALF_WIDTH - RACING_LINE_EDGE_MARGIN).max(0.0)
        })
        .collect::<Vec<_>>();

    let mut target = vec![0.0f32; count];
    for index in 0..count {
        let previous = points[(index + count - 1) % count];
        let current = points[index];
        let next = points[(index + 1) % count];
        let incoming = Vec3::new(current.x - previous.x, 0.0, current.z - previous.z);
        let outgoing = Vec3::new(next.x - current.x, 0.0, next.z - current.z);
        let incoming_length = incoming.length();
        let outgoing_length = outgoing.length();
        if incoming_length < 1e-4 || outgoing_length < 1e-4 {
            continue;
        }
        let sin_turn = (incoming.x * outgoing.z - incoming.z * outgoing.x)
            / (incoming_length * outgoing_length);
        let turn = sin_turn.clamp(-1.0, 1.0).asin();
        let spacing = (incoming_length + outgoing_length) * 0.5;
        let curvature = turn / spacing.max(1e-3);
        let strength = (curvature.abs() / RACING_LINE_REFERENCE_CURVATURE).clamp(0.0, 1.0);
        target[index] = max_offsets[index] * strength * curvature.signum();
    }

    // Circular 1-2-1 smoothing removes per-sample curvature kinks while keeping
    // the loop continuous. Straight samples relax back toward the centreline.
    let mut offsets = target;
    for _ in 0..RACING_LINE_SMOOTHING_PASSES {
        let previous = offsets.clone();
        for index in 0..count {
            offsets[index] = (previous[(index + count - 1) % count]
                + 2.0 * previous[index]
                + previous[(index + 1) % count])
                * 0.25;
        }
    }
    for (index, offset) in offsets.iter_mut().enumerate() {
        *offset = offset.clamp(-max_offsets[index], max_offsets[index]);
    }
    offsets
}

#[allow(dead_code)]
fn racing_line_ribbon(points: &[Vec3], offsets: &[f32]) -> MeshBuffers {
    racing_line_ribbon_with_width(points, offsets, ASPHALT_HALF_WIDTH, OUTER_EDGE_RADIUS)
}

fn racing_line_ribbon_with_width(
    points: &[Vec3],
    offsets: &[f32],
    half_width: f32,
    outer_edge: f32,
) -> MeshBuffers {
    let half_widths = vec![half_width; points.len()];
    racing_line_ribbon_with_half_widths(points, offsets, &half_widths, outer_edge)
}

fn racing_line_ribbon_with_half_widths(
    points: &[Vec3],
    offsets: &[f32],
    half_widths: &[f32],
    outer_edge: f32,
) -> MeshBuffers {
    let mut buffers = empty_buffers();
    let count = points.len();
    if count == 0 {
        return buffers;
    }

    let asphalt = Vec3::from_array(RACING_LINE_ASPHALT_COLOR);
    let dark = Vec3::from_array(RACING_LINE_DARK_COLOR);
    let profile_count = RACING_LINE_PROFILE.len();
    let total_length = (0..count)
        .map(|index| points[index].distance(points[(index + 1) % count]))
        .sum::<f32>();
    let tiles_along = (total_length / TARMAC_TILE_METERS).round().max(1.0);
    let mut distance = 0.0;
    for station in 0..=count {
        let sample = station % count;
        let point = points[sample];
        let next = points[(sample + 1) % count];
        let binormal = binormal_at(points, sample);
        let offset = offsets.get(sample).copied().unwrap_or(0.0);
        let half_width = half_widths
            .get(sample)
            .copied()
            .unwrap_or(ASPHALT_HALF_WIDTH);
        let left = point - binormal * half_width;
        let right = point + binormal * half_width;
        let left_y = terrain_height(left.x, left.z) + SURFACE_HEIGHT_OFFSET;
        let right_y = terrain_height(right.x, right.z) + SURFACE_HEIGHT_OFFSET;
        let u = if total_length > 1e-4 {
            (distance / total_length) * tiles_along
        } else {
            station as f32
        };
        for profile in RACING_LINE_PROFILE.iter() {
            let lateral = offset + profile * RACING_LINE_HALF_WIDTH;
            let position = point + binormal * lateral;
            // The asphalt band is a flat quad between its edge vertices.
            // Interpolate those edge heights so the line follows the rendered
            // surface instead of the underlying terrain curve.
            let t = if half_width > 1e-4 {
                ((lateral + half_width) / (2.0 * half_width)).clamp(0.0, 1.0)
            } else {
                0.5
            };
            let y = left_y + (right_y - left_y) * t + RACING_LINE_LIFT;
            buffers.positions.push([position.x, y, position.z]);
            buffers.normals.push(terrain_normal(position.x, position.z));
            let v = if outer_edge > 1e-4 {
                ((lateral + outer_edge) / (2.0 * outer_edge)) * TARMAC_UV_ACROSS
            } else {
                0.0
            };
            buffers.uvs.push([u, v]);
            // Keep a wide dark core, then feather into the asphalt at the edges.
            // The gain matches the road shader so the feathered edge lands on
            // exactly the same textured colour as the surrounding asphalt.
            let color = dark.lerp(asphalt, smoothstep(0.6, 1.0, profile.abs())) * TARMAC_GAIN;
            buffers.colors.push(color.to_array());
        }
        if station < count {
            for band in 0..profile_count - 1 {
                let base = (station * profile_count + band) as u32;
                let next_base = ((station + 1) * profile_count + band) as u32;
                buffers.indices.extend_from_slice(&[
                    base,
                    base + 1,
                    next_base,
                    base + 1,
                    next_base + 1,
                    next_base,
                ]);
            }
            distance += point.distance(next);
        }
    }
    buffers
}

fn mountain_grid() -> MeshBuffers {
    let mut buffers = empty_buffers();
    let row = MOUNTAIN_SEGMENTS + 1;
    for ring in 0..=MOUNTAIN_RINGS {
        let radial = ring as f32 / MOUNTAIN_RINGS as f32;
        let radius = MOUNTAIN_INNER_RADIUS
            + (MOUNTAIN_OUTER_RADIUS - MOUNTAIN_INNER_RADIUS) * radial.powf(1.6);
        for segment in 0..=MOUNTAIN_SEGMENTS {
            let angle = segment as f32 / MOUNTAIN_SEGMENTS as f32 * TAU;
            let x = radius * angle.cos();
            let z = radius * angle.sin();
            let rise = mountain_height(x, z);
            let rock = smoothstep(10.0, 70.0, rise);
            let color = Vec3::from_array(MOUNTAIN_GROUND_COLOR)
                .lerp(Vec3::from_array(MOUNTAIN_ROCK_COLOR), rock);
            buffers.positions.push([x, distant_terrain_height(x, z), z]);
            buffers.normals.push(distant_terrain_normal(x, z));
            buffers
                .uvs
                .push([segment as f32 / MOUNTAIN_SEGMENTS as f32, radial]);
            buffers.colors.push(color.to_array());
        }
    }
    for ring in 0..MOUNTAIN_RINGS {
        for segment in 0..MOUNTAIN_SEGMENTS {
            let base = (ring * row + segment) as u32;
            let next = ((ring + 1) * row + segment) as u32;
            buffers
                .indices
                .extend_from_slice(&[base, next, next + 1, base, next + 1, base + 1]);
        }
    }
    buffers
}

#[allow(dead_code)]
fn terrain_grid(points: &[Vec3]) -> MeshBuffers {
    terrain_grid_for_layouts(&[(points.to_vec(), OUTER_EDGE_RADIUS)])
}

fn terrain_grid_for_layouts(layouts: &[(Vec<Vec3>, f32)]) -> MeshBuffers {
    let mut buffers = empty_buffers();
    let half = TERRAIN_SIZE * 0.5;
    let step = TERRAIN_SIZE / TERRAIN_CELLS as f32;
    let uv_tiles = TERRAIN_SIZE / TERRAIN_TILE_METERS;
    for iz in 0..=TERRAIN_CELLS {
        for ix in 0..=TERRAIN_CELLS {
            let x = -half + ix as f32 * step;
            let z = -half + iz as f32 * step;
            buffers.positions.push([x, terrain_height(x, z), z]);
            buffers.normals.push(terrain_normal(x, z));
            buffers.uvs.push([
                ix as f32 / TERRAIN_CELLS as f32 * uv_tiles,
                iz as f32 / TERRAIN_CELLS as f32 * uv_tiles,
            ]);
        }
    }
    let row = TERRAIN_CELLS + 1;
    for iz in 0..TERRAIN_CELLS {
        for ix in 0..TERRAIN_CELLS {
            let cell_x = -half + (ix as f32 + 0.5) * step;
            let cell_z = -half + (iz as f32 + 0.5) * step;
            let under_track = layouts.iter().any(|(points, width)| {
                let radius = (outer_edge_radius(*width) - 2.5).max(0.0);
                let hole_radius_squared = radius * radius;
                points.iter().any(|point| {
                    let dx = point.x - cell_x;
                    let dz = point.z - cell_z;
                    dx * dx + dz * dz < hole_radius_squared
                })
            });
            if under_track {
                continue;
            }
            let base = (iz * row + ix) as u32;
            let next = base + row as u32;
            buffers
                .indices
                .extend_from_slice(&[base, next, next + 1, base, next + 1, base + 1]);
        }
    }
    buffers
}

fn binormal_at(points: &[Vec3], index: usize) -> Vec3 {
    let count = points.len();
    let sample = index % count;
    let incoming = (points[sample] - points[(sample + count - 1) % count]).normalize_or_zero();
    let outgoing = (points[(sample + 1) % count] - points[sample]).normalize_or_zero();
    let tangent = (incoming + outgoing).normalize_or_zero();
    Vec3::new(-tangent.z, 0.0, tangent.x).normalize_or_zero()
}

fn track_frame(points: &[Vec3], index: f32) -> (Vec3, Vec3) {
    let count = points.len();
    if count == 0 {
        return (Vec3::ZERO, Vec3::Z);
    }
    let wrapped = index.clamp(0.0, count as f32) % count as f32;
    let base = wrapped.floor() as usize;
    let t = wrapped - base as f32;
    let sample = base % count;
    let next = (sample + 1) % count;
    let point = points[sample].lerp(points[next], t);
    let binormal = binormal_at(points, sample).lerp(binormal_at(points, next), t);
    (point, binormal.normalize_or_zero())
}

fn offset_path_vertices(points: &[Vec3], start: f32, end: f32, offset: f32) -> Vec<Vec3> {
    let limit = points.len() as f32;
    let start = start.clamp(0.0, limit);
    let end = end.clamp(0.0, limit);
    if end - start <= 1e-4 {
        return Vec::new();
    }
    let mut steps = vec![start];
    let mut next = start.floor() + 1.0;
    while next < end - 1e-4 {
        steps.push(next);
        next += 1.0;
    }
    if (end - steps.last().copied().unwrap_or(start)).abs() > 1e-4 {
        steps.push(end);
    }
    steps
        .into_iter()
        .map(|index| {
            let (point, binormal) = track_frame(points, index);
            point + binormal * offset
        })
        .collect()
}

fn prop_placements(points: &[Vec3], line: &PropLine) -> Vec<(Vec3, Vec3)> {
    let spacing = match line.kind {
        PropKind::Blocks => BLOCK_SPACING,
        PropKind::Tyres => TYRE_SPACING,
        PropKind::Trees => TREE_SPACING,
        PropKind::Off => return Vec::new(),
    };
    let vertices =
        offset_path_vertices(points, line.start, line.end, line.offset * line.side as f32);
    let mut segments = Vec::with_capacity(vertices.len().saturating_sub(1));
    let mut total = 0.0;
    for pair in vertices.windows(2) {
        let length = pair[0].distance(pair[1]);
        if length <= 1e-6 {
            continue;
        }
        segments.push((pair[0], pair[1], length, total));
        total += length;
    }
    if total <= 1e-4 {
        return Vec::new();
    }

    let mut placements = Vec::new();
    let mut distance = 0.0;
    let mut segment = 0;
    while distance < total - 1e-4 {
        while segment + 1 < segments.len() && distance > segments[segment].3 + segments[segment].2 {
            segment += 1;
        }
        let (start, end, length, segment_start) = segments[segment];
        let t = ((distance - segment_start) / length).clamp(0.0, 1.0);
        let position = start.lerp(end, t);
        let tangent = (end - start).normalize_or_zero();
        placements.push((position, tangent));
        distance += spacing;
    }
    placements
}

fn prop_noise(line: &PropLine, index: usize, salt: u32) -> f32 {
    let kind_salt = match line.kind {
        PropKind::Blocks => 0x9e37_79b9,
        PropKind::Tyres => 0x85eb_ca6b,
        PropKind::Trees => 0xc2b2_ae35,
        PropKind::Off => 0,
    };
    let mut seed = line.start.to_bits().wrapping_mul(0x9e37_79b9)
        ^ line.end.to_bits().wrapping_mul(0x85eb_ca6b)
        ^ line.offset.to_bits().wrapping_mul(0x27d4_eb2d)
        ^ ((line.side as i32 as u32) << 8)
        ^ kind_salt
        ^ (index as u32).wrapping_mul(0xc2b2_ae35)
        ^ salt;
    seed ^= seed >> 16;
    seed = seed.wrapping_mul(0x7feb_352d);
    seed ^= seed >> 15;
    seed = seed.wrapping_mul(0x846c_a68b);
    seed ^= seed >> 16;
    seed as f32 / u32::MAX as f32 * 2.0 - 1.0
}

fn jittered_prop_placement(
    position: Vec3,
    tangent: Vec3,
    line: &PropLine,
    index: usize,
) -> (Vec3, f32) {
    let forward = Vec3::new(tangent.x, 0.0, tangent.z).normalize_or_zero();
    let binormal = Vec3::new(-forward.z, 0.0, forward.x).normalize_or_zero();
    let position = position
        + binormal * (PROP_LATERAL_JITTER * prop_noise(line, index, 0x1))
        + forward * (PROP_LONGITUDINAL_JITTER * prop_noise(line, index, 0x2));
    let heading = forward.x.atan2(forward.z) + PROP_YAW_JITTER * prop_noise(line, index, 0x3);
    (position, heading)
}

#[allow(clippy::too_many_arguments)]
fn append_prop_lines(
    points: &[Vec3],
    lines: &[PropLine],
    barriers: &mut MeshBuffers,
    tires_blue: &mut MeshBuffers,
    tires_white: &mut MeshBuffers,
    tree_trunks_high: &mut MeshBuffers,
    tree_foliage_high: &mut MeshBuffers,
    tree_trunks_low: &mut MeshBuffers,
    tree_foliage_low: &mut MeshBuffers,
    tree_cards: &mut MeshBuffers,
) {
    for line in lines {
        for (index, (position, tangent)) in prop_placements(points, line).into_iter().enumerate() {
            let (position, heading) = jittered_prop_placement(position, tangent, line, index);
            match line.kind {
                PropKind::Blocks => append_box(
                    barriers,
                    position,
                    Vec3::new(BLOCK_HALF_WIDTH, BLOCK_HALF_HEIGHT, BLOCK_HALF_LENGTH),
                    heading,
                ),
                PropKind::Tyres => {
                    for layer in 0..TYRE_LAYERS {
                        let tire = if layer % 2 == 0 {
                            &mut *tires_blue
                        } else {
                            &mut *tires_white
                        };
                        append_cylinder(
                            tire,
                            position,
                            TYRE_RADIUS,
                            TYRE_LAYER_HEIGHT,
                            8,
                            layer as f32 * TYRE_LAYER_HEIGHT,
                        );
                    }
                }
                PropKind::Trees => {
                    append_conifer_tree(tree_trunks_high, tree_foliage_high, position, line, index);
                    append_simple_tree(tree_trunks_low, tree_foliage_low, position, line, index);
                    append_conifer_card(tree_cards, position, line, index);
                }
                PropKind::Off => {}
            }
        }
    }
}

fn prop_wall_dimensions(kind: PropKind) -> Option<(f32, f32)> {
    match kind {
        PropKind::Blocks => Some((BLOCK_HALF_WIDTH, BLOCK_HALF_HEIGHT * 2.0)),
        PropKind::Tyres => Some((TYRE_RADIUS, TYRE_LAYER_HEIGHT * TYRE_LAYERS as f32)),
        PropKind::Trees | PropKind::Off => None,
    }
}

fn prop_wall_vertical(point: Vec3, height: f32) -> (Vec3, Vec3) {
    let base = terrain_height(point.x, point.z);
    (
        Vec3::new(point.x, base, point.z),
        Vec3::new(point.x, base + height, point.z),
    )
}

fn push_wall_quad(buffers: &mut MeshBuffers, corners: [Vec3; 4], hint: Vec3) {
    let a = corners[0];
    let b = corners[1];
    let c = corners[2];
    let mut normal = (b - a).cross(c - a).normalize_or_zero();
    if normal.dot(hint) < 0.0 {
        normal = -normal;
    }
    push_quad(
        buffers,
        corners.map(|corner| corner.to_array()),
        normal.to_array(),
    );
}

fn append_prop_walls(points: &[Vec3], lines: &[PropLine], walls: &mut MeshBuffers) {
    for line in lines {
        let Some((half_width, height)) = prop_wall_dimensions(line.kind) else {
            continue;
        };
        let side = line.side as f32;
        let inner = offset_path_vertices(
            points,
            line.start,
            line.end,
            (line.offset - half_width) * side,
        );
        let outer = offset_path_vertices(
            points,
            line.start,
            line.end,
            (line.offset + half_width) * side,
        );
        if inner.len() < 2 || inner.len() != outer.len() {
            continue;
        }
        for (inner_pair, outer_pair) in inner.windows(2).zip(outer.windows(2)) {
            let (inner_start_bottom, inner_start_top) = prop_wall_vertical(inner_pair[0], height);
            let (inner_end_bottom, inner_end_top) = prop_wall_vertical(inner_pair[1], height);
            let (outer_start_bottom, outer_start_top) = prop_wall_vertical(outer_pair[0], height);
            let (outer_end_bottom, outer_end_top) = prop_wall_vertical(outer_pair[1], height);
            let tangent = (inner_pair[1] - inner_pair[0]).normalize_or_zero();
            let binormal = Vec3::new(-tangent.z, 0.0, tangent.x).normalize_or_zero();
            push_wall_quad(
                walls,
                [
                    inner_start_bottom,
                    inner_start_top,
                    inner_end_top,
                    inner_end_bottom,
                ],
                -binormal * side,
            );
            push_wall_quad(
                walls,
                [
                    outer_start_bottom,
                    outer_start_top,
                    outer_end_top,
                    outer_end_bottom,
                ],
                binormal * side,
            );
            push_wall_quad(
                walls,
                [
                    inner_start_top,
                    outer_start_top,
                    outer_end_top,
                    inner_end_top,
                ],
                Vec3::Y,
            );
        }

        let closed = line.start <= 0.0 && line.end >= points.len() as f32;
        if !closed {
            let (inner_start_bottom, inner_start_top) = prop_wall_vertical(inner[0], height);
            let (outer_start_bottom, outer_start_top) = prop_wall_vertical(outer[0], height);
            let start_tangent = (inner[1] - inner[0]).normalize_or_zero();
            push_wall_quad(
                walls,
                [
                    inner_start_bottom,
                    outer_start_bottom,
                    outer_start_top,
                    inner_start_top,
                ],
                -start_tangent,
            );

            let (inner_end_bottom, inner_end_top) =
                prop_wall_vertical(inner[inner.len() - 1], height);
            let (outer_end_bottom, outer_end_top) =
                prop_wall_vertical(outer[outer.len() - 1], height);
            let end_tangent = (inner[inner.len() - 1] - inner[inner.len() - 2]).normalize_or_zero();
            push_wall_quad(
                walls,
                [
                    inner_end_bottom,
                    outer_end_bottom,
                    outer_end_top,
                    inner_end_top,
                ],
                end_tangent,
            );
        }
    }
}

#[allow(dead_code)]
fn append_kerb(
    buffers: &mut MeshBuffers,
    start: Vec3,
    end: Vec3,
    start_binormal: Vec3,
    end_binormal: Vec3,
    side: f32,
) -> ([Vec3; 2], [Vec3; 2]) {
    append_kerb_with_radii(
        buffers,
        start,
        end,
        start_binormal,
        end_binormal,
        side,
        KERB_INNER_RADIUS,
        KERB_OUTER_RADIUS,
    )
}

#[allow(clippy::too_many_arguments)]
fn append_kerb_with_radii(
    buffers: &mut MeshBuffers,
    start: Vec3,
    end: Vec3,
    start_binormal: Vec3,
    end_binormal: Vec3,
    side: f32,
    kerb_inner: f32,
    kerb_outer: f32,
) -> ([Vec3; 2], [Vec3; 2]) {
    let inner_start = start + start_binormal * (kerb_inner * side);
    let inner_end = end + end_binormal * (kerb_inner * side);
    let outer_start = start + start_binormal * (kerb_outer * side);
    let outer_end = end + end_binormal * (kerb_outer * side);
    let inner_base_start = terrain_height(inner_start.x, inner_start.z);
    let inner_base_end = terrain_height(inner_end.x, inner_end.z);
    let outer_base_start = terrain_height(outer_start.x, outer_start.z);
    let outer_base_end = terrain_height(outer_end.x, outer_end.z);
    let inner_top_start = Vec3::new(
        inner_start.x,
        inner_base_start + KERB_SURFACE_OFFSET,
        inner_start.z,
    );
    let inner_top_end = Vec3::new(
        inner_end.x,
        inner_base_end + KERB_SURFACE_OFFSET,
        inner_end.z,
    );
    let outer_top_start = Vec3::new(
        outer_start.x,
        outer_base_start + KERB_SURFACE_OFFSET,
        outer_start.z,
    );
    let outer_top_end = Vec3::new(outer_end.x, outer_base_end + KERB_TOP_OFFSET, outer_end.z);
    let outer_bottom_start = Vec3::new(
        outer_start.x,
        outer_base_start + KERB_BOTTOM_OFFSET,
        outer_start.z,
    );
    let outer_bottom_end = Vec3::new(
        outer_end.x,
        outer_base_end + KERB_BOTTOM_OFFSET,
        outer_end.z,
    );
    let side_normal = (start_binormal + end_binormal).normalize_or_zero() * side;
    let along = (end - start).normalize_or_zero();
    let across = (outer_top_start - inner_top_start).normalize_or_zero();
    let mut top_normal = across.cross(along).normalize_or_zero();
    if top_normal.y < 0.0 {
        top_normal = -top_normal;
    }
    push_quad(
        buffers,
        [
            inner_top_start.to_array(),
            outer_top_start.to_array(),
            outer_top_end.to_array(),
            inner_top_end.to_array(),
        ],
        top_normal.to_array(),
    );
    push_quad(
        buffers,
        [
            outer_top_start.to_array(),
            outer_top_end.to_array(),
            outer_bottom_end.to_array(),
            outer_bottom_start.to_array(),
        ],
        side_normal.to_array(),
    );
    (
        [inner_top_start, outer_top_start],
        [inner_top_end, outer_top_end],
    )
}

fn append_kerb_riser(
    buffers: &mut MeshBuffers,
    previous_end: [Vec3; 2],
    current_start: [Vec3; 2],
    along: Vec3,
) {
    let corners = [previous_end[0], previous_end[1], current_start[1]];
    let a = corners[0];
    let b = corners[1];
    let c = corners[2];
    let mut normal = (b - a).cross(c - a).normalize_or_zero();
    if normal.dot(-along) < 0.0 {
        normal = -normal;
    }
    push_triangle(
        buffers,
        corners.map(|corner| corner.to_array()),
        normal.to_array(),
    );
}

fn append_box(buffers: &mut MeshBuffers, center: Vec3, half: Vec3, yaw: f32) {
    let (sin_yaw, cos_yaw) = yaw.sin_cos();
    let rotate = |local: Vec3| {
        let x = center.x + local.x * cos_yaw + local.z * sin_yaw;
        let z = center.z - local.x * sin_yaw + local.z * cos_yaw;
        let y = terrain_height(x, z) + local.y + half.y;
        Vec3::new(x, y, z)
    };
    let faces = [
        (
            Vec3::X,
            [
                Vec3::new(half.x, -half.y, half.z),
                Vec3::new(half.x, -half.y, -half.z),
                Vec3::new(half.x, half.y, -half.z),
                Vec3::new(half.x, half.y, half.z),
            ],
        ),
        (
            Vec3::NEG_X,
            [
                Vec3::new(-half.x, -half.y, -half.z),
                Vec3::new(-half.x, -half.y, half.z),
                Vec3::new(-half.x, half.y, half.z),
                Vec3::new(-half.x, half.y, -half.z),
            ],
        ),
        (
            Vec3::Y,
            [
                Vec3::new(-half.x, half.y, half.z),
                Vec3::new(half.x, half.y, half.z),
                Vec3::new(half.x, half.y, -half.z),
                Vec3::new(-half.x, half.y, -half.z),
            ],
        ),
        (
            Vec3::NEG_Y,
            [
                Vec3::new(-half.x, -half.y, -half.z),
                Vec3::new(half.x, -half.y, -half.z),
                Vec3::new(half.x, -half.y, half.z),
                Vec3::new(-half.x, -half.y, half.z),
            ],
        ),
        (
            Vec3::Z,
            [
                Vec3::new(-half.x, -half.y, half.z),
                Vec3::new(half.x, -half.y, half.z),
                Vec3::new(half.x, half.y, half.z),
                Vec3::new(-half.x, half.y, half.z),
            ],
        ),
        (
            Vec3::NEG_Z,
            [
                Vec3::new(half.x, -half.y, -half.z),
                Vec3::new(-half.x, -half.y, -half.z),
                Vec3::new(-half.x, half.y, -half.z),
                Vec3::new(half.x, half.y, -half.z),
            ],
        ),
    ];
    for (normal, corners) in faces {
        let rotated_normal = if normal.y.abs() > 0.5 {
            Vec3::from_array(terrain_normal(center.x, center.z)) * normal.y.signum()
        } else {
            Vec3::new(
                normal.x * cos_yaw + normal.z * sin_yaw,
                normal.y,
                -normal.x * sin_yaw + normal.z * cos_yaw,
            )
        };
        push_quad(
            buffers,
            corners.map(|corner| rotate(corner).to_array()),
            rotated_normal.to_array(),
        );
    }
}

fn append_cylinder(
    buffers: &mut MeshBuffers,
    center: Vec3,
    radius: f32,
    height: f32,
    segments: usize,
    base_offset: f32,
) {
    let top_normal = Vec3::from_array(terrain_normal(center.x, center.z));
    let top_center = Vec3::new(
        center.x,
        terrain_height(center.x, center.z) + base_offset + height,
        center.z,
    );
    let bottom_center = Vec3::new(
        center.x,
        terrain_height(center.x, center.z) + base_offset,
        center.z,
    );
    for index in 0..segments {
        let angle0 = index as f32 / segments as f32 * TAU;
        let angle1 = (index + 1) as f32 / segments as f32 * TAU;
        let x0 = center.x + angle0.cos() * radius;
        let z0 = center.z + angle0.sin() * radius;
        let x1 = center.x + angle1.cos() * radius;
        let z1 = center.z + angle1.sin() * radius;
        let ground0 = terrain_height(x0, z0) + base_offset;
        let ground1 = terrain_height(x1, z1) + base_offset;
        let bottom0 = Vec3::new(x0, ground0, z0);
        let bottom1 = Vec3::new(x1, ground1, z1);
        let top0 = Vec3::new(x0, ground0 + height, z0);
        let top1 = Vec3::new(x1, ground1 + height, z1);
        let middle = (angle0 + angle1) * 0.5;
        push_quad(
            buffers,
            [
                bottom0.to_array(),
                top0.to_array(),
                top1.to_array(),
                bottom1.to_array(),
            ],
            [middle.cos(), 0.0, middle.sin()],
        );
        push_triangle(
            buffers,
            [top_center.to_array(), top1.to_array(), top0.to_array()],
            top_normal.to_array(),
        );
        push_triangle(
            buffers,
            [
                bottom_center.to_array(),
                bottom0.to_array(),
                bottom1.to_array(),
            ],
            (-top_normal).to_array(),
        );
    }
}

fn lerp_color(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn append_simple_tree(
    trunks: &mut MeshBuffers,
    foliage: &mut MeshBuffers,
    position: Vec3,
    line: &PropLine,
    index: usize,
) {
    let variant = prop_noise(line, index, 0x4);
    let scale = 0.85 + 0.3 * (prop_noise(line, index, 0x5) * 0.5 + 0.5);
    let tint = prop_noise(line, index, 0x6) * 0.5 + 0.5;
    let ground = terrain_height(position.x, position.z);
    let base = Vec3::new(position.x, ground, position.z);
    let pine = variant > 0.0;
    let trunk_height = if pine { 2.4 } else { 2.0 } * scale;
    let trunk_radius = if pine { 0.16 } else { 0.21 } * scale;
    append_tree_trunk(trunks, base, trunk_radius, trunk_height);

    let (color, tiers) = if pine {
        (
            lerp_color(TREE_PINE_COLOR, TREE_PINE_COLOR_ALT, tint),
            [(0.0, 1.5, 2.2), (1.25, 1.15, 2.0), (2.35, 0.8, 1.8)],
        )
    } else {
        (
            lerp_color(TREE_COMMON_COLOR, TREE_COMMON_COLOR_ALT, tint),
            [(0.0, 1.7, 2.4), (1.0, 1.35, 2.1), (1.9, 0.95, 1.7)],
        )
    };
    let canopy_base = base + Vec3::Y * (trunk_height * 0.45);
    for (offset, radius, height) in tiers {
        append_tree_cone(
            foliage,
            canopy_base + Vec3::Y * (offset * scale),
            radius * scale,
            height * scale,
            color,
        );
    }
}

fn tarmac_texture() -> TextureRef {
    let size = TARMAC_TEXTURE_SIZE;
    let mut rgba8 = Vec::with_capacity(size * size * 4);
    for y in 0..size {
        for x in 0..size {
            let u = x as f32 / size as f32;
            let v = y as f32 / size as f32;
            let coarse = tarmac_value_noise(u, v, 6, 0x1a2b_3c4d);
            let medium = tarmac_value_noise(u, v, 18, 0x5e6f_7a8b);
            let fine = tarmac_value_noise(u, v, 54, 0x9c0d_1e2f);
            let grain = tarmac_hash(x as u32, y as u32, 0x3141_5926);
            let patches = smoothstep(0.52, 0.82, coarse);
            let shade = (medium - 0.5) * 0.09 + (fine - 0.5) * 0.045 + (grain - 0.5) * 0.025
                - patches * 0.03;
            let value = (0.71 + shade).clamp(0.62, 0.79);
            let cool = (medium - 0.5) * 0.02;
            let r = ((value - cool).clamp(0.0, 1.0) * 255.0).round() as u8;
            let g = (value * 255.0).round() as u8;
            let b = ((value + cool).clamp(0.0, 1.0) * 255.0).round() as u8;
            rgba8.extend_from_slice(&[r, g, b, 255]);
        }
    }
    TextureRef {
        name: "tarmac_procedural".to_string(),
        mime: "image/png".to_string(),
        hash: [0; 32],
        width: size as u32,
        height: size as u32,
        rgba8,
    }
}

fn grass_texture() -> TextureRef {
    let size = GRASS_TEXTURE_SIZE;
    let mut rgba8 = Vec::with_capacity(size * size * 4);
    for y in 0..size {
        for x in 0..size {
            let u = x as f32 / size as f32;
            let v = y as f32 / size as f32;
            let coarse = tarmac_value_noise(u, v, 5, 0x2b3c_4d5e);
            let medium = tarmac_value_noise(u, v, 17, 0x6f7a_8b9c);
            let fine = tarmac_value_noise(u, v, 48, 0xa1b2_c3d4);
            let grain = tarmac_hash(x as u32, y as u32, 0x0f1e_2d3c);
            let shade = (coarse - 0.5) * 0.12
                + (medium - 0.5) * 0.07
                + (fine - 0.5) * 0.04
                + (grain - 0.5) * 0.025;
            let r = (72.0 * (1.0 + shade)).clamp(0.0, 255.0).round() as u8;
            let g = (107.0 * (1.0 + shade)).clamp(0.0, 255.0).round() as u8;
            let b = (22.0 * (1.0 + shade)).clamp(0.0, 255.0).round() as u8;
            rgba8.extend_from_slice(&[r, g, b, 255]);
        }
    }
    TextureRef {
        name: "grass_procedural".to_string(),
        mime: "image/png".to_string(),
        hash: [0; 32],
        width: size as u32,
        height: size as u32,
        rgba8,
    }
}

fn tarmac_hash(x: u32, y: u32, seed: u32) -> f32 {
    let mut hash = x.wrapping_mul(0x9e37_79b1) ^ y.wrapping_mul(0x85eb_ca77) ^ seed;
    hash ^= hash >> 15;
    hash = hash.wrapping_mul(0x2c1b_3c6d);
    hash ^= hash >> 12;
    hash = hash.wrapping_mul(0x297a_2d39);
    hash ^= hash >> 15;
    hash as f32 / u32::MAX as f32
}

fn tarmac_value_noise(u: f32, v: f32, cells: u32, seed: u32) -> f32 {
    let x = u * cells as f32;
    let y = v * cells as f32;
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let tx = x - x0 as f32;
    let ty = y - y0 as f32;
    let sx = tx * tx * (3.0 - 2.0 * tx);
    let sy = ty * ty * (3.0 - 2.0 * ty);
    let wrap = |value: i32| value.rem_euclid(cells as i32) as u32;
    let a = tarmac_hash(wrap(x0), wrap(y0), seed);
    let b = tarmac_hash(wrap(x0 + 1), wrap(y0), seed);
    let c = tarmac_hash(wrap(x0), wrap(y0 + 1), seed);
    let d = tarmac_hash(wrap(x0 + 1), wrap(y0 + 1), seed);
    let ab = a + (b - a) * sx;
    let cd = c + (d - c) * sx;
    ab + (cd - ab) * sy
}

fn conifer_card_texture() -> TextureRef {
    TextureRef {
        name: "tree_cards_atlas".to_string(),
        mime: "image/png".to_string(),
        hash: [0; 32],
        width: TREE_CARD_ATLAS_WIDTH as u32,
        height: TREE_CARD_ATLAS_HEIGHT as u32,
        rgba8: TREE_CARD_ATLAS_RGBA.to_vec(),
    }
}

fn append_conifer_card(buffers: &mut MeshBuffers, position: Vec3, line: &PropLine, index: usize) {
    let profile = ((prop_noise(line, index, 0x4) * 0.5 + 0.5) * 3.0)
        .floor()
        .clamp(0.0, 2.0) as usize;
    let base_height = match profile {
        0 => 8.0,
        1 => 7.5,
        _ => 4.4,
    };
    let height = base_height
        * TREE_CONIFER_SIZE
        * (0.88 + 0.28 * (prop_noise(line, index, 0x5) * 0.5 + 0.5));
    let width = height * 0.52;
    let ground = terrain_height(position.x, position.z);
    let base = Vec3::new(position.x, ground, position.z);
    let center = base + Vec3::Y * (height * 0.5);
    let tint = 0.92 + 0.12 * (prop_noise(line, index, 0xA) * 0.5 + 0.5);
    let variant = ((prop_noise(line, index, 0xB) * 0.5 + 0.5) * TREE_CARD_VARIANTS as f32)
        .floor()
        .clamp(0.0, (TREE_CARD_VARIANTS - 1) as f32) as usize;
    let u0 = variant as f32 / TREE_CARD_VARIANTS as f32;
    let u1 = (variant + 1) as f32 / TREE_CARD_VARIANTS as f32;
    push_billboard_card(
        buffers,
        center,
        width,
        height,
        [[u0, 1.0], [u1, 1.0], [u1, 0.0], [u0, 0.0]],
        tint,
    );
}

fn push_billboard_card(
    buffers: &mut MeshBuffers,
    center: Vec3,
    width: f32,
    height: f32,
    uvs: [[f32; 2]; 4],
    tint: f32,
) {
    let base = buffers.positions.len() as u32;
    let corners = [[-0.5, -0.5], [0.5, -0.5], [0.5, 0.5], [-0.5, 0.5]];
    for (corner, uv) in corners.into_iter().zip(uvs) {
        buffers.positions.push(center.to_array());
        buffers.normals.push([width, height, 0.0]);
        buffers.uvs.push(uv);
        buffers.colors.push([corner[0], corner[1], tint]);
    }
    buffers
        .indices
        .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn append_conifer_tree(
    trunks: &mut MeshBuffers,
    foliage: &mut MeshBuffers,
    position: Vec3,
    line: &PropLine,
    index: usize,
) {
    let profile = ((prop_noise(line, index, 0x4) * 0.5 + 0.5) * 3.0)
        .floor()
        .clamp(0.0, 2.0) as usize;
    let (trunk_height, trunk_radius, tiers) = match profile {
        0 => (1.6, 0.18, TREE_CONIFER_TALL),
        1 => (1.8, 0.24, TREE_CONIFER_BROAD),
        _ => (1.0, 0.13, TREE_CONIFER_YOUNG),
    };
    let total_height = tiers.last().map(|tier| tier.0 + tier.2).unwrap_or(1.0);
    let scale = TREE_CONIFER_SIZE * (0.88 + 0.28 * (prop_noise(line, index, 0x5) * 0.5 + 0.5));
    let tint = prop_noise(line, index, 0x6) * 0.5 + 0.5;
    let lean = Vec3::new(
        prop_noise(line, index, 0x7) * 0.35,
        0.0,
        prop_noise(line, index, 0x8) * 0.35,
    ) * scale;
    let ground = terrain_height(position.x, position.z);
    let base = Vec3::new(position.x, ground, position.z);
    let canopy_color = lerp_color(TREE_PINE_COLOR, TREE_PINE_COLOR_ALT, tint);

    append_conifer_trunk(
        trunks,
        base,
        trunk_radius * scale,
        trunk_height * scale,
        lean * (trunk_height / total_height),
        TREE_TRUNK_COLOR,
    );

    let tier_count = tiers.len();
    for (tier_index, (offset, radius, height)) in tiers.iter().enumerate() {
        let tier_lean = lean * ((offset + height * 0.5) / total_height);
        append_conifer_tier(
            foliage,
            base,
            *offset * scale,
            *radius * scale,
            *height * scale,
            tier_index,
            tier_count,
            tier_lean,
            canopy_color,
            line,
            index,
        );
    }

    let last = tiers[tiers.len() - 1];
    let last_lean = lean * ((last.0 + last.2) / total_height);
    append_tree_cone(
        foliage,
        base + Vec3::Y * ((last.0 + last.2) * scale) + last_lean,
        last.1 * scale * 0.10,
        last.2 * scale * 0.5,
        lerp_color(TREE_PINE_DARK, canopy_color, 0.85),
    );
}

fn append_conifer_trunk(
    buffers: &mut MeshBuffers,
    base: Vec3,
    radius: f32,
    height: f32,
    lean: Vec3,
    color: [f32; 3],
) {
    let top = base + Vec3::Y * height + lean;
    let top_radius = radius * 0.5;
    for side in 0..TREE_CONIFER_TRUNK_SIDES {
        let angle0 = side as f32 / TREE_CONIFER_TRUNK_SIDES as f32 * TAU;
        let angle1 = (side + 1) as f32 / TREE_CONIFER_TRUNK_SIDES as f32 * TAU;
        let direction0 = Vec3::new(angle0.cos(), 0.0, angle0.sin());
        let direction1 = Vec3::new(angle1.cos(), 0.0, angle1.sin());
        let bottom0 = base + direction0 * radius;
        let bottom1 = base + direction1 * radius;
        let top0 = top + direction0 * top_radius;
        let top1 = top + direction1 * top_radius;
        let mut normal = (bottom1 - bottom0)
            .cross(top0 - bottom0)
            .normalize_or_zero();
        if normal.dot(direction0 + direction1) < 0.0 {
            normal = -normal;
        }
        push_quad_colored(
            buffers,
            [
                bottom0.to_array(),
                top0.to_array(),
                top1.to_array(),
                bottom1.to_array(),
            ],
            normal.to_array(),
            color,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn append_conifer_tier(
    buffers: &mut MeshBuffers,
    base: Vec3,
    offset: f32,
    radius: f32,
    height: f32,
    tier_index: usize,
    tier_count: usize,
    lean: Vec3,
    color: [f32; 3],
    line: &PropLine,
    index: usize,
) {
    let center = base + Vec3::Y * offset + lean;
    let top_center = center + Vec3::Y * height;
    let top_radius = radius * 0.10;
    let phase = tier_index as f32 * 0.37 + prop_noise(line, index, 0x20 + tier_index as u32) * 0.18;
    let tier_t = if tier_count > 1 {
        tier_index as f32 / (tier_count - 1) as f32
    } else {
        0.0
    };
    let tier_color = lerp_color(TREE_PINE_DARK, color, 0.5 + 0.5 * tier_t);
    let mut ring_radius = [0.0_f32; TREE_CONIFER_SIDES];
    let mut ring_droop = [0.0_f32; TREE_CONIFER_SIDES];
    for (side, slot) in ring_radius.iter_mut().enumerate() {
        let jitter = 0.88
            + 0.24
                * (prop_noise(line, index, 0x40 + tier_index as u32 * 8 + side as u32) * 0.5 + 0.5);
        *slot = radius * jitter;
        ring_droop[side] = height
            * (0.06
                + 0.12
                    * (prop_noise(line, index, 0x80 + tier_index as u32 * 8 + side as u32) * 0.5
                        + 0.5));
    }
    for (side, &radius0) in ring_radius.iter().enumerate() {
        let next = (side + 1) % TREE_CONIFER_SIDES;
        let radius1 = ring_radius[next];
        let droop0 = ring_droop[side];
        let droop1 = ring_droop[next];
        let angle0 = phase + side as f32 / TREE_CONIFER_SIDES as f32 * TAU;
        let angle1 = phase + next as f32 / TREE_CONIFER_SIDES as f32 * TAU;
        let direction0 = Vec3::new(angle0.cos(), 0.0, angle0.sin());
        let direction1 = Vec3::new(angle1.cos(), 0.0, angle1.sin());
        let bottom0 = center + direction0 * radius0 - Vec3::Y * droop0;
        let bottom1 = center + direction1 * radius1 - Vec3::Y * droop1;
        let top0 = top_center + direction0 * top_radius;
        let top1 = top_center + direction1 * top_radius;
        let mut normal = (bottom1 - bottom0)
            .cross(top0 - bottom0)
            .normalize_or_zero();
        if normal.dot(direction0 + direction1) < 0.0 {
            normal = -normal;
        }
        push_quad_colored(
            buffers,
            [
                bottom0.to_array(),
                top0.to_array(),
                top1.to_array(),
                bottom1.to_array(),
            ],
            normal.to_array(),
            tier_color,
        );
    }
}

fn append_tree_trunk(buffers: &mut MeshBuffers, base: Vec3, radius: f32, height: f32) {
    let top = base + Vec3::Y * height;
    let top_radius = radius * 0.55;
    let top_normal = Vec3::Y.to_array();
    for index in 0..TREE_SIDES {
        let angle0 = index as f32 / TREE_SIDES as f32 * TAU;
        let angle1 = (index + 1) as f32 / TREE_SIDES as f32 * TAU;
        let direction0 = Vec3::new(angle0.cos(), 0.0, angle0.sin());
        let direction1 = Vec3::new(angle1.cos(), 0.0, angle1.sin());
        let bottom0 = base + direction0 * radius;
        let bottom1 = base + direction1 * radius;
        let top0 = top + direction0 * top_radius;
        let top1 = top + direction1 * top_radius;
        let mut normal = (bottom1 - bottom0)
            .cross(top0 - bottom0)
            .normalize_or_zero();
        if normal.dot(direction0 + direction1) < 0.0 {
            normal = -normal;
        }
        push_quad_colored(
            buffers,
            [
                bottom0.to_array(),
                top0.to_array(),
                top1.to_array(),
                bottom1.to_array(),
            ],
            normal.to_array(),
            TREE_TRUNK_COLOR,
        );
        push_triangle_colored(
            buffers,
            [top.to_array(), top1.to_array(), top0.to_array()],
            top_normal,
            TREE_TRUNK_COLOR,
        );
    }
}

fn append_tree_cone(
    buffers: &mut MeshBuffers,
    base: Vec3,
    radius: f32,
    height: f32,
    color: [f32; 3],
) {
    let apex = base + Vec3::Y * height;
    for index in 0..TREE_SIDES {
        let angle0 = index as f32 / TREE_SIDES as f32 * TAU;
        let angle1 = (index + 1) as f32 / TREE_SIDES as f32 * TAU;
        let direction0 = Vec3::new(angle0.cos(), 0.0, angle0.sin());
        let direction1 = Vec3::new(angle1.cos(), 0.0, angle1.sin());
        let bottom0 = base + direction0 * radius;
        let bottom1 = base + direction1 * radius;
        let mut normal = (apex - bottom0)
            .cross(bottom1 - bottom0)
            .normalize_or_zero();
        let radial = direction0 + direction1;
        if normal.dot(radial) < 0.0 {
            normal = -normal;
        }
        push_triangle_colored(
            buffers,
            [bottom0.to_array(), apex.to_array(), bottom1.to_array()],
            normal.to_array(),
            color,
        );
    }
}

fn push_quad_colored(
    buffers: &mut MeshBuffers,
    corners: [[f32; 3]; 4],
    normal: [f32; 3],
    color: [f32; 3],
) {
    let base = buffers.positions.len() as u32;
    for corner in corners {
        buffers.positions.push(corner);
        buffers.normals.push(normal);
        buffers.uvs.push([0.0, 0.0]);
        buffers.colors.push(color);
    }
    buffers
        .indices
        .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn push_triangle_colored(
    buffers: &mut MeshBuffers,
    corners: [[f32; 3]; 3],
    normal: [f32; 3],
    color: [f32; 3],
) {
    let base = buffers.positions.len() as u32;
    for corner in corners {
        buffers.positions.push(corner);
        buffers.normals.push(normal);
        buffers.uvs.push([0.0, 0.0]);
        buffers.colors.push(color);
    }
    buffers
        .indices
        .extend_from_slice(&[base, base + 1, base + 2]);
}

fn push_quad(buffers: &mut MeshBuffers, corners: [[f32; 3]; 4], normal: [f32; 3]) {
    let base = buffers.positions.len() as u32;
    for corner in corners {
        buffers.positions.push(corner);
        buffers.normals.push(normal);
        buffers.uvs.push([0.0, 0.0]);
    }
    buffers
        .indices
        .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn push_triangle(buffers: &mut MeshBuffers, corners: [[f32; 3]; 3], normal: [f32; 3]) {
    let base = buffers.positions.len() as u32;
    for corner in corners {
        buffers.positions.push(corner);
        buffers.normals.push(normal);
        buffers.uvs.push([0.0, 0.0]);
    }
    buffers
        .indices
        .extend_from_slice(&[base, base + 1, base + 2]);
}

fn append_collision_surfaces(
    collision: &mut TrackCollision,
    buffers: &MeshBuffers,
    surface_at: impl Fn(usize) -> u16,
) {
    let base = collision.vertices.len() as u32;
    collision.vertices.extend_from_slice(&buffers.positions);
    for (triangle, [a, b, c]) in buffers.indices.as_chunks::<3>().0.iter().enumerate() {
        let a = base + *a;
        let b = base + *b;
        let c = base + *c;
        let pa = Vec3::from_array(collision.vertices[a as usize]);
        let pb = Vec3::from_array(collision.vertices[b as usize]);
        let pc = Vec3::from_array(collision.vertices[c as usize]);
        let normal = (pb - pa).cross(pc - pa);
        if normal.y < 0.0 {
            collision.indices.extend_from_slice(&[a, c, b]);
        } else {
            collision.indices.extend_from_slice(&[a, b, c]);
        }
        collision.surface_ids.push(surface_at(triangle));
    }
}

fn append_collision(collision: &mut TrackCollision, buffers: &MeshBuffers, surface: u16) {
    append_collision_surfaces(collision, buffers, |_| surface);
}

fn collision_bounds(vertices: &[[f32; 3]]) -> Aabb {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for vertex in vertices {
        let point = Vec3::from_array(*vertex);
        min = min.min(point);
        max = max.max(point);
    }
    if !min.is_finite() || !max.is_finite() {
        return Aabb {
            min: [0.0; 3],
            max: [0.0; 3],
        };
    }
    Aabb {
        min: min.to_array(),
        max: max.to_array(),
    }
}

fn visual_bounds(visual: &TrackVisual) -> Result<Aabb, String> {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for buffers in &visual.mesh_buffers {
        for vertex in &buffers.positions {
            let point = Vec3::from_array(*vertex);
            min = min.min(point);
            max = max.max(point);
        }
    }
    if !min.is_finite() || !max.is_finite() {
        return Err("procedural track has no finite geometry".to_string());
    }
    Ok(Aabb {
        min: min.to_array(),
        max: max.to_array(),
    })
}

fn empty_buffers() -> MeshBuffers {
    MeshBuffers {
        positions: Vec::new(),
        normals: Vec::new(),
        uvs: Vec::new(),
        colors: Vec::new(),
        indices: Vec::new(),
    }
}

fn append_buffers(target: &mut MeshBuffers, source: &MeshBuffers) {
    let vertex_offset = target.positions.len() as u32;
    target.positions.extend_from_slice(&source.positions);
    target.normals.extend_from_slice(&source.normals);
    target.uvs.extend_from_slice(&source.uvs);
    target.colors.extend_from_slice(&source.colors);
    target
        .indices
        .extend(source.indices.iter().map(|index| index + vertex_offset));
}

fn push_mesh(
    visual: &mut TrackVisual,
    name: &str,
    material: u16,
    buffers: MeshBuffers,
    outline: bool,
) {
    if buffers.positions.is_empty() || buffers.indices.is_empty() {
        return;
    }
    let vertex_offset = visual
        .mesh_buffers
        .iter()
        .map(|buffers| buffers.positions.len() as u32)
        .sum();
    let index_offset = visual
        .mesh_buffers
        .iter()
        .map(|buffers| buffers.indices.len() as u32)
        .sum();
    let bounds = collision_bounds(&buffers.positions);
    visual.meshes.push(MeshRef {
        name: name.to_string(),
        vertex_offset,
        vertex_count: buffers.positions.len() as u32,
        index_offset,
        index_count: buffers.indices.len() as u32,
        material,
        bounds,
        lod: 0,
    });
    if let Some(slot) = visual.materials.get_mut(material as usize) {
        slot.outline = outline;
    }
    visual.mesh_buffers.push(buffers);
}

#[cfg(test)]
mod tests {
    use super::*;

    const TIGHT_CONTROL_POINTS: [[f32; 3]; 10] = [
        [-30.0, 0.0, -20.0],
        [0.0, 0.0, -35.0],
        [30.0, 0.0, -25.0],
        [40.0, 0.0, 0.0],
        [25.0, 0.0, 25.0],
        [20.0, 0.0, 15.0],
        [13.0, 0.0, 17.0],
        [8.0, 0.0, 13.0],
        [2.0, 0.0, 15.0],
        [-3.0, 0.0, 13.0],
    ];

    #[test]
    fn track_project_round_trips_paths_and_start_finish() {
        let mut project = TrackProject::default_project();
        project.paths.push(PathProject {
            id: "pit".to_string(),
            kind: "pit_lane".to_string(),
            route: vec!["edge-0".to_string()],
            boxes: Vec::new(),
        });
        project.layouts[0].start_finish = StartFinish {
            edge: "edge-0".to_string(),
            fraction: 0.25,
        };
        project.edges[0].points = vec![[1.0, 2.0]];
        project.edges[0].from_tangent = Some([1.0, 0.0]);
        project.edges[0].to_tangent = Some([0.0, 1.0]);

        let json = serde_json::to_string(&project).expect("serialize project");
        let parsed: TrackProject = serde_json::from_str(&json).expect("deserialize project");
        assert_eq!(parsed, project);
    }

    #[test]
    fn track_project_v4_round_trips_paths_and_start_finish() {
        let mut project = TrackProject::default_project();
        project.schema = TRACK_PROJECT_SCHEMA_V4.to_string();
        project.layouts[0].start_finish = StartFinish {
            edge: "edge-0".to_string(),
            fraction: 0.25,
        };
        project.paths.push(PathProject {
            id: "pit".to_string(),
            kind: "pit_lane".to_string(),
            route: vec!["edge-0".to_string(), "edge-1".to_string()],
            boxes: Vec::new(),
        });
        project.edges[0].from_tangent = Some([2.0, 0.0]);

        let json = serde_json::to_string(&project).expect("serialize project");
        let parsed = TrackProject::from_json(&json).expect("parse @4 project");
        assert_eq!(parsed.schema, TRACK_PROJECT_SCHEMA_V4);
        assert_eq!(parsed.layouts[0].start_finish.edge, "edge-0");
        assert!((parsed.layouts[0].start_finish.fraction - 0.25).abs() < 1e-6);
        assert_eq!(parsed.paths, project.paths);
        assert_eq!(parsed.edges[0].from_tangent, Some([1.0, 0.0]));
    }

    #[test]
    fn editor_project_v3_migrates_to_v4_routes_and_paths() {
        let mut editor = EditorProject::default_project();
        let loop_id = editor.default_loop_id().to_string();
        let loop_index = editor.loop_index(&loop_id).expect("default loop");
        editor.control_points.push(EditorControlPoint {
            id: "pit-entry".to_string(),
            position: [10.0, -10.0],
            kind: ControlPointKind::PitEntry,
            tangent: None,
        });
        editor.control_points.push(EditorControlPoint {
            id: "pit-mid".to_string(),
            position: [20.0, -15.0],
            kind: ControlPointKind::Normal,
            tangent: None,
        });
        editor.control_points.push(EditorControlPoint {
            id: "pit-exit".to_string(),
            position: [30.0, -10.0],
            kind: ControlPointKind::PitExit,
            tangent: None,
        });
        let route = &mut editor.loops[loop_index].control_points;
        route.insert(1, "pit-entry".to_string());
        let exit_index = route
            .iter()
            .position(|point| point == "cp-3")
            .expect("cp-3 in loop");
        route.insert(exit_index + 1, "pit-exit".to_string());
        editor.pit_lane = Some(EditorPitLane {
            control_points: vec![
                "pit-entry".to_string(),
                "pit-mid".to_string(),
                "pit-exit".to_string(),
            ],
            boxes: Vec::new(),
        });

        let json = serde_json::to_string(&editor).expect("serialize @3 project");
        let parsed = TrackProject::from_json(&json).expect("migrate @3 to @4");

        assert_eq!(parsed.schema, TRACK_PROJECT_SCHEMA_V4);
        for layout in &parsed.layouts {
            assert!(
                layout.route.contains(&layout.start_finish.edge),
                "layout '{}' start/finish '{}' is not in its route",
                layout.id,
                layout.start_finish.edge
            );
        }
        assert_eq!(parsed.paths.len(), 1);
        assert_eq!(parsed.paths[0].id, "pit");
        assert_eq!(parsed.paths[0].kind, "pit_lane");
        assert_eq!(
            parsed.paths[0].route,
            vec![
                "pit-entry->pit-mid".to_string(),
                "pit-mid->pit-exit".to_string()
            ]
        );
    }

    #[test]
    fn path_route_must_be_a_canonical_chain() {
        let mut project = TrackProject::default_project();
        project.schema = TRACK_PROJECT_SCHEMA_V4.to_string();
        project.layouts[0].start_finish = StartFinish {
            edge: "edge-0".to_string(),
            fraction: 0.0,
        };
        project.paths.push(PathProject {
            id: "pit".to_string(),
            kind: "pit_lane".to_string(),
            route: vec!["edge-0".to_string(), "edge-2".to_string()],
            boxes: Vec::new(),
        });

        let error = project.normalize().expect_err("non-consecutive path chain");
        assert!(error.contains("path"), "unexpected error: {error}");
    }

    #[test]
    fn layout_start_finish_must_reference_a_route_edge() {
        let mut project = fork_project();
        project.schema = TRACK_PROJECT_SCHEMA_V4.to_string();
        let short = project
            .layouts
            .iter_mut()
            .find(|layout| layout.id == "short")
            .expect("short layout");
        short.start_finish = StartFinish {
            edge: "e0".to_string(),
            fraction: 0.0,
        };
        let full = project
            .layouts
            .iter_mut()
            .find(|layout| layout.id == "full")
            .expect("full layout");
        full.start_finish = StartFinish {
            edge: "e4".to_string(),
            fraction: 0.0,
        };
        let error = project.normalize().expect_err("start/finish outside route");
        assert!(
            error.contains("missing from its route"),
            "unexpected error: {error}"
        );

        let mut project = fork_project();
        project.schema = TRACK_PROJECT_SCHEMA_V4.to_string();
        for layout in &mut project.layouts {
            layout.start_finish = StartFinish {
                edge: "e0".to_string(),
                fraction: 0.0,
            };
        }
        project.layouts[0].start_finish.edge = "missing".to_string();
        let error = project.normalize().expect_err("unknown start/finish edge");
        assert!(error.contains("unknown edge"), "unexpected error: {error}");
    }

    fn minimum_radius(points: &[Vec3]) -> f32 {
        let count = points.len();
        (0..count)
            .map(|index| {
                curvature_radius(
                    points[(index + count - 1) % count],
                    points[index],
                    points[(index + 1) % count],
                )
            })
            .fold(f32::INFINITY, f32::min)
    }

    fn segment_crosses(a0: Vec3, a1: Vec3, b0: Vec3, b1: Vec3) -> bool {
        let orientation =
            |a: Vec3, b: Vec3, c: Vec3| (b.x - a.x) * (c.z - a.z) - (b.z - a.z) * (c.x - a.x);
        orientation(a0, a1, b0) * orientation(a0, a1, b1) < 0.0
            && orientation(b0, b1, a0) * orientation(b0, b1, a1) < 0.0
    }

    fn offset_crossings(points: &[Vec3], offset: f32) -> usize {
        let count = points.len();
        let offset_points = (0..count)
            .map(|index| {
                let tangent = (points[(index + 1) % count] - points[index]).normalize_or_zero();
                points[index] + Vec3::new(-tangent.z, 0.0, tangent.x) * offset
            })
            .collect::<Vec<_>>();
        let mut crossings = 0;
        for first in 0..count {
            for second in (first + 2)..count {
                if first == 0 && second == count - 1 {
                    continue;
                }
                if segment_crosses(
                    offset_points[first],
                    offset_points[(first + 1) % count],
                    offset_points[second],
                    offset_points[(second + 1) % count],
                ) {
                    crossings += 1;
                }
            }
        }
        crossings
    }

    #[test]
    fn kerb_spans_can_overlap_across_sides() {
        let spans = sanitize_kerb_spans(
            &[
                KerbSpan {
                    start: 0.0,
                    end: 20.0,
                    side: -1,
                },
                KerbSpan {
                    start: 0.0,
                    end: 20.0,
                    side: 1,
                },
            ],
            120,
        );
        assert_eq!(spans.len(), 2);
    }

    #[test]
    fn spawn_faces_increasing_track_direction_after_drive_flip() {
        let track = track_from_control_points_with_spans(&CONTROL_POINTS, None, None)
            .expect("editor track");
        let spawn = &track.metadata.spawns[0];
        let rotation = Quat::from_xyzw(
            spawn.rotation_xyzw[0],
            spawn.rotation_xyzw[1],
            spawn.rotation_xyzw[2],
            spawn.rotation_xyzw[3],
        );
        let drive_rotation = Quat::from_rotation_y(PI) * rotation;
        let forward = drive_rotation * Vec3::NEG_Z;
        let points = sample_track(&CONTROL_POINTS);
        let tangent = (points[1] - points[0]).normalize_or_zero();
        assert!(forward.dot(tangent) > 0.99);
    }

    #[test]
    fn editor_track_has_a_mountain_backdrop() {
        let track = track_from_control_points_with_spans(&CONTROL_POINTS, None, None)
            .expect("editor track");
        let index = track
            .visual
            .meshes
            .iter()
            .position(|mesh| mesh.name == "mountains")
            .expect("mountain mesh");
        let mesh = &track.visual.meshes[index];
        assert_eq!(
            track.visual.materials[mesh.material as usize].name,
            "mountain"
        );
        assert!(track.visual.mesh_buffers[index].indices.len() / 3 > 10_000);
        assert!(!track.visual.mesh_buffers[index].positions.is_empty());
        assert_eq!(
            track.visual.mesh_buffers[index].colors.len(),
            track.visual.mesh_buffers[index].positions.len()
        );
    }

    #[test]
    fn track_surface_uses_one_continuous_ribbon() {
        let track = track_from_control_points_with_spans(&CONTROL_POINTS, None, None)
            .expect("editor track");
        let surface_meshes = track
            .visual
            .meshes
            .iter()
            .filter(|mesh| mesh.name == "asphalt_dither")
            .count();
        assert_eq!(surface_meshes, 1);
        assert!(!track
            .visual
            .meshes
            .iter()
            .any(|mesh| mesh.name.starts_with("dither_shoulder")));
        assert!(track.collision.surface_ids.contains(&TRACK_SURFACE));
        assert!(track.collision.surface_ids.contains(&DIRT_SURFACE));
    }

    fn signed_turn(points: &[Vec3], index: usize) -> f32 {
        let count = points.len();
        let previous = points[(index + count - 1) % count];
        let current = points[index];
        let next = points[(index + 1) % count];
        let incoming = Vec3::new(current.x - previous.x, 0.0, current.z - previous.z);
        let outgoing = Vec3::new(next.x - current.x, 0.0, next.z - current.z);
        incoming.x * outgoing.z - incoming.z * outgoing.x
    }

    #[test]
    fn racing_line_offsets_stay_inside_the_asphalt() {
        for control_points in [&CONTROL_POINTS[..], &TIGHT_CONTROL_POINTS[..]] {
            let points = sample_track(control_points);
            let offsets = racing_line_offsets(&points);
            assert_eq!(offsets.len(), points.len());
            assert!(offsets.iter().all(|offset| offset.is_finite()));
            assert!(offsets
                .iter()
                .all(|offset| offset.abs() <= RACING_LINE_MAX_OFFSET + 1e-5));
        }
    }

    #[test]
    fn racing_line_biases_into_corners_and_centres_on_straights() {
        let points = sample_track(&CONTROL_POINTS);
        let offsets = racing_line_offsets(&points);
        let count = points.len();
        let (corner, turn) = (0..count)
            .map(|index| (index, signed_turn(&points, index)))
            .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
            .expect("a corner");
        assert!(turn.abs() > 1e-3);
        assert!(
            offsets[corner].abs() > 1.0,
            "corner offset {} is too small",
            offsets[corner]
        );
        assert_eq!(
            offsets[corner].signum(),
            turn.signum(),
            "line should move toward the inside of the corner"
        );

        let (straight, _) = (0..count)
            .map(|index| (index, signed_turn(&points, index).abs()))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .expect("a straight");
        assert!(
            offsets[straight].abs() < 1.0,
            "straight offset {} should stay near centre",
            offsets[straight]
        );
    }

    #[test]
    fn racing_line_mesh_sits_above_asphalt_within_the_lane() {
        let points = sample_track(&CONTROL_POINTS);
        let offsets = racing_line_offsets(&points);
        let buffers = racing_line_ribbon(&points, &offsets);
        let profile_count = RACING_LINE_PROFILE.len();
        assert_eq!(buffers.positions.len(), (points.len() + 1) * profile_count);
        assert_eq!(buffers.colors.len(), buffers.positions.len());
        assert_eq!(buffers.normals.len(), buffers.positions.len());
        assert_eq!(buffers.uvs.len(), buffers.positions.len());
        assert_eq!(
            buffers.indices.len() / 3,
            points.len() * (profile_count - 1) * 2
        );

        for station in 0..points.len() {
            let center = points[station];
            let binormal = binormal_at(&points, station);
            let left = center - binormal * ASPHALT_HALF_WIDTH;
            let right = center + binormal * ASPHALT_HALF_WIDTH;
            let left_y = terrain_height(left.x, left.z) + SURFACE_HEIGHT_OFFSET;
            let right_y = terrain_height(right.x, right.z) + SURFACE_HEIGHT_OFFSET;
            for column in 0..profile_count {
                let position =
                    Vec3::from_array(buffers.positions[station * profile_count + column]);
                let lateral = (position - center).dot(binormal);
                assert!(
                    lateral.abs() <= ASPHALT_HALF_WIDTH,
                    "racing line vertex outside asphalt: {lateral}"
                );
                let t =
                    ((lateral + ASPHALT_HALF_WIDTH) / (2.0 * ASPHALT_HALF_WIDTH)).clamp(0.0, 1.0);
                let asphalt_y = left_y + (right_y - left_y) * t;
                assert!(
                    position.y > asphalt_y,
                    "racing line vertex is not above the asphalt: {} <= {}",
                    position.y,
                    asphalt_y
                );
                assert!((position.y - asphalt_y - RACING_LINE_LIFT).abs() < 1e-4);
            }
        }

        let first_station = &buffers.colors[..profile_count];
        assert!(first_station[profile_count / 2][0] < first_station[0][0]);
        let expected = Vec3::from_array(RACING_LINE_ASPHALT_COLOR) * TARMAC_GAIN;
        let actual = Vec3::from_array(first_station[0]);
        assert!((actual - expected).length() < 1e-5);
    }

    #[test]
    fn editor_track_includes_a_racing_line_mesh() {
        let track = track_from_control_points_with_spans(&CONTROL_POINTS, None, None)
            .expect("editor track");
        let index = track
            .visual
            .meshes
            .iter()
            .position(|mesh| mesh.name == "racing_line")
            .expect("racing line mesh");
        let mesh = &track.visual.meshes[index];
        assert_eq!(
            track.visual.materials[mesh.material as usize].name,
            "racing_line"
        );
        let buffers = &track.visual.mesh_buffers[index];
        assert!(!buffers.positions.is_empty());
        assert!(!buffers.indices.is_empty());
        assert_eq!(buffers.colors.len(), buffers.positions.len());
        assert!(track
            .metadata
            .surfaces
            .iter()
            .all(|surface| surface.name != "racing_line"));
    }

    #[test]
    fn tarmac_texture_is_deterministic_and_opaque() {
        let first = tarmac_texture();
        let second = tarmac_texture();
        assert_eq!(first, second);
        assert_eq!(first.name, "tarmac_procedural");
        assert_eq!(first.width as usize, TARMAC_TEXTURE_SIZE);
        assert_eq!(first.height as usize, TARMAC_TEXTURE_SIZE);
        assert_eq!(
            first.rgba8.len(),
            TARMAC_TEXTURE_SIZE * TARMAC_TEXTURE_SIZE * 4
        );
        let (pixels, remainder) = first.rgba8.as_chunks::<4>();
        assert!(remainder.is_empty());
        assert!(pixels.iter().all(|pixel| pixel[3] == 255));
        assert!(pixels
            .iter()
            .any(|pixel| pixel[0] != pixel[1] || pixel[1] != pixel[2]));
        let min = pixels.iter().map(|pixel| pixel[0]).min().unwrap_or(0);
        let max = pixels.iter().map(|pixel| pixel[0]).max().unwrap_or(0);
        assert!(
            (15..70).contains(&(max - min)),
            "tarmac texture contrast is not subtle: {min}..{max}"
        );
    }

    #[test]
    fn grass_texture_is_deterministic_subtle_and_opaque() {
        let first = grass_texture();
        let second = grass_texture();
        assert_eq!(first, second);
        assert_eq!(first.name, "grass_procedural");
        assert_eq!(first.width as usize, GRASS_TEXTURE_SIZE);
        assert_eq!(first.height as usize, GRASS_TEXTURE_SIZE);
        assert_eq!(
            first.rgba8.len(),
            GRASS_TEXTURE_SIZE * GRASS_TEXTURE_SIZE * 4
        );
        let (pixels, remainder) = first.rgba8.as_chunks::<4>();
        assert!(remainder.is_empty());
        assert!(pixels.iter().all(|pixel| pixel[3] == 255));
        let min = pixels.iter().map(|pixel| pixel[1]).min().unwrap_or(0);
        let max = pixels.iter().map(|pixel| pixel[1]).max().unwrap_or(0);
        assert!(
            (10..60).contains(&(max - min)),
            "grass texture variation is not subtle: {min}..{max}"
        );
        assert!(pixels.iter().all(|pixel| pixel[1] >= pixel[0]));
        assert!(pixels.iter().all(|pixel| pixel[0] >= pixel[2]));
    }

    #[test]
    fn tarmac_noise_wraps_at_the_tile_edge() {
        for v in [0.0, 0.25, 0.5, 0.75] {
            let left = tarmac_value_noise(0.0, v, 18, 0x5e6f_7a8b);
            let right = tarmac_value_noise(1.0, v, 18, 0x5e6f_7a8b);
            assert!((left - right).abs() < 1e-5);
        }
    }

    #[test]
    fn editor_track_assigns_the_tarmac_texture() {
        let track = track_from_control_points_with_spans(&CONTROL_POINTS, None, None)
            .expect("editor track");
        let tarmac_index = track
            .visual
            .textures
            .iter()
            .position(|texture| texture.name == "tarmac_procedural")
            .expect("tarmac texture");
        let material = &track.visual.materials[1];
        assert_eq!(material.name, "dither_shoulder");
        assert_eq!(material.base_color_texture, Some(tarmac_index as u32));
        let racing = track
            .visual
            .materials
            .iter()
            .find(|material| material.name == "racing_line")
            .expect("racing line material");
        assert_eq!(racing.base_color_texture, Some(tarmac_index as u32));
        let grass = track
            .visual
            .materials
            .iter()
            .find(|material| material.name == "grass")
            .expect("grass material");
        assert_eq!(grass.base_color_texture, Some(GRASS_TEXTURE_INDEX));
        let terrain = track
            .visual
            .meshes
            .iter()
            .position(|mesh| mesh.name == "terrain")
            .expect("terrain mesh");
        let terrain_uvs = &track.visual.mesh_buffers[terrain].uvs;
        assert!(terrain_uvs.iter().any(|uv| uv[0] > 1.0));
        assert!(terrain_uvs.iter().any(|uv| uv[1] > 1.0));
        assert_eq!(
            track.manifest.stats.textures,
            track.visual.textures.len() as u16
        );
        let road = track
            .visual
            .meshes
            .iter()
            .position(|mesh| mesh.name == "asphalt_dither")
            .expect("road mesh");
        assert_eq!(track.visual.meshes[road].material, 1);
        let uvs = &track.visual.mesh_buffers[road].uvs;
        assert!(uvs.iter().any(|uv| uv[0] > 2.0));
        assert!(uvs.iter().any(|uv| uv[1] > 2.0));
    }

    #[test]
    fn editor_shoulders_collide_as_dirt() {
        use sim_core::TrackCollider;

        let points = sample_track(&CONTROL_POINTS);
        let track = track_from_control_points_with_spans(&CONTROL_POINTS, None, None)
            .expect("editor track");
        let collider = TrackCollider::from_collision(&track.collision, &track.metadata);
        let (point, binormal) = track_frame(&points, 10.0);
        for lateral in [7.0, 8.0, 9.0, 10.0, 11.0] {
            for side in [-1.0, 1.0] {
                let sample = point + binormal * (lateral * side);
                let hit = collider
                    .raycast(
                        Vec3::new(sample.x, sample.y + 20.0, sample.z),
                        Vec3::NEG_Y,
                        100.0,
                    )
                    .unwrap_or_else(|| panic!("no hit at lateral {lateral} side {side}"));
                assert_eq!(
                    hit.surface, DIRT_SURFACE,
                    "lateral {lateral} side {side} hit surface {}",
                    hit.surface
                );
            }
        }
    }

    #[test]
    fn editor_track_exports_a_valid_pack() {
        let track = track_from_control_points_with_spans(&CONTROL_POINTS, None, None)
            .expect("editor track");
        let bytes = pack_from_track(&track).expect("export pack");
        let pack = sim_asset::Pack::from_bytes(&bytes).expect("parse pack");
        assert_eq!(pack.kind(), sim_types::AssetKind::Track);
        assert!(pack.chunk_bytes(sim_types::ChunkKind::Visual).is_ok());
        assert!(pack.chunk_bytes(sim_types::ChunkKind::Collision).is_ok());
        assert!(pack.chunk_bytes(sim_types::ChunkKind::Metadata).is_ok());
        let metadata = pack.track_metadata_owned().expect("metadata");
        assert!(metadata
            .surfaces
            .iter()
            .any(|surface| surface.name == "kerb"));
        assert!(metadata
            .surfaces
            .iter()
            .any(|surface| surface.name == "wall"));
        let collision = pack.track_collision_owned().expect("collision");
        assert!(collision.surface_ids.contains(&KERB_SURFACE));
        assert!(collision.surface_ids.contains(&WALL_SURFACE));
        assert_eq!(pack.track_visual_owned().expect("visual"), track.visual);
    }

    #[test]
    fn prop_lines_use_simplified_wall_collision() {
        use sim_core::TrackCollider;

        let points = sample_track(&CONTROL_POINTS);
        let track = track_from_control_points_with_spans(&CONTROL_POINTS, None, None)
            .expect("editor track");
        let collider = TrackCollider::from_collision(&track.collision, &track.metadata);
        let lines = default_prop_lines(&points);
        assert!(lines.iter().any(|line| line.kind != PropKind::Off));
        for line in lines
            .into_iter()
            .filter(|line| prop_wall_dimensions(line.kind).is_some())
        {
            let (half_width, _) = prop_wall_dimensions(line.kind).expect("prop wall");
            let side = line.side as f32;
            let (point, binormal) = track_frame(&points, (line.start + line.end) * 0.5);
            let center = point + binormal * (line.offset * side);
            let contact = |position: Vec3, radius: f32| {
                collider.wall_contacts(
                    Vec3::new(
                        position.x,
                        terrain_height(position.x, position.z) + 0.4,
                        position.z,
                    ),
                    radius,
                )
            };
            assert!(contact(center, 0.2).is_empty());
            let inner_face = center - binormal * (half_width * side);
            assert!(!contact(inner_face - binormal * (side * 0.1), 0.2).is_empty());
            assert!(contact(inner_face - binormal * (side * 0.8), 0.2).is_empty());
        }
    }

    #[test]
    fn kerb_top_is_driveable_collision() {
        use sim_core::TrackCollider;

        let points = sample_track(&CONTROL_POINTS);
        let spans = [KerbSpan {
            start: 10.25,
            end: 10.75,
            side: 1,
        }];
        let track = track_from_control_points_with_spans(&CONTROL_POINTS, Some(&spans), Some(&[]))
            .expect("editor track");
        assert!(track
            .metadata
            .surfaces
            .iter()
            .any(|surface| surface.name == "kerb"));
        let collider = TrackCollider::from_collision(&track.collision, &track.metadata);

        let (point, binormal) = track_frame(&points, 10.5);
        let sample = point + binormal * 6.4;
        let hit = collider
            .raycast(
                Vec3::new(sample.x, sample.y + 20.0, sample.z),
                Vec3::NEG_Y,
                50.0,
            )
            .expect("kerb raycast");
        let kerb = track
            .metadata
            .surfaces
            .iter()
            .find(|surface| surface.name == "kerb")
            .expect("kerb surface");
        assert_eq!(hit.surface, KERB_SURFACE);
        assert!((hit.friction - kerb.friction).abs() < 1e-5);
        assert!(hit.point.y > terrain_height(hit.point.x, hit.point.z) + 0.07);
    }

    #[test]
    fn jagged_kerb_segments_are_closed_by_risers() {
        let points = sample_track(&CONTROL_POINTS);
        let index = 10.0;
        let (start, start_binormal) = track_frame(&points, index);
        let (middle, middle_binormal) = track_frame(&points, index + 1.0);
        let (end, end_binormal) = track_frame(&points, index + 2.0);
        let mut buffers = empty_buffers();
        let (_, first_end) = append_kerb(
            &mut buffers,
            start,
            middle,
            start_binormal,
            middle_binormal,
            1.0,
        );
        let (second_start, _) = append_kerb(
            &mut buffers,
            middle,
            end,
            middle_binormal,
            end_binormal,
            1.0,
        );
        assert!(first_end[1].y > second_start[1].y + 0.01);
        let before = buffers.positions.len();
        append_kerb_riser(&mut buffers, first_end, second_start, end - middle);
        assert_eq!(buffers.positions.len(), before + 3);
        let riser = &buffers.positions[before..];
        assert!(riser
            .iter()
            .any(|position| Vec3::from_array(*position).distance(first_end[1]) < 1e-5));
        assert!(riser
            .iter()
            .any(|position| Vec3::from_array(*position).distance(second_start[1]) < 1e-5));
    }

    #[test]
    fn auto_kerbs_cover_only_split_severe_bends() {
        let points = sample_track(&CONTROL_POINTS);
        let spans = auto_kerb_spans(&points);
        assert!(!spans.is_empty());
        assert!(
            spans.len() <= DEFAULT_KERB_BENDS * DEFAULT_KERB_SEGMENTS_PER_BEND,
            "unexpected kerb span count: {}",
            spans.len()
        );
        let covered = spans.iter().map(|span| span.end - span.start).sum::<f32>();
        assert!(covered < points.len() as f32 * 0.6);
        assert!(spans
            .windows(2)
            .any(|pair| { pair[0].side == pair[1].side && pair[1].start - pair[0].end > 0.5 }));
    }

    #[test]
    fn low_speed_turn_can_cross_the_kerb() {
        use sim_core::{Controls, Vehicle};

        let spans = [
            KerbSpan {
                start: 0.0,
                end: 120.0,
                side: -1,
            },
            KerbSpan {
                start: 0.0,
                end: 120.0,
                side: 1,
            },
        ];
        let track = track_from_control_points_with_spans(&CONTROL_POINTS, Some(&spans), Some(&[]))
            .expect("editor track");
        let collider = sim_core::TrackCollider::from_collision(&track.collision, &track.metadata);
        let car_bytes = include_bytes!("../../../fixtures/cars/mazda-rx7/mazda-rx7.sima").to_vec();
        let pack = sim_asset::Pack::from_bytes(&car_bytes).expect("car pack");
        let physics = pack.car_physics_owned().expect("car physics");
        let mut spawn = track.metadata.spawns[0].clone();
        let spawn_rotation = Quat::from_xyzw(
            spawn.rotation_xyzw[0],
            spawn.rotation_xyzw[1],
            spawn.rotation_xyzw[2],
            spawn.rotation_xyzw[3],
        );
        spawn.rotation_xyzw = (Quat::from_rotation_y(PI) * spawn_rotation).to_array();
        let mut vehicle = Vehicle::new(physics, collider, &spawn);
        for _ in 0..500 {
            vehicle.step(Controls::default());
        }
        let accelerate = Controls {
            throttle: 0.35,
            gear: 1,
            ..Controls::default()
        };
        for _ in 0..1000 {
            vehicle.step(accelerate);
        }
        let turn = Controls {
            throttle: 0.35,
            steering: 0.9,
            gear: 1,
            ..Controls::default()
        };
        let mut touched_kerb = false;
        for _ in 0..2500 {
            vehicle.step(turn);
            touched_kerb |= vehicle
                .state
                .wheels
                .iter()
                .any(|wheel| wheel.surface_id == KERB_SURFACE);
        }
        assert!(touched_kerb);
        assert!(
            vehicle.state.velocity.length() * 3.6 > 10.0,
            "car stalled on the kerb: {} kph",
            vehicle.state.velocity.length() * 3.6
        );
    }

    #[test]
    fn tight_control_points_are_smoothed_to_the_minimum_radius() {
        let points = sample_track(&TIGHT_CONTROL_POINTS);
        assert!(minimum_radius(&points) >= MIN_TURN_RADIUS - 0.01);
        assert_eq!(offset_crossings(&points, OUTER_EDGE_RADIUS), 0);
        assert_eq!(offset_crossings(&points, -OUTER_EDGE_RADIUS), 0);
        assert_eq!(offset_crossings(&points, PROP_OFFSET), 0);
        assert_eq!(offset_crossings(&points, -PROP_OFFSET), 0);
        assert_eq!(offset_crossings(&points, PROP_FAR_OFFSET), 0);
        assert_eq!(offset_crossings(&points, -PROP_FAR_OFFSET), 0);
    }

    #[test]
    fn prop_spacing_is_uniform_along_offset_paths() {
        let points = sample_track(&TIGHT_CONTROL_POINTS);
        for line in default_prop_lines(&points) {
            let spacing = match line.kind {
                PropKind::Blocks => BLOCK_SPACING,
                PropKind::Tyres => TYRE_SPACING,
                PropKind::Trees => TREE_SPACING,
                PropKind::Off => continue,
            };
            let placements = prop_placements(&points, &line);
            assert!(
                !placements.is_empty(),
                "line {:?} produced no placements",
                line
            );
            let tolerance = if line.kind == PropKind::Trees {
                0.15
            } else {
                0.03
            };
            for pair in placements.windows(2) {
                let distance = pair[0].0.distance(pair[1].0);
                assert!(
                    (distance - spacing).abs() <= spacing * tolerance,
                    "prop spacing drifted: expected {spacing}, got {distance}"
                );
            }
        }
    }

    #[test]
    fn tree_prop_lines_build_canopies_and_trunk_collision() {
        let lines = [PropLine {
            start: 10.0,
            end: 18.0,
            side: 1,
            offset: PROP_FAR_OFFSET,
            kind: PropKind::Trees,
        }];
        let track = track_from_control_points_with_spans(&CONTROL_POINTS, Some(&[]), Some(&lines))
            .expect("editor track");
        let mesh = |name: &str| {
            track
                .visual
                .meshes
                .iter()
                .position(|mesh| mesh.name == name)
                .and_then(|index| track.visual.mesh_buffers.get(index))
                .unwrap_or_else(|| panic!("missing {name} mesh"))
        };
        for name in [
            "tree_trunks_high",
            "tree_foliage_high",
            "tree_trunks_low",
            "tree_foliage_low",
            "tree_cards",
        ] {
            assert!(!mesh(name).positions.is_empty(), "empty {name} mesh");
            assert_eq!(
                mesh(name).colors.len(),
                mesh(name).positions.len(),
                "{name} colors"
            );
        }
        let placements = prop_placements(&sample_track(&CONTROL_POINTS), &lines[0]).len();
        assert!(placements > 0);
        let triangles = |name: &str| mesh(name).indices.len() / 3;
        let high = triangles("tree_trunks_high") + triangles("tree_foliage_high");
        let low = triangles("tree_trunks_low") + triangles("tree_foliage_low");
        assert!(high > low);
        assert!(
            high <= placements * 120,
            "high tree budget exceeded: {high} tris for {placements} trees"
        );
        assert!(
            high >= placements * 50,
            "high tree detail too low: {high} tris for {placements} trees"
        );
        assert!(!track.visual.textures.is_empty());
        assert!(!mesh("tree_cards").uvs.is_empty());
        let cards = triangles("tree_cards");
        assert!(
            cards <= placements * 8,
            "card tree budget exceeded: {cards} tris for {placements} trees"
        );
        assert!(track.collision.surface_ids.contains(&WALL_SURFACE));
        assert!(track
            .visual
            .meshes
            .iter()
            .all(|mesh| mesh.name != "barriers" && !mesh.name.starts_with("tires_")));
    }

    #[test]
    fn fractional_prop_range_follows_the_offset_path() {
        let points = sample_track(&CONTROL_POINTS);
        let line = PropLine {
            start: 10.25,
            end: 12.75,
            side: 1,
            offset: PROP_FAR_OFFSET,
            kind: PropKind::Blocks,
        };
        let vertices = offset_path_vertices(
            &points,
            line.start,
            line.end,
            line.offset * line.side as f32,
        );
        assert_eq!(vertices.len(), 4);
        let (start, start_binormal) = track_frame(&points, line.start);
        let (end, end_binormal) = track_frame(&points, line.end);
        assert!(vertices[0].distance(start + start_binormal * line.offset) < 1e-3);
        assert!(
            vertices
                .last()
                .expect("end vertex")
                .distance(end + end_binormal * line.offset)
                < 1e-3
        );
        assert!(!prop_placements(&points, &line).is_empty());
    }

    #[test]
    fn prop_jitter_is_tiny_and_deterministic() {
        let points = sample_track(&CONTROL_POINTS);
        let mut moved = false;
        for line in default_prop_lines(&points) {
            for (index, (position, tangent)) in
                prop_placements(&points, &line).into_iter().enumerate()
            {
                let (jittered, heading) = jittered_prop_placement(position, tangent, &line, index);
                let (again, again_heading) =
                    jittered_prop_placement(position, tangent, &line, index);
                let offset = jittered.distance(position);
                moved |= offset > 1e-4;
                assert!(offset <= 0.05);
                assert_eq!(jittered, again);
                assert_eq!(heading, again_heading);
            }
        }
        assert!(moved);
    }

    #[test]
    fn default_track_meets_the_minimum_radius() {
        let points = sample_track(&CONTROL_POINTS);
        assert!(minimum_radius(&points) >= MIN_TURN_RADIUS - 0.01);
        assert_eq!(offset_crossings(&points, OUTER_EDGE_RADIUS), 0);
        assert_eq!(offset_crossings(&points, -OUTER_EDGE_RADIUS), 0);
        assert_eq!(offset_crossings(&points, PROP_OFFSET), 0);
        assert_eq!(offset_crossings(&points, -PROP_OFFSET), 0);
        assert_eq!(offset_crossings(&points, PROP_FAR_OFFSET), 0);
        assert_eq!(offset_crossings(&points, -PROP_FAR_OFFSET), 0);
    }

    fn road_mesh_extent(track: &EditorTrack) -> f32 {
        let index = track
            .visual
            .meshes
            .iter()
            .position(|mesh| mesh.name == "asphalt_dither")
            .expect("asphalt mesh");
        let buffers = &track.visual.mesh_buffers[index];
        let rows = buffers.positions.len() / 4;
        (0..rows)
            .map(|row| {
                Vec3::from_array(buffers.positions[row * 4])
                    .distance(Vec3::from_array(buffers.positions[row * 4 + 3]))
            })
            .fold(0.0, f32::max)
    }

    #[test]
    fn default_project_round_trips_through_json() {
        let project = TrackProject::default_project();
        let json = serde_json::to_string(&project).expect("serialize project");
        let parsed = TrackProject::from_json(&json).expect("parse project");
        let expected = project.clone().normalize().expect("normalize project");
        assert_eq!(parsed, expected);
        assert_eq!(
            serde_json::to_value(&parsed).expect("parsed json value"),
            serde_json::to_value(&expected).expect("expected json value")
        );
        assert_eq!(expected.schema, TRACK_PROJECT_SCHEMA_V4);
        assert_eq!(expected.layouts.len(), 1);
    }

    #[test]
    fn default_project_includes_default_prop_splines() {
        let project = TrackProject::default_project();
        let splines = &project.settings.splines;
        let first_prop = splines
            .iter()
            .position(|spline| spline.role == SplineRole::Prop)
            .expect("prop spline");
        assert!(splines[..first_prop]
            .iter()
            .all(|spline| spline.role == SplineRole::Surface));
        let prop_ids = splines[first_prop..]
            .iter()
            .map(|spline| spline.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            prop_ids,
            [
                "prop-near-left",
                "prop-far-left",
                "prop-near-right",
                "prop-far-right"
            ]
        );
        assert_eq!(splines[first_prop].base.as_deref(), Some("runoff-left"));
        assert_eq!(splines[first_prop].offset, -2.0);
    }

    #[test]
    fn default_project_restores_default_kerb_and_prop_geometry() {
        let track = track_from_project(&TrackProject::default_project()).expect("default track");
        assert!(
            track.visual.meshes.len() >= 7,
            "default project should produce at least 7 visual meshes, got {}",
            track.visual.meshes.len()
        );
        for name in [
            "kerb_red",
            "kerb_white",
            "barriers",
            "tree_trunks_high",
            "tree_foliage_high",
            "tree_trunks_low",
            "tree_foliage_low",
            "tree_cards",
        ] {
            assert!(
                track.visual.meshes.iter().any(|mesh| mesh.name == name),
                "default project is missing {name}"
            );
        }

        let kerb_indices = |track: &EditorTrack| {
            track
                .visual
                .meshes
                .iter()
                .filter(|mesh| matches!(mesh.name.as_str(), "kerb_red" | "kerb_white"))
                .map(|mesh| mesh.index_count as usize)
                .sum::<usize>()
        };
        let prop_indices = |track: &EditorTrack| {
            track
                .visual
                .meshes
                .iter()
                .filter(|mesh| {
                    matches!(
                        mesh.name.as_str(),
                        "barriers" | "tires_blue" | "tires_white"
                    ) || mesh.name.starts_with("tree_")
                })
                .map(|mesh| mesh.index_count as usize)
                .sum::<usize>()
        };

        let mut trimmed = TrackProject::default_project();
        trimmed.edges[0].kerbs.push(KerbSpan {
            start: 0.1,
            end: 0.2,
            side: 1,
        });
        trimmed.edges[0].prop_spans.push(PropSpan {
            spline: "prop-near-left".to_string(),
            start: 0.1,
            end: 0.2,
            kind: PropKind::Blocks,
        });
        let trimmed = track_from_project(&trimmed).expect("trimmed track");
        assert!(kerb_indices(&trimmed) < kerb_indices(&track));
        assert!(prop_indices(&trimmed) < prop_indices(&track));
    }

    #[test]
    fn default_project_synthesizes_props_once_per_route() {
        let project = TrackProject::default_project();
        let layout = &project.layouts[0];
        let edge_ids = layout.route.iter().cloned().collect::<BTreeSet<_>>();
        let built = build_unique_edges(&project, &edge_ids).expect("built edges");
        let route =
            assemble_route(&project, layout, &built, &HashMap::new()).expect("default route");
        let width = global_asphalt_width(&project);
        let splines = surface_splines(&project);
        let spans = synthesize_default_prop_spans(&project, &route, &built, &splines, width);
        let total = spans.values().map(Vec::len).sum::<usize>();
        let per_edge_count = DEFAULT_NEAR_PROP_SEGMENTS * 2 + DEFAULT_TREE_RING_SEGMENTS * 2;
        let old_per_edge_total = layout.route.len() * per_edge_count;
        assert!(total > 0, "default route should synthesize prop spans");
        assert!(
            total < old_per_edge_total,
            "synthesized {total} spans should be below the per-edge count {old_per_edge_total}"
        );

        let outside_side = route_outside_side(&route.points);
        let span_sides = spans
            .values()
            .flatten()
            .filter_map(|span| {
                project
                    .settings
                    .splines
                    .iter()
                    .find(|spline| spline.id == span.spline)
            })
            .map(|spline| {
                if prop_spline_lateral(&project.settings.splines, spline) >= 0.0 {
                    1
                } else {
                    -1
                }
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(
            span_sides,
            BTreeSet::from([outside_side]),
            "synthesized props should only occupy the outside of the route"
        );

        let tree_count = spans
            .values()
            .flatten()
            .filter(|span| span.kind == PropKind::Trees)
            .count();
        assert!(tree_count > 0, "default route should synthesize trees");

        let track = track_from_project(&project).expect("default track");
        for name in ["tree_trunks_high", "tree_foliage_high"] {
            assert!(
                track.visual.meshes.iter().any(|mesh| mesh.name == name),
                "default project is missing {name}"
            );
        }
    }

    #[test]
    fn default_synthesized_props_clear_non_local_route_samples() {
        let project = TrackProject::default_project();
        let layout = &project.layouts[0];
        let edge_ids = layout.route.iter().cloned().collect::<BTreeSet<_>>();
        let built = build_unique_edges(&project, &edge_ids).expect("built edges");
        let route = assemble_route(&project, layout, &built, &HashMap::new()).expect("route");
        let splines = surface_splines(&project);
        let width = global_asphalt_width(&project);
        let spans_by_edge =
            synthesize_default_prop_spans(&project, &route, &built, &splines, width);

        let mut span_count = 0;
        let mut tree_count = 0;
        for (edge_id, range_start, range_end) in &route.edge_ranges {
            let Some(spans) = spans_by_edge.get(edge_id) else {
                continue;
            };
            let edge = built.get(edge_id).expect("built edge");
            let skip_start = range_start.saturating_sub(PROP_ROUTE_CLEARANCE_MARGIN);
            let skip_end =
                (range_end + PROP_ROUTE_CLEARANCE_MARGIN).min(route.points.len().saturating_sub(1));
            for span in spans {
                span_count += 1;
                if span.kind == PropKind::Trees {
                    tree_count += 1;
                }
                let spline = project
                    .settings
                    .splines
                    .iter()
                    .find(|spec| spec.id == span.spline)
                    .expect("prop spline");
                let points =
                    prop_span_world_points(edge, &splines, spline, span).expect("world points");
                for (index, route_point) in route.points.iter().enumerate() {
                    if index >= skip_start && index <= skip_end {
                        continue;
                    }
                    for point in points {
                        let dx = point.x - route_point.x;
                        let dz = point.z - route_point.z;
                        assert!(
                            dx * dx + dz * dz >= width * width - 1e-3,
                            "synthesized {:?} span on '{edge_id}' is {:?} from non-local route sample {index}",
                            span.kind,
                            (dx * dx + dz * dz).sqrt()
                        );
                    }
                }
            }
        }
        assert!(
            span_count > 0,
            "default route should keep some synthesized props where safe"
        );
        assert!(
            tree_count > 0,
            "default route should keep some synthesized trees where safe"
        );

        // Authored prop spans bypass synthesis, so both sides stay unfiltered.
        let mut authored = project.clone();
        for spline in ["prop-near-left", "prop-near-right"] {
            authored.edges[0].prop_spans.push(PropSpan {
                spline: spline.to_string(),
                start: 0.1,
                end: 0.3,
                kind: PropKind::Blocks,
            });
        }
        let authored = authored.normalize().expect("normalize authored project");
        assert_eq!(
            authored.edges[0].prop_spans.len(),
            2,
            "authored prop spans on both sides should be preserved"
        );
        let authored_track = track_from_project(&authored).expect("authored track");
        let barrier_indices = authored_track
            .visual
            .meshes
            .iter()
            .filter(|mesh| mesh.name == "barriers")
            .map(|mesh| mesh.index_count as usize)
            .sum::<usize>();
        assert!(
            barrier_indices > 0,
            "authored props on both sides should still render"
        );
    }

    #[test]
    fn authored_prop_spans_on_both_sides_still_render() {
        let barrier_indices = |track: &EditorTrack| {
            track
                .visual
                .meshes
                .iter()
                .filter(|mesh| mesh.name == "barriers")
                .map(|mesh| mesh.index_count as usize)
                .sum::<usize>()
        };

        let mut left = TrackProject::default_project();
        left.edges[0].prop_spans.push(PropSpan {
            spline: "prop-near-left".to_string(),
            start: 0.1,
            end: 0.3,
            kind: PropKind::Blocks,
        });
        let left_track = track_from_project(&left).expect("left authored track");
        let left_indices = barrier_indices(&left_track);
        assert!(left_indices > 0, "authored prop on the left should render");

        let mut both = left.clone();
        both.edges[0].prop_spans.push(PropSpan {
            spline: "prop-near-right".to_string(),
            start: 0.1,
            end: 0.3,
            kind: PropKind::Blocks,
        });
        let both_track = track_from_project(&both).expect("both authored track");
        let both_indices = barrier_indices(&both_track);
        assert!(
            both_indices > left_indices,
            "authored prop on the right should render alongside the left one"
        );
    }

    #[test]
    fn default_layout_prop_lines_are_one_route_pass() {
        let project = TrackProject::default_project();
        let layout = &project.layouts[0];
        let lines = layout_prop_lines(&project, layout).expect("default prop lines");
        let points = layout_route_points(&project, layout);
        let width = global_asphalt_width(&project);
        let expected = default_prop_lines_with_offsets(
            &points,
            default_prop_offset(width),
            default_prop_far_offset(width),
        );
        assert_eq!(
            lines, expected,
            "default prop lines should be generated once over the route"
        );
        let per_edge_count = DEFAULT_NEAR_PROP_SEGMENTS * 2 + DEFAULT_TREE_RING_SEGMENTS * 2;
        assert!(
            lines.len() < layout.route.len() * per_edge_count,
            "default prop line count should not be multiplied by the edge count"
        );

        let mut seen = BTreeSet::new();
        for line in &lines {
            let key = (
                line.start.to_bits(),
                line.end.to_bits(),
                line.side,
                line.offset.to_bits(),
                format!("{:?}", line.kind),
            );
            assert!(seen.insert(key), "duplicate default prop line: {line:?}");
        }
    }

    #[test]
    fn single_loop_fast_path_default_props_stay_outside() {
        let editor = EditorProject::default_project();
        let graph = editor.to_graph().expect("default graph");
        let default_loop = editor.loops[0].id.clone();
        let legacy = legacy_from_project(&graph, Some(&default_loop)).expect("legacy project");
        let layout = &legacy.layouts[0];
        let points = sample_track_with_min_radius(
            &layout_control_points_with_heights(&layout.control_points),
            min_turn_radius(layout.width),
        );
        let outside_side = route_outside_side(&points);
        let lines = layout
            .prop_lines
            .as_ref()
            .expect("single loop should synthesize default props");
        assert!(!lines.is_empty(), "default props should not be empty");
        assert!(
            lines.iter().all(|line| line.side == outside_side),
            "default single-loop props must only occupy the outside side"
        );

        // The single-loop editor path must keep rendering with authored props
        // on either side untouched.
        let mut authored = TrackProject::default_project();
        let route = authored.layouts[0].route.clone();
        authored
            .edges
            .iter_mut()
            .find(|edge| edge.id == route[0])
            .expect("first route edge")
            .prop_spans
            .push(PropSpan {
                spline: "prop-near-left".to_string(),
                start: 0.1,
                end: 0.4,
                kind: PropKind::Blocks,
            });
        authored
            .edges
            .iter_mut()
            .find(|edge| edge.id == route[1])
            .expect("second route edge")
            .prop_spans
            .push(PropSpan {
                spline: "prop-near-right".to_string(),
                start: 0.5,
                end: 0.8,
                kind: PropKind::Blocks,
            });
        let legacy = legacy_from_project(&authored, None).expect("authored legacy project");
        let sides = legacy.layouts[0]
            .prop_lines
            .as_ref()
            .expect("authored props")
            .iter()
            .map(|line| line.side)
            .collect::<BTreeSet<_>>();
        assert_eq!(
            sides,
            BTreeSet::from([-1, 1]),
            "authored prop lines on either side should be preserved"
        );

        track_from_editor_project(&editor).expect("single-loop fast path track");
    }

    #[test]
    fn setting_a_near_prop_line_round_trips_through_layout_prop_lines() {
        let mut project = TrackProject::default_project();
        let layout = project.layouts[0].clone();
        let count = layout_route_points(&project, &layout).len() as f32;
        let (ranges, route_len) =
            layout_route_ranges(&project, &layout).expect("layout route ranges");
        let last = route_len.saturating_sub(1).max(1) as f32;
        let (first_edge, range_start, range_end) = ranges.first().expect("first route edge");
        let range_start = *range_start as f32;
        let range_len = *range_end as f32 - range_start;
        let route_start = range_start + range_len * 0.1;
        let route_end = range_start + range_len * 0.4;
        let width = global_asphalt_width(&project);
        let line = PropLine {
            start: route_start / last * count,
            end: route_end / last * count,
            side: -1,
            offset: default_prop_offset(width),
            kind: PropKind::Blocks,
        };
        project
            .set_layout_prop_lines(&layout.id, &[line])
            .expect("set prop line");
        let first_edge = first_edge.clone();
        let edge = project.edge_by_id(&first_edge).expect("edge");
        assert_eq!(edge.prop_spans.len(), 1);
        assert_eq!(edge.prop_spans[0].spline, "prop-near-left");
        let lines = layout_prop_lines(&project, &layout).expect("layout prop lines");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].side, -1);
        assert_eq!(lines[0].kind, PropKind::Blocks);
        assert!((lines[0].offset - default_prop_offset(width)).abs() < 1e-4);
        assert!((lines[0].start - line.start).abs() < 1e-3);
        assert!((lines[0].end - line.end).abs() < 1e-3);
    }

    #[test]
    fn trimming_kerb_span_reduces_kerb_vertices() {
        let mut project = TrackProject::default_project();
        let layout = project.layouts[0].clone();
        let spans = layout_kerb_spans(&project, &layout).expect("kerb spans");
        assert!(!spans.is_empty(), "default route should have kerb spans");
        let mut trimmed = spans.clone();
        trimmed[0].end = trimmed[0].start + 1.0;
        let kerb_vertices = |project: &TrackProject| {
            let track = track_from_project(project).expect("kerb track");
            track
                .visual
                .meshes
                .iter()
                .filter(|mesh| matches!(mesh.name.as_str(), "kerb_red" | "kerb_white"))
                .map(|mesh| mesh.index_count as usize)
                .sum::<usize>()
        };
        let before = kerb_vertices(&project);
        project
            .set_layout_kerb_spans(&layout.id, &trimmed)
            .expect("set kerb spans");
        let after = kerb_vertices(&project);
        assert!(
            after < before,
            "trimmed kerbs should have fewer vertices: {after} vs {before}"
        );
    }

    #[test]
    fn switching_a_blocks_span_off_reduces_barriers_and_keeps_tyres() {
        let mut project = TrackProject::default_project();
        let layout = project.layouts[0].clone();
        let initial = layout_prop_lines(&project, &layout).expect("default prop lines");
        project
            .set_layout_prop_lines(&layout.id, &initial)
            .expect("author default prop lines");
        let before = layout_prop_lines(&project, &layout).expect("prop lines");
        let blocks_index = before
            .iter()
            .position(|line| line.kind == PropKind::Blocks)
            .expect("a blocks line");
        let across_section_vertices = |project: &TrackProject| {
            let track = track_from_project(project).expect("prop track");
            track
                .visual
                .meshes
                .iter()
                .filter(|mesh| mesh.name == "barriers" || mesh.name.starts_with("tires_"))
                .map(|mesh| mesh.index_count as usize)
                .sum::<usize>()
        };
        let before_vertices = across_section_vertices(&project);
        let mut changed = before.clone();
        changed[blocks_index].kind = PropKind::Off;
        project
            .set_layout_prop_lines(&layout.id, &changed)
            .expect("set prop lines");
        let after_vertices = across_section_vertices(&project);
        assert!(
            after_vertices < before_vertices,
            "switching a blocks span off should remove barriers: {after_vertices} vs {before_vertices}"
        );
        let after = layout_prop_lines(&project, &layout).expect("prop lines");
        assert!(
            after.iter().any(|line| line.kind == PropKind::Tyres),
            "other prop kinds should be preserved"
        );
        assert!(
            after.iter().any(|line| line.kind == PropKind::Off),
            "the edited span should remain off"
        );
    }

    #[test]
    fn boundary_crossing_prop_line_spans_multiple_route_edges() {
        let mut project = TrackProject::default_project();
        let layout = project.layouts[0].clone();
        let count = layout_route_points(&project, &layout).len() as f32;
        let (ranges, route_len) =
            layout_route_ranges(&project, &layout).expect("layout route ranges");
        assert!(
            ranges.len() >= 2,
            "default route should have multiple edges"
        );
        let last = route_len.saturating_sub(1).max(1) as f32;
        let boundary = ranges[0].2 as f32;
        let route_start = (boundary - 1.0).max(0.0);
        let route_end = boundary + 1.0;
        let width = global_asphalt_width(&project);
        let line = PropLine {
            start: route_start / last * count,
            end: route_end / last * count,
            side: -1,
            offset: default_prop_offset(width),
            kind: PropKind::Tyres,
        };
        project
            .set_layout_prop_lines(&layout.id, &[line])
            .expect("set crossing prop line");
        let first = project.edge_by_id(&ranges[0].0).expect("first route edge");
        let second = project.edge_by_id(&ranges[1].0).expect("second route edge");
        assert!(
            !first.prop_spans.is_empty(),
            "crossing line should leave a span on the first edge"
        );
        assert!(
            !second.prop_spans.is_empty(),
            "crossing line should continue onto the second edge"
        );
    }

    #[test]
    fn prop_span_with_unknown_spline_is_rejected() {
        let mut project = TrackProject::default_project();
        project.edges[0].prop_spans.push(PropSpan {
            spline: "missing-prop".to_string(),
            start: 0.1,
            end: 0.2,
            kind: PropKind::Blocks,
        });
        assert!(project.normalize().is_err());
    }

    #[test]
    fn two_layout_project_exports_one_schema_4_pack() {
        let mut project = TrackProject::default_project();
        let second = project.add_layout("Second Circuit").expect("add layout");
        assert_ne!(second, "procedural-drift-loop");
        let track = track_from_project(&project).expect("track");
        assert_eq!(track.metadata.layouts.len(), 2);
        let bytes = pack_from_project(&project).expect("pack");
        let pack = sim_asset::Pack::from_bytes(&bytes).expect("parse pack");
        assert_eq!(pack.manifest().schema_version, SCHEMA_VERSION);
        assert_eq!(pack.manifest().id, "procedural-drift-loop");
        let metadata = pack.track_metadata_owned().expect("metadata");
        assert_eq!(metadata.layouts.len(), 2);
        assert!((metadata.default_layout as usize) < metadata.layouts.len());
        let default = metadata
            .layout(metadata.default_layout)
            .expect("default layout");
        assert_eq!(metadata.name, default.name);
        assert_eq!(metadata.length, default.length);
    }

    #[test]
    fn default_project_keeps_the_120_sample_centerline_and_spawn_direction() {
        let project = TrackProject::default_project();
        let layout = &project.layouts[0];
        let points = layout_centerline(&project, layout).expect("centerline");
        assert_eq!(points.len(), SEGMENTS);
        let track = track_from_project(&project).expect("track");
        let spawn = &track.metadata.spawns[0];
        let rotation = Quat::from_xyzw(
            spawn.rotation_xyzw[0],
            spawn.rotation_xyzw[1],
            spawn.rotation_xyzw[2],
            spawn.rotation_xyzw[3],
        );
        let drive_rotation = Quat::from_rotation_y(PI) * rotation;
        let forward = drive_rotation * Vec3::NEG_Z;
        let tangent =
            (Vec3::from_array(points[1]) - Vec3::from_array(points[0])).normalize_or_zero();
        assert!(forward.dot(tangent) > 0.99);
    }

    #[test]
    fn layout_width_changes_the_road_extent() {
        let mut narrow_project = TrackProject::default_project();
        narrow_project
            .set_layout_width("procedural-drift-loop", MIN_LAYOUT_WIDTH)
            .expect("narrow width");
        let mut wide_project = TrackProject::default_project();
        wide_project
            .set_layout_width("procedural-drift-loop", MAX_LAYOUT_WIDTH)
            .expect("wide width");
        let narrow = track_from_project(&narrow_project).expect("narrow track");
        let wide = track_from_project(&wide_project).expect("wide track");
        let narrow_extent = road_mesh_extent(&narrow);
        let wide_extent = road_mesh_extent(&wide);
        assert!(
            wide_extent > narrow_extent * 3.0,
            "wide extent {wide_extent} vs narrow extent {narrow_extent}"
        );
    }

    #[test]
    fn duplicate_and_delete_layout_rules() {
        let mut project = TrackProject::default_project();
        let second = project.add_layout("Second Circuit").expect("add");
        let duplicate = project.duplicate_layout(&second).expect("duplicate");
        assert_eq!(project.layouts.len(), 3);
        assert_ne!(duplicate, second);
        assert!(
            project
                .layouts
                .iter()
                .filter(|layout| layout.id == duplicate)
                .count()
                == 1
        );
        project.delete_layout(&duplicate).expect("delete duplicate");
        project.delete_layout(&second).expect("delete second");
        assert_eq!(project.layouts.len(), 1);
        assert!(project.delete_layout("procedural-drift-loop").is_err());
        assert_eq!(project.layouts.len(), 1);
    }

    #[test]
    fn duplicate_layout_shares_route_and_edges() {
        let mut project = TrackProject::default_project();
        project.layouts[0].sectors = vec![SectorSpec {
            name: "one".to_string(),
            start_fraction: 0.0,
            end_fraction: 0.5,
        }];
        project.layouts[0].cameras = vec![CameraSpec {
            name: "cam".to_string(),
            position: [1.0, 2.0, 3.0],
            target: [4.0, 5.0, 6.0],
            fov_y: 1.2,
        }];
        project.layouts[0].spawns = vec![SpawnPoint {
            name: "grid".to_string(),
            position: [0.0, 0.0, 0.0],
            rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
        }];

        let source = project.layouts[0].clone();
        let source_id = source.id.clone();
        let edge_count = project.edges.len();

        let duplicate_id = project.duplicate_layout(&source_id).expect("duplicate");
        assert_eq!(project.layouts.len(), 2);
        assert_eq!(project.edges.len(), edge_count);
        assert_ne!(duplicate_id, source_id);

        let duplicate = project
            .layout_by_id(&duplicate_id)
            .expect("duplicate exists")
            .clone();
        assert_eq!(duplicate.name, format!("{} Copy", source.name));
        assert_eq!(duplicate.route, source.route);
        assert_eq!(duplicate.start_finish, source.start_finish);
        assert_eq!(duplicate.sectors, source.sectors);
        assert_eq!(duplicate.cameras, source.cameras);
        assert_eq!(duplicate.spawns, source.spawns);

        assert!(project.normalize().is_ok());
    }

    #[test]
    fn project_json_uses_schema_tag_and_camel_case_keys() {
        let project = TrackProject::default_project();
        let value = serde_json::to_value(&project).expect("json value");
        assert_eq!(value["schema"], TRACK_PROJECT_SCHEMA_V4);
        assert_eq!(value["layouts"][0]["id"], "procedural-drift-loop");
        assert!(value["controlPoints"].is_array());
        assert!(value["edges"].is_array());
        assert!(value["layouts"][0]["route"].is_array());
        assert!(value["layouts"][0]["startFinish"]["edge"].is_string());
        assert!(value["startFinish"]["edge"].is_string());
        assert!(value["settings"]["splines"].is_array());
        assert!(value["paths"].is_array());
        assert!(value.get("segments").is_none());
        assert!(value.get("defaultLayout").is_none());
        assert!(value.get("default_layout").is_none());
    }

    #[test]
    fn legacy_v1_project_migrates_to_v4_venue_graph() {
        let legacy = serde_json::json!({
            "schema": "lapster/track-project@1",
            "name": "Legacy Venue",
            "defaultLayout": "old-a",
            "layouts": [
                {
                    "id": "old-a",
                    "name": "Old A",
                    "width": 12.0,
                    "controlPoints": [[-30.0, -20.0], [0.0, -35.0], [30.0, -25.0], [0.0, 0.0]],
                    "startFinish": 0.0,
                    "sectors": [{"name": "one", "start_fraction": 0.0, "end_fraction": 1.0}]
                },
                {
                    "id": "old-b",
                    "name": "Old B",
                    "width": 8.0,
                    "controlPoints": [[-20.0, 60.0], [20.0, 60.0], [20.0, 100.0], [-20.0, 100.0]],
                    "startFinish": 0.0
                }
            ]
        });
        let project = TrackProject::from_json(&legacy.to_string()).expect("migrate");
        assert_eq!(project.schema, TRACK_PROJECT_SCHEMA_V4);
        assert_eq!(project.layouts.len(), 2);
        assert!(project.edges.len() > 8);
        for layout in &project.layouts {
            assert!(!layout.route.is_empty());
            assert!(!layout.start_finish.edge.is_empty());
            assert!(
                layout
                    .route
                    .iter()
                    .any(|edge| edge == &layout.start_finish.edge),
                "layout '{}' start/finish edge '{}' must be in its route",
                layout.id,
                layout.start_finish.edge
            );
            assert!(layout.start_finish.fraction.is_finite());
            assert!((0.0..1.0).contains(&layout.start_finish.fraction));
        }
        let mut ids = BTreeSet::new();
        for edge in &project.edges {
            assert!(
                ids.insert(edge.id.as_str()),
                "duplicate edge id {}",
                edge.id
            );
        }
        let width = |layout_index: usize| {
            let first = project
                .control_points
                .iter()
                .find(|point| point.id == format!("layout-{layout_index}-control-0"))
                .expect("migrated control point");
            let profile = first.profile.as_ref().expect("migrated profile");
            profile.values["asphalt-right"] - profile.values["asphalt-left"]
        };
        assert!((width(0) - 12.0).abs() < 1e-4);
        assert!((width(1) - 8.0).abs() < 1e-4);
        assert_eq!(project.layouts[0].sectors.len(), 1);
        assert!(project.normalize().is_ok());
    }

    #[test]
    fn route_closure_rejects_open_routes() {
        let mut project = TrackProject::default_project();
        project.layouts[0].route.pop();
        assert!(project.normalize().is_err());
    }

    #[test]
    fn shared_edge_direction_must_be_canonical() {
        let mut project = TrackProject::default_project();
        project.layouts[0].route.reverse();
        assert!(project.normalize().is_err());
    }

    #[test]
    fn start_finish_edge_must_appear_in_every_route() {
        let mut project = TrackProject::default_project();
        let start = project
            .edge_by_id(&project.start_finish.edge)
            .unwrap()
            .clone();
        let detour = EdgeProject {
            id: "detour".to_string(),
            from: start.from.clone(),
            to: start.to.clone(),
            start_profile: None,
            end_profile: None,
            kerbs: Vec::new(),
            prop_spans: Vec::new(),
            points: Vec::new(),
            from_tangent: None,
            to_tangent: None,
        };
        project.edges.push(detour.clone());
        project.layouts[0].route[0] = detour.id.clone();
        assert!(project.normalize().is_err());
    }

    #[test]
    fn profile_offsets_must_increase_in_declared_order() {
        let mut project = TrackProject::default_project();
        if let Some(spline) = project
            .settings
            .splines
            .iter_mut()
            .find(|spline| spline.id == "asphalt-right")
        {
            spline.offset = -100.0;
        }
        assert!(project.normalize().is_err());
    }

    #[test]
    fn fork_control_points_need_three_or_more_edges() {
        let mut project = TrackProject::default_project();
        project.control_points[0].kind = ControlPointKind::Fork;
        assert!(project.normalize().is_err());
    }

    #[test]
    fn pit_lane_route_must_be_an_entry_to_exit_chain() {
        let mut project = TrackProject::default_project();
        let edge = project.edges[0].id.clone();
        project.pit_lane = Some(PitLaneProject {
            id: "pit".to_string(),
            route: vec![edge],
            boxes: Vec::new(),
        });
        assert!(project.normalize().is_err());
    }

    fn asphalt_vertex_count(track: &EditorTrack) -> usize {
        let index = track
            .visual
            .meshes
            .iter()
            .position(|mesh| mesh.name == "asphalt_dither")
            .expect("asphalt mesh");
        track.visual.mesh_buffers[index].positions.len()
    }

    fn asphalt_band_extent(track: &EditorTrack) -> f32 {
        let index = track
            .visual
            .meshes
            .iter()
            .position(|mesh| mesh.name == "asphalt_dither")
            .expect("asphalt mesh");
        let buffers = &track.visual.mesh_buffers[index];
        let rows = buffers.positions.len() / 4;
        (0..rows)
            .map(|row| {
                Vec3::from_array(buffers.positions[row * 4 + 1])
                    .distance(Vec3::from_array(buffers.positions[row * 4 + 2]))
            })
            .fold(0.0, f32::max)
    }

    fn unique_route_edges(project: &TrackProject) -> BTreeSet<String> {
        project
            .layouts
            .iter()
            .flat_map(|layout| layout.route.iter().cloned())
            .collect()
    }

    fn fork_project() -> TrackProject {
        let point = |id: &str, position: [f32; 2], kind: ControlPointKind| ControlPoint {
            id: id.to_string(),
            position,
            kind,
            profile: None,
            branch_tangents: BTreeMap::new(),
        };
        let edge = |id: &str, from: &str, to: &str| EdgeProject {
            id: id.to_string(),
            from: from.to_string(),
            to: to.to_string(),
            start_profile: None,
            end_profile: None,
            kerbs: Vec::new(),
            prop_spans: Vec::new(),
            points: Vec::new(),
            from_tangent: None,
            to_tangent: None,
        };
        TrackProject {
            schema: TRACK_PROJECT_SCHEMA.to_string(),
            name: "Fork Venue".to_string(),
            settings: VenueSettings::default(),
            start_finish: StartFinish {
                edge: "e0".to_string(),
                fraction: 0.0,
            },
            pit_lane: None,
            control_points: vec![
                point("c0", [0.0, 0.0], ControlPointKind::Fork),
                point("c1", [40.0, 0.0], ControlPointKind::Normal),
                point("c2", [40.0, 40.0], ControlPointKind::Fork),
                point("c3", [0.0, 40.0], ControlPointKind::Normal),
            ],
            edges: vec![
                edge("e0", "c0", "c1"),
                edge("e1", "c1", "c2"),
                edge("e2", "c2", "c3"),
                edge("e3", "c3", "c0"),
                edge("e4", "c2", "c0"),
            ],
            segments: Vec::new(),
            layouts: vec![
                LayoutProject {
                    id: "full".to_string(),
                    name: "Full".to_string(),
                    route: vec![
                        "e0".to_string(),
                        "e1".to_string(),
                        "e2".to_string(),
                        "e3".to_string(),
                    ],
                    sectors: Vec::new(),
                    cameras: Vec::new(),
                    spawns: Vec::new(),
                    start_finish: StartFinish::default(),
                },
                LayoutProject {
                    id: "short".to_string(),
                    name: "Short".to_string(),
                    route: vec!["e0".to_string(), "e1".to_string(), "e4".to_string()],
                    sectors: Vec::new(),
                    cameras: Vec::new(),
                    spawns: Vec::new(),
                    start_finish: StartFinish::default(),
                },
            ],
            paths: Vec::new(),
        }
    }

    #[test]
    fn shared_route_layouts_do_not_duplicate_geometry() {
        let single = track_from_project(&TrackProject::default_project()).expect("single track");
        let mut project = TrackProject::default_project();
        let mut second = project.layouts[0].clone();
        second.id = "shared-route-copy".to_string();
        second.name = "Shared Route Copy".to_string();
        project.layouts.push(second);

        let shared = track_from_project(&project).expect("shared track");
        assert_eq!(shared.metadata.layouts.len(), 2);
        assert_eq!(asphalt_vertex_count(&shared), asphalt_vertex_count(&single));
        assert_eq!(
            shared.collision.vertices.len(),
            single.collision.vertices.len()
        );
    }

    #[test]
    fn disjoint_layout_routes_sum_unique_edges() {
        let mut project = TrackProject::default_project();
        let second = project.add_layout("Second Circuit").expect("add layout");
        let mut first_only = project.clone();
        first_only.layouts.retain(|layout| layout.id != second);
        let mut second_only = project.clone();
        second_only
            .layouts
            .retain(|layout| layout.id != "procedural-drift-loop");

        let combined = track_from_project(&project).expect("combined track");
        let first_track = track_from_project(&first_only).expect("first track");
        let second_track = track_from_project(&second_only).expect("second track");

        let ids = unique_route_edges(&project);
        let splines = surface_splines(&project);
        let built = build_unique_edges(&project, &ids).expect("built edges");
        let edge_vertices = ids
            .iter()
            .map(|id| {
                edge_ribbon(built.get(id).expect("built edge"), &splines)
                    .0
                    .positions
                    .len()
            })
            .sum::<usize>();
        let pads = build_junction_pads(&project, &built, &splines);
        assert_eq!(
            asphalt_vertex_count(&combined),
            edge_vertices + pads.positions.len()
        );
        assert!(
            asphalt_vertex_count(&combined)
                < asphalt_vertex_count(&first_track) + asphalt_vertex_count(&second_track)
        );
    }

    #[test]
    fn layout_lengths_and_shared_start_finish_spawns_are_correct() {
        let mut project = TrackProject::default_project();
        project.add_layout("Second Circuit").expect("add layout");
        project.start_finish.fraction = 0.0;
        let track = track_from_project(&project).expect("track");

        for layout in &project.layouts {
            let spec = track
                .metadata
                .layout_by_id(&layout.id)
                .expect("layout metadata");
            let ids = layout.route.iter().cloned().collect::<BTreeSet<_>>();
            let built = build_unique_edges(&project, &ids).expect("built edges");
            let expected = layout
                .route
                .iter()
                .map(|id| built.get(id).expect("route edge").length)
                .sum::<f32>();
            assert!((spec.length - expected).abs() < 1e-3);
            assert!((spec.start_finish - project.start_finish.fraction).abs() < 1e-6);
            let start_edge = built
                .get(&project.start_finish.edge)
                .expect("shared start/finish edge");
            assert_eq!(
                spec.spawns.first().expect("spawn").position,
                start_edge.points[0].to_array()
            );
        }
    }

    #[test]
    fn two_routes_bake_their_own_start_finish() {
        let mut project = TrackProject::default_project();
        let second_id = project.add_layout("Second Circuit").expect("add layout");
        project.layouts[0].start_finish = StartFinish {
            edge: project.layouts[0].route[0].clone(),
            fraction: 0.25,
        };
        let second_index = project
            .layout_index(&second_id)
            .expect("second layout index");
        project.layouts[second_index].start_finish = StartFinish {
            edge: project.layouts[second_index].route[1].clone(),
            fraction: 0.75,
        };
        let project = project.normalize().expect("normalized project");
        let track = track_from_project(&project).expect("track");
        let ids = unique_route_edges(&project);
        let built = build_unique_edges(&project, &ids).expect("built edges");

        let expected_fraction = |layout: &LayoutProject| -> f32 {
            let mut total = 0.0_f32;
            let mut preceding = 0.0_f32;
            let mut start_finish_distance = None;
            for edge_id in &layout.route {
                let edge = built.get(edge_id).expect("route edge");
                if edge_id == &layout.start_finish.edge {
                    start_finish_distance =
                        Some(preceding + edge.length * layout.start_finish.fraction);
                }
                preceding += edge.length;
                total += edge.length;
            }
            start_finish_distance.unwrap_or(0.0) / total
        };

        let mut fractions = Vec::new();
        let mut spawns = Vec::new();
        for layout in &project.layouts {
            let spec = track
                .metadata
                .layout_by_id(&layout.id)
                .expect("layout metadata");
            let expected = expected_fraction(layout);
            assert!(
                (spec.start_finish - expected).abs() < 1e-4,
                "layout '{}' start/finish {} != expected {}",
                layout.id,
                spec.start_finish,
                expected
            );
            fractions.push(spec.start_finish);

            let route =
                assemble_route(&project, layout, &built, &HashMap::new()).expect("assembled route");
            let count = route.points.len();
            let index =
                ((route.start_finish.clamp(0.0, 1.0) * count as f32).round() as usize) % count;
            let spawn = spec.spawns.first().expect("spawn");
            assert!(spawn.position.iter().all(|value| value.is_finite()));
            assert!(
                Vec3::from_array(spawn.position).distance(route.points[index]) < 1e-3,
                "layout '{}' spawn is not at its start/finish sample",
                layout.id
            );
            spawns.push(spawn.position);
        }

        assert!(
            (fractions[0] - fractions[1]).abs() > 1e-4,
            "per-layout start/finish fractions should differ: {:?}",
            fractions
        );
        assert_ne!(spawns[0], spawns[1], "per-layout spawns should differ");
    }

    #[test]
    fn fork_topology_samples_both_branches() {
        let project = fork_project();
        let track = track_from_project(&project).expect("fork track");
        assert_eq!(track.metadata.layouts.len(), 2);
        assert!(asphalt_vertex_count(&track) > 0);
        let built =
            build_unique_edges(&project, &unique_route_edges(&project)).expect("built edges");
        for branch in ["e2", "e3", "e4"] {
            let edge = built.get(branch).expect("branch edge");
            assert!(edge.points.len() >= 4);
        }
    }

    #[test]
    fn fork_nodes_build_junction_pads_and_collision() {
        use sim_core::TrackCollider;

        let project = fork_project();
        let built =
            build_unique_edges(&project, &unique_route_edges(&project)).expect("built edges");
        let splines = surface_splines(&project);
        let pads = build_junction_pads(&project, &built, &splines);
        assert!(!pads.positions.is_empty());
        assert!(pads.indices.len() / 3 >= 6);

        let track = track_from_project(&project).expect("fork track");
        let edge_vertices = unique_route_edges(&project)
            .iter()
            .map(|id| {
                edge_ribbon(built.get(id).expect("built edge"), &splines)
                    .0
                    .positions
                    .len()
            })
            .sum::<usize>();
        assert_eq!(
            asphalt_vertex_count(&track),
            edge_vertices + pads.positions.len()
        );

        let collider = TrackCollider::from_collision(&track.collision, &track.metadata);
        for node in [[0.0, 0.0], [40.0, 40.0]] {
            let y = terrain_height(node[0], node[1]) + 20.0;
            let hit = collider
                .raycast(Vec3::new(node[0], y, node[1]), Vec3::NEG_Y, 50.0)
                .unwrap_or_else(|| panic!("no junction collision at {:?}", node));
            assert_eq!(hit.surface, TRACK_SURFACE);
        }
    }

    #[test]
    fn shared_junction_collision_continues_across_routes() {
        use sim_core::TrackCollider;

        let project = fork_project();
        let built =
            build_unique_edges(&project, &unique_route_edges(&project)).expect("built edges");
        let splines = surface_splines(&project);
        let pads = build_junction_pads(&project, &built, &splines);
        let track = track_from_project(&project).expect("fork track");
        let collider = TrackCollider::from_collision(&track.collision, &track.metadata);

        // The shared e0/e1 approach is baked once for both routes; only the
        // junction pads add asphalt on top of the unique edge ribbons.
        let edge_vertices = unique_route_edges(&project)
            .iter()
            .map(|id| {
                edge_ribbon(built.get(id).expect("built edge"), &splines)
                    .0
                    .positions
                    .len()
            })
            .sum::<usize>();
        assert_eq!(
            asphalt_vertex_count(&track),
            edge_vertices + pads.positions.len()
        );

        let hit_at = |point: Vec3, label: &str| {
            let hit = collider
                .raycast(
                    Vec3::new(point.x, point.y + 20.0, point.z),
                    Vec3::NEG_Y,
                    50.0,
                )
                .unwrap_or_else(|| panic!("gap at {label}"));
            assert_eq!(hit.surface, TRACK_SURFACE, "wrong surface at {label}");
            assert!(
                (hit.point.y - point.y).abs() < 0.5,
                "surface height jumps at {label}"
            );
        };

        // Walk both routes over the shared approach and across the forks at
        // c2 (e1 -> e2 or e4) and c0 (e2/e4 -> e0). Every sampled centreline
        // point, including the junction endpoints themselves, must land on
        // exactly one continuous asphalt surface.
        let routes: [&[&str]; 2] = [&["e0", "e1", "e2", "e3"], &["e0", "e1", "e4"]];
        for route in routes {
            for edge_id in route {
                let edge = built.get(*edge_id).expect("route edge");
                let last = edge.points.len() - 1;
                let stations = [
                    0.0,
                    1.0,
                    last as f32 * 0.5,
                    (last - 2) as f32,
                    (last - 1) as f32,
                    last as f32,
                ];
                for station in stations {
                    let (point, _) = edge_frame_at(&edge.points, station);
                    hit_at(point, &format!("edge {edge_id} station {station}"));
                }
            }
        }

        // Explicitly cross the shared junction seam from e1 onto each branch.
        let approach = built.get("e1").expect("shared approach");
        let last = approach.points.len() - 1;
        for branch_id in ["e2", "e4"] {
            let branch = built.get(branch_id).expect("branch edge");
            for point in [approach.points[last - 1], approach.points[last]]
                .into_iter()
                .chain(branch.points[..=2].iter().copied())
            {
                hit_at(point, &format!("e1 -> {branch_id} seam"));
            }
        }
    }

    #[test]
    fn degree_two_loop_builds_join_pads() {
        use sim_core::TrackCollider;

        let project = TrackProject::default_project();
        let built =
            build_unique_edges(&project, &unique_route_edges(&project)).expect("built edges");
        let splines = surface_splines(&project);
        let pads = build_junction_pads(&project, &built, &splines);
        assert!(!pads.positions.is_empty());
        assert!(!pads.indices.is_empty());
        assert_eq!(pads.indices.len() % 3, 0);

        let track = track_from_project(&project).expect("track");
        let edge_vertices = unique_route_edges(&project)
            .iter()
            .map(|id| {
                edge_ribbon(built.get(id).expect("built edge"), &splines)
                    .0
                    .positions
                    .len()
            })
            .sum::<usize>();
        assert_eq!(
            asphalt_vertex_count(&track),
            edge_vertices + pads.positions.len()
        );

        // Join pads also contribute track collision at their control points.
        assert!(track.collision.vertices.len() >= pads.positions.len());
        let collider = TrackCollider::from_collision(&track.collision, &track.metadata);
        let pad_center = pads.positions[0];
        let hit = collider
            .raycast(
                Vec3::new(pad_center[0], pad_center[1] + 20.0, pad_center[2]),
                Vec3::NEG_Y,
                50.0,
            )
            .expect("no join pad collision");
        assert_eq!(hit.surface, TRACK_SURFACE);
    }

    #[test]
    fn branch_tangents_change_fork_endpoint_directions() {
        let fallback = fork_project();
        let fallback_tangent = {
            let built =
                build_unique_edges(&fallback, &unique_route_edges(&fallback)).expect("built edges");
            let edge = built.get("e4").expect("branch edge");
            (edge.points[1] - edge.points[0]).normalize_or_zero()
        };

        let mut project = fork_project();
        let branch = project
            .edges
            .iter_mut()
            .find(|edge| edge.id == "e4")
            .expect("branch edge");
        branch.from_tangent = Some([1.0, -1.0]);
        let built =
            build_unique_edges(&project, &unique_route_edges(&project)).expect("built edges");
        let edge = built.get("e4").expect("branch edge");
        let expected = Vec3::new(1.0, 0.0, -1.0).normalize();

        let start_tangent = (edge.points[1] - edge.points[0]).normalize_or_zero();
        let start_xz = Vec3::new(start_tangent.x, 0.0, start_tangent.z).normalize();
        assert!(
            start_xz.dot(expected) > 0.99,
            "start {start_xz:?} expected {expected:?}"
        );
        assert!(start_xz.dot(fallback_tangent) < 0.99);

        // At a `to` node the tangent is the canonical incoming direction.
        let mut merge = fork_project();
        let merge_edge = merge
            .edges
            .iter_mut()
            .find(|edge| edge.id == "e3")
            .expect("merge edge");
        merge_edge.to_tangent = Some([0.0, -1.0]);
        let built = build_unique_edges(&merge, &unique_route_edges(&merge)).expect("built edges");
        let edge = built.get("e3").expect("merge edge");
        let end = edge.points.len() - 1;
        let endpoint_tangent = (edge.points[end] - edge.points[end - 1]).normalize_or_zero();
        let endpoint_xz = Vec3::new(endpoint_tangent.x, 0.0, endpoint_tangent.z).normalize();
        assert!(endpoint_xz.dot(Vec3::new(0.0, 0.0, -1.0)) > 0.99);
    }

    #[test]
    fn branch_tangents_validate_and_normalize() {
        let mut project = fork_project();
        let point = project
            .control_points
            .iter_mut()
            .find(|point| point.id == "c2")
            .expect("fork point");
        point.branch_tangents.insert("e4".to_string(), [3.0, 4.0]);
        let normalized = project.clone().normalize().expect("normalized project");
        let tangent = normalized
            .control_points
            .iter()
            .find(|point| point.id == "c2")
            .and_then(|point| point.branch_tangents.get("e4"))
            .copied()
            .expect("normalized tangent");
        assert!((tangent[0] - 0.6).abs() < 1e-6);
        assert!((tangent[1] - 0.8).abs() < 1e-6);

        let mut non_incident = fork_project();
        non_incident.control_points[2]
            .branch_tangents
            .insert("missing".to_string(), [1.0, 0.0]);
        assert!(non_incident.normalize().is_err());

        let mut zero = fork_project();
        zero.control_points[2]
            .branch_tangents
            .insert("e4".to_string(), [0.0, 0.0]);
        assert!(zero.normalize().is_err());

        let mut non_finite = fork_project();
        non_finite.control_points[2]
            .branch_tangents
            .insert("e4".to_string(), [f32::NAN, 1.0]);
        assert!(non_finite.normalize().is_err());
    }

    #[test]
    fn profile_width_changes_the_asphalt_band_extent() {
        let mut narrow = TrackProject::default_project();
        narrow
            .set_layout_width("procedural-drift-loop", MIN_LAYOUT_WIDTH)
            .expect("narrow width");
        let mut wide = TrackProject::default_project();
        wide.set_layout_width("procedural-drift-loop", MAX_LAYOUT_WIDTH)
            .expect("wide width");
        let narrow_track = track_from_project(&narrow).expect("narrow track");
        let wide_track = track_from_project(&wide).expect("wide track");
        let narrow_extent = asphalt_band_extent(&narrow_track);
        let wide_extent = asphalt_band_extent(&wide_track);
        assert!((narrow_extent - MIN_LAYOUT_WIDTH).abs() < 0.5);
        assert!((wide_extent - MAX_LAYOUT_WIDTH).abs() < 0.5);
        assert!(wide_extent > narrow_extent * 3.0);
    }

    #[test]
    fn default_layout_has_exactly_one_racing_line_mesh() {
        let mut project = TrackProject::default_project();
        project.add_layout("Second Circuit").expect("add layout");
        let track = track_from_project(&project).expect("track");
        let count = track
            .visual
            .meshes
            .iter()
            .filter(|mesh| mesh.name == "racing_line")
            .count();
        assert_eq!(count, 1);
    }

    fn shared_pair_project() -> EditorProject {
        let mut project = EditorProject::default_project()
            .normalize()
            .expect("default editor project");
        for (id, position) in [
            ("cp-a", [50.0, 50.0]),
            ("cp-b", [60.0, 50.0]),
            ("cp-c", [50.0, 60.0]),
        ] {
            project.control_points.push(EditorControlPoint {
                id: id.to_string(),
                position,
                kind: ControlPointKind::Normal,
                tangent: None,
            });
        }
        project.loops.push(EditorLoop {
            id: "short-loop".to_string(),
            name: "Short Loop".to_string(),
            control_points: vec![
                "cp-0".to_string(),
                "cp-1".to_string(),
                "cp-a".to_string(),
                "cp-b".to_string(),
                "cp-c".to_string(),
            ],
            sectors: Vec::new(),
            cameras: Vec::new(),
            spawns: Vec::new(),
        });
        project.normalize().expect("shared loop project")
    }

    fn layout_graph_positions(graph: &TrackProject, layout_id: &str) -> Vec<[f32; 2]> {
        let layout = graph.layout_by_id(layout_id).expect("layout");
        layout
            .route
            .iter()
            .map(|edge_id| {
                let edge = graph.edge_by_id(edge_id).expect("edge");
                let point = graph
                    .control_points
                    .iter()
                    .find(|point| point.id == edge.from)
                    .expect("control point");
                point.position
            })
            .collect()
    }

    #[test]
    fn editor_project_round_trips_through_json_with_schema_tag() {
        let project = EditorProject::default_project()
            .normalize()
            .expect("normalized default project");
        let json = serde_json::to_string(&project).expect("serialize editor project");
        let parsed = EditorProject::from_json(&json).expect("parse editor project");
        assert_eq!(parsed, project);
        assert_eq!(parsed.schema, EDITOR_PROJECT_SCHEMA);
        assert_eq!(parsed.loops.len(), 1);
        assert_eq!(parsed.loops[0].id, "procedural-drift-loop");
        assert_eq!(parsed.segments.len(), 8);
        let value = serde_json::to_value(&parsed).expect("editor project value");
        assert_eq!(value["schema"], EDITOR_PROJECT_SCHEMA);
        assert!(value["loops"].is_array());
        assert!(value["controlPoints"].is_array());
        assert_eq!(value["settings"]["width"], 12.0);
        assert!(value.get("layouts").is_none());
        assert!(value.get("edges").is_none());
    }

    #[test]
    fn shared_loop_pair_converts_to_one_internal_edge() {
        let project = shared_pair_project();
        let graph = project.to_graph().expect("graph");
        let shared_edges = graph
            .edges
            .iter()
            .filter(|edge| edge.from == "cp-0" && edge.to == "cp-1")
            .count();
        assert_eq!(shared_edges, 1);
        assert_eq!(graph.edges.len(), 12);
        let route_edges = graph
            .layouts
            .iter()
            .map(|layout| layout.route.len())
            .sum::<usize>();
        assert_eq!(route_edges, 13);
    }

    #[test]
    fn moving_a_shared_point_changes_both_loop_graphs() {
        let mut project = shared_pair_project();
        let before = project.to_graph().expect("before graph");
        let before_first = layout_graph_positions(&before, "procedural-drift-loop");
        let before_second = layout_graph_positions(&before, "short-loop");
        project
            .control_points
            .iter_mut()
            .find(|point| point.id == "cp-0")
            .expect("shared point")
            .position = [5.0, 7.0];
        let after = project.to_graph().expect("after graph");
        let after_first = layout_graph_positions(&after, "procedural-drift-loop");
        let after_second = layout_graph_positions(&after, "short-loop");
        assert_ne!(before_first[0], after_first[0]);
        assert_ne!(before_second[0], after_second[0]);
        assert_eq!(after_first[0], [5.0, 7.0]);
        assert_eq!(after_second[0], [5.0, 7.0]);
    }

    #[test]
    fn duplicate_loop_shares_all_control_point_ids() {
        let mut project = EditorProject::default_project()
            .normalize()
            .expect("default project");
        let original = project.loops[0].control_points.clone();
        let duplicate = project
            .duplicate_loop("procedural-drift-loop")
            .expect("duplicate loop");
        assert_eq!(project.loops.len(), 2);
        assert_eq!(project.control_points.len(), original.len());
        assert_eq!(
            project
                .loop_by_id(&duplicate)
                .expect("duplicated loop")
                .control_points,
            original
        );
    }

    #[test]
    fn v2_project_migrates_to_v3_with_decorations() {
        let mut graph = TrackProject::default_project();
        let second = graph.add_layout("Second Circuit").expect("add layout");
        graph.edges[0].kerbs.push(KerbSpan {
            start: 0.1,
            end: 0.2,
            side: 1,
        });
        graph.edges[0].prop_spans.push(PropSpan {
            spline: "prop-near-left".to_string(),
            start: 0.1,
            end: 0.2,
            kind: PropKind::Blocks,
        });
        let mut graph = graph.normalize().expect("normalized graph");
        graph.schema = TRACK_PROJECT_SCHEMA.to_string();
        let source = graph.edges[0].clone();
        let json = serde_json::to_string(&graph).expect("serialize graph");
        let project = EditorProject::from_json(&json).expect("migrate project");
        assert_eq!(project.schema, EDITOR_PROJECT_SCHEMA);
        assert_eq!(project.loops.len(), 2);
        assert!(project
            .loops
            .iter()
            .any(|loop_| loop_.id == second && loop_.name == "Second Circuit"));
        let segment = project
            .segments
            .iter()
            .find(|segment| segment.from == source.from && segment.to == source.to)
            .expect("migrated segment");
        assert_eq!(segment.kerbs.len(), 1);
        assert_eq!(segment.prop_spans.len(), 1);
        assert_eq!(segment.prop_spans[0].spline, "prop-near-left");
        assert!((project.settings.width - DEFAULT_LAYOUT_WIDTH).abs() < 1e-6);
    }

    #[test]
    fn editor_validation_rejects_missing_short_and_duplicate_pairs() {
        let normalized = EditorProject::default_project()
            .normalize()
            .expect("default project");

        let mut missing = normalized.clone();
        missing.loops[0].control_points[0] = "missing".to_string();
        assert!(missing.normalize().is_err());

        let mut short = normalized.clone();
        short.loops[0].control_points.truncate(2);
        assert!(short.normalize().is_err());

        let mut duplicate = normalized.clone();
        duplicate.segments.push(duplicate.segments[0].clone());
        assert!(duplicate.normalize().is_err());
    }

    #[test]
    fn averaged_shared_tangent_is_used_by_both_loops() {
        let project = EditorProject {
            schema: EDITOR_PROJECT_SCHEMA.to_string(),
            name: "Averaged Tangent".to_string(),
            settings: EditorSettings::default(),
            loops: vec![
                EditorLoop {
                    id: "loop-a".to_string(),
                    name: "Loop A".to_string(),
                    control_points: vec!["a".to_string(), "b".to_string(), "d".to_string()],
                    sectors: Vec::new(),
                    cameras: Vec::new(),
                    spawns: Vec::new(),
                },
                EditorLoop {
                    id: "loop-b".to_string(),
                    name: "Loop B".to_string(),
                    control_points: vec!["a".to_string(), "c".to_string(), "d".to_string()],
                    sectors: Vec::new(),
                    cameras: Vec::new(),
                    spawns: Vec::new(),
                },
            ],
            control_points: vec![
                EditorControlPoint {
                    id: "a".to_string(),
                    position: [0.0, 0.0],
                    kind: ControlPointKind::StartFinish,
                    tangent: None,
                },
                EditorControlPoint {
                    id: "b".to_string(),
                    position: [10.0, 0.0],
                    kind: ControlPointKind::Normal,
                    tangent: None,
                },
                EditorControlPoint {
                    id: "c".to_string(),
                    position: [0.0, 10.0],
                    kind: ControlPointKind::Normal,
                    tangent: None,
                },
                EditorControlPoint {
                    id: "d".to_string(),
                    position: [-10.0, -10.0],
                    kind: ControlPointKind::Normal,
                    tangent: None,
                },
            ],
            segments: Vec::new(),
            pit_lane: None,
        }
        .normalize()
        .expect("tangent project");
        let graph = project.to_graph().expect("graph");
        let point = graph
            .control_points
            .iter()
            .find(|point| point.id == "a")
            .expect("shared point");
        let first = point.branch_tangents.get("a->b").expect("loop A tangent");
        let second = point.branch_tangents.get("a->c").expect("loop B tangent");
        let expected = std::f32::consts::FRAC_1_SQRT_2;
        assert!((first[0] - expected).abs() < 1e-5);
        assert!((first[1] - expected).abs() < 1e-5);
        assert_eq!(first, second);
    }

    fn point_info<'a>(info: &'a [EditorPointInfo], id: &str) -> &'a EditorPointInfo {
        info.iter()
            .find(|point| point.id == id)
            .unwrap_or_else(|| panic!("missing point info for '{id}'"))
    }

    #[test]
    fn moving_a_shared_point_changes_both_loop_point_info_and_centerline() {
        let mut project = shared_pair_project();
        let before_first = project
            .loop_centerline(project.loop_by_id("procedural-drift-loop").expect("loop"))
            .expect("first centerline");
        let before_second = project
            .loop_centerline(project.loop_by_id("short-loop").expect("loop"))
            .expect("second centerline");

        project
            .move_control_point("cp-0", -32.0, -22.0)
            .expect("move shared point");

        let graph = project.to_graph().expect("graph");
        assert_eq!(
            layout_graph_positions(&graph, "procedural-drift-loop")[0],
            [-32.0, -22.0]
        );
        assert_eq!(
            layout_graph_positions(&graph, "short-loop")[0],
            [-32.0, -22.0]
        );

        let first_info = project
            .loop_point_info("procedural-drift-loop")
            .expect("first info");
        let second_info = project.loop_point_info("short-loop").expect("second info");
        let shared = point_info(&first_info, "cp-0");
        assert_eq!(shared.position, [-32.0, -22.0]);
        assert!(shared.shared);
        assert_eq!(shared.shared_count, 2);
        assert_eq!(point_info(&second_info, "cp-0").position, [-32.0, -22.0]);

        let after_first = project
            .loop_centerline(project.loop_by_id("procedural-drift-loop").expect("loop"))
            .expect("first centerline");
        let after_second = project
            .loop_centerline(project.loop_by_id("short-loop").expect("loop"))
            .expect("second centerline");
        assert_ne!(before_first, after_first);
        assert_ne!(before_second, after_second);
    }

    #[test]
    fn make_control_point_unique_breaks_sharing_and_diverges_after_move() {
        let mut project = shared_pair_project();
        let original_position = project
            .control_points
            .iter()
            .find(|point| point.id == "cp-0")
            .expect("shared point")
            .position;
        let new_id = project
            .make_control_point_unique("short-loop", "cp-0")
            .expect("unique point");

        assert_ne!(new_id, "cp-0");
        let short_loop = project.loop_by_id("short-loop").expect("short loop");
        assert_eq!(short_loop.control_points[0], new_id);
        let default_loop = project
            .loop_by_id("procedural-drift-loop")
            .expect("default loop");
        assert_eq!(default_loop.control_points[0], "cp-0");
        assert_eq!(
            project
                .control_points
                .iter()
                .find(|point| point.id == new_id)
                .expect("new point")
                .position,
            original_position
        );

        let second_info = project.loop_point_info("short-loop").expect("info");
        assert!(!point_info(&second_info, &new_id).shared);
        assert_eq!(point_info(&second_info, &new_id).shared_count, 1);
        let first_info = project
            .loop_point_info("procedural-drift-loop")
            .expect("info");
        assert!(!point_info(&first_info, "cp-0").shared);
        assert_eq!(point_info(&first_info, "cp-0").shared_count, 1);

        project
            .move_control_point(&new_id, 250.0, 250.0)
            .expect("move unique point");
        let graph = project.to_graph().expect("graph");
        let first_positions = layout_graph_positions(&graph, "procedural-drift-loop");
        let second_positions = layout_graph_positions(&graph, "short-loop");
        assert_eq!(first_positions[0], original_position);
        assert_eq!(second_positions[0], [250.0, 250.0]);
    }

    #[test]
    fn insert_control_point_only_changes_the_selected_loop() {
        let mut project = shared_pair_project();
        let first_before =
            layout_graph_positions(&project.to_graph().expect("graph"), "procedural-drift-loop");
        let new_id = project
            .insert_control_point("short-loop", "cp-0", 12.0, -12.0)
            .expect("insert point");

        let short_loop = project.loop_by_id("short-loop").expect("short loop");
        assert_eq!(short_loop.control_points[0], "cp-0");
        assert_eq!(short_loop.control_points[1], new_id);
        let default_loop = project
            .loop_by_id("procedural-drift-loop")
            .expect("default loop");
        assert!(!default_loop.control_points.contains(&new_id));

        let info = project.loop_point_info("short-loop").expect("info");
        let inserted = point_info(&info, &new_id);
        assert_eq!(inserted.position, [12.0, -12.0]);
        assert!(!inserted.shared);
        assert_eq!(inserted.shared_count, 1);

        let first_after =
            layout_graph_positions(&project.to_graph().expect("graph"), "procedural-drift-loop");
        assert_eq!(first_before, first_after);

        project
            .move_control_point(&new_id, 150.0, 150.0)
            .expect("move inserted point");
        let graph = project.to_graph().expect("graph");
        assert_eq!(
            layout_graph_positions(&graph, "procedural-drift-loop"),
            first_before
        );
        assert_eq!(
            layout_graph_positions(&graph, "short-loop")[1],
            [150.0, 150.0]
        );
    }

    #[test]
    fn delete_control_point_removes_one_reference_and_keeps_shared_point() {
        let mut project = shared_pair_project();
        let original_position = project
            .control_points
            .iter()
            .find(|point| point.id == "cp-0")
            .expect("shared point")
            .position;

        project
            .delete_control_point("short-loop", "cp-0")
            .expect("delete from short loop");

        let short_loop = project.loop_by_id("short-loop").expect("short loop");
        assert!(!short_loop.control_points.contains(&"cp-0".to_string()));
        let default_loop = project
            .loop_by_id("procedural-drift-loop")
            .expect("default loop");
        assert!(default_loop.control_points.contains(&"cp-0".to_string()));
        assert_eq!(
            project
                .control_points
                .iter()
                .find(|point| point.id == "cp-0")
                .expect("shared point retained")
                .position,
            original_position
        );

        let info = project
            .loop_point_info("procedural-drift-loop")
            .expect("info");
        let shared = point_info(&info, "cp-0");
        assert!(!shared.shared);
        assert_eq!(shared.shared_count, 1);
    }

    #[test]
    fn delete_control_point_refuses_below_three_points() {
        let mut project = EditorProject::default_project()
            .normalize()
            .expect("default project");
        project.loops[0].control_points.truncate(3);
        project = project.normalize().expect("three point loop");
        let before = project.loops[0].control_points.clone();

        let result = project.delete_control_point("procedural-drift-loop", &before[0]);

        assert!(result.is_err());
        assert_eq!(project.loops[0].control_points, before);
    }

    #[test]
    fn loop_point_info_reports_shared_counts() {
        let project = shared_pair_project();

        let first = project
            .loop_point_info("procedural-drift-loop")
            .expect("first info");
        assert_eq!(first.len(), 8);
        let cp0 = point_info(&first, "cp-0");
        assert!(cp0.shared);
        assert_eq!(cp0.shared_count, 2);
        let cp1 = point_info(&first, "cp-1");
        assert!(cp1.shared);
        assert_eq!(cp1.shared_count, 2);
        let cp2 = point_info(&first, "cp-2");
        assert!(!cp2.shared);
        assert_eq!(cp2.shared_count, 1);

        let second = project.loop_point_info("short-loop").expect("second info");
        assert_eq!(second.len(), 5);
        assert_eq!(point_info(&second, "cp-0").shared_count, 2);
        assert_eq!(point_info(&second, "cp-1").shared_count, 2);
        assert_eq!(point_info(&second, "cp-a").shared_count, 1);
        assert!(!point_info(&second, "cp-a").shared);
        assert!(project.loop_point_info("missing-loop").is_err());
    }

    fn square_editor_project() -> EditorProject {
        EditorProject {
            schema: EDITOR_PROJECT_SCHEMA.to_string(),
            name: "Square".to_string(),
            settings: EditorSettings::default(),
            loops: vec![EditorLoop {
                id: "main".to_string(),
                name: "Main".to_string(),
                control_points: vec![
                    "a".to_string(),
                    "b".to_string(),
                    "c".to_string(),
                    "d".to_string(),
                ],
                sectors: Vec::new(),
                cameras: Vec::new(),
                spawns: Vec::new(),
            }],
            control_points: vec![
                EditorControlPoint {
                    id: "a".to_string(),
                    position: [0.0, 0.0],
                    kind: ControlPointKind::StartFinish,
                    tangent: None,
                },
                EditorControlPoint {
                    id: "b".to_string(),
                    position: [10.0, 0.0],
                    kind: ControlPointKind::Normal,
                    tangent: None,
                },
                EditorControlPoint {
                    id: "c".to_string(),
                    position: [10.0, 10.0],
                    kind: ControlPointKind::Normal,
                    tangent: None,
                },
                EditorControlPoint {
                    id: "d".to_string(),
                    position: [0.0, 10.0],
                    kind: ControlPointKind::Normal,
                    tangent: None,
                },
            ],
            segments: Vec::new(),
            pit_lane: None,
        }
        .normalize()
        .expect("square editor project")
    }

    fn add_square_branch_loop(project: &mut EditorProject) {
        project.control_points.push(EditorControlPoint {
            id: "e".to_string(),
            position: [15.0, -5.0],
            kind: ControlPointKind::Normal,
            tangent: None,
        });
        project.loops.push(EditorLoop {
            id: "branch".to_string(),
            name: "Branch".to_string(),
            control_points: vec!["b".to_string(), "e".to_string(), "d".to_string()],
            sectors: Vec::new(),
            cameras: Vec::new(),
            spawns: Vec::new(),
        });
        for (from, to) in [("b", "e"), ("e", "d"), ("d", "b")] {
            project.segments.push(EditorSegment {
                from: from.to_string(),
                to: to.to_string(),
                role: EditorSegmentRole::Branch,
                kerbs: Vec::new(),
                prop_spans: Vec::new(),
            });
        }
    }

    fn loop_edge_tangents(
        graph: &TrackProject,
        layout_id: &str,
    ) -> BTreeMap<String, ([f32; 2], [f32; 2])> {
        let layout = graph.layout_by_id(layout_id).expect("layout");
        let mut tangents = BTreeMap::new();
        for edge_id in &layout.route {
            let edge = graph.edge_by_id(edge_id).expect("edge");
            let from = graph
                .control_points
                .iter()
                .find(|point| point.id == edge.from)
                .expect("from point");
            let to = graph
                .control_points
                .iter()
                .find(|point| point.id == edge.to)
                .expect("to point");
            tangents.insert(
                edge_id.clone(),
                (
                    *from.branch_tangents.get(edge_id).expect("from tangent"),
                    *to.branch_tangents.get(edge_id).expect("to tangent"),
                ),
            );
        }
        tangents
    }

    fn assert_tangent_close(actual: [f32; 2], expected: [f32; 2]) {
        assert!(
            (actual[0] - expected[0]).abs() < 1e-5,
            "tangent x {actual:?} != {expected:?}"
        );
        assert!(
            (actual[1] - expected[1]).abs() < 1e-5,
            "tangent z {actual:?} != {expected:?}"
        );
    }

    #[test]
    fn segment_role_round_trips_and_v2_migration_defaults_to_main() {
        let mut project = square_editor_project();
        project.segments[0].role = EditorSegmentRole::Branch;
        let json = serde_json::to_string(&project).expect("serialize editor project");
        let parsed = EditorProject::from_json(&json).expect("parse editor project");
        assert_eq!(parsed.schema, EDITOR_PROJECT_SCHEMA);
        assert_eq!(parsed, project);
        assert_eq!(
            parsed
                .segments
                .iter()
                .filter(|segment| segment.role == EditorSegmentRole::Branch)
                .count(),
            1
        );

        let mut graph = TrackProject::default_project();
        graph.schema = TRACK_PROJECT_SCHEMA.to_string();
        let json = serde_json::to_string(&graph).expect("serialize v2 graph");
        let migrated = EditorProject::from_json(&json).expect("migrate v2 graph");
        assert_eq!(migrated.schema, EDITOR_PROJECT_SCHEMA);
        assert!(!migrated.segments.is_empty());
        assert!(migrated
            .segments
            .iter()
            .all(|segment| segment.role == EditorSegmentRole::Main));
    }

    #[test]
    fn adding_a_branch_loop_preserves_original_graph_tangents_and_centerline() {
        let base = square_editor_project();
        let before = base.to_graph().expect("before graph");
        let before_tangents = loop_edge_tangents(&before, "main");
        let before_centerline = {
            let layout = before.layout_by_id("main").expect("before layout");
            layout_centerline(&before, layout).expect("before centerline")
        };

        let mut branched = base;
        add_square_branch_loop(&mut branched);
        let branched = branched.normalize().expect("branch project");
        let after = branched.to_graph().expect("after graph");
        assert!(after.edge_by_id("b->e").is_some());
        assert!(after.edge_by_id("e->d").is_some());
        assert!(after.edge_by_id("d->b").is_some());
        let after_tangents = loop_edge_tangents(&after, "main");
        let after_centerline = {
            let layout = after.layout_by_id("main").expect("after layout");
            layout_centerline(&after, layout).expect("after centerline")
        };

        assert_eq!(before_tangents, after_tangents);
        assert_eq!(before_centerline, after_centerline);
    }

    #[test]
    fn branch_edge_at_junction_uses_edge_direction_while_main_keeps_loop_tangent() {
        let mut project = square_editor_project();
        add_square_branch_loop(&mut project);
        let project = project.normalize().expect("branch project");
        let graph = project.to_graph().expect("graph");

        let position = |id: &str| {
            graph
                .control_points
                .iter()
                .find(|point| point.id == id)
                .expect("control point")
                .position
        };
        let direction = |from: &str, to: &str| {
            let from = position(from);
            let to = position(to);
            let dx = to[0] - from[0];
            let dz = to[1] - from[1];
            let length = dx.hypot(dz);
            [dx / length, dz / length]
        };
        let point = |id: &str| {
            graph
                .control_points
                .iter()
                .find(|point| point.id == id)
                .expect("control point")
        };

        let b = point("b");
        let d = point("d");

        let main_at_b = b.branch_tangents.get("a->b").copied().expect("main b");
        assert_eq!(b.branch_tangents.get("b->c").copied(), Some(main_at_b));
        let branch_b_to_e = b.branch_tangents.get("b->e").copied().expect("branch b->e");
        assert_tangent_close(branch_b_to_e, direction("b", "e"));
        assert_ne!(branch_b_to_e, main_at_b);
        assert_tangent_close(
            b.branch_tangents
                .get("d->b")
                .copied()
                .expect("branch d->b at b"),
            direction("d", "b"),
        );

        let main_at_d = d.branch_tangents.get("c->d").copied().expect("main d");
        assert_eq!(d.branch_tangents.get("d->a").copied(), Some(main_at_d));
        let branch_e_to_d = d.branch_tangents.get("e->d").copied().expect("branch e->d");
        assert_tangent_close(branch_e_to_d, direction("e", "d"));
        assert_ne!(branch_e_to_d, main_at_d);
        assert_tangent_close(
            d.branch_tangents
                .get("d->b")
                .copied()
                .expect("branch d->b at d"),
            direction("d", "b"),
        );
    }

    #[test]
    fn shared_main_segment_is_identical_in_both_loops() {
        let project = shared_pair_project();
        let graph = project.to_graph().expect("graph");
        let shared = "cp-0->cp-1";

        assert_eq!(
            graph.edges.iter().filter(|edge| edge.id == shared).count(),
            1
        );
        assert_eq!(
            graph
                .layouts
                .iter()
                .filter(|layout| layout.route.iter().any(|edge_id| edge_id == shared))
                .count(),
            2
        );

        let unique = unique_route_edges(&graph);
        assert_eq!(unique.len(), graph.edges.len());
        let built = build_unique_edges(&graph, &unique).expect("built edges");
        assert_eq!(built.len(), unique.len());
        let built_shared = built.get(shared).expect("shared built edge");
        assert!(built_shared.points.len() >= 2);

        let point = |id: &str| {
            graph
                .control_points
                .iter()
                .find(|point| point.id == id)
                .expect("control point")
        };
        let cp0 = point("cp-0");
        let cp1 = point("cp-1");
        let shared_from = cp0
            .branch_tangents
            .get(shared)
            .copied()
            .expect("shared from tangent");
        let shared_to = cp1
            .branch_tangents
            .get(shared)
            .copied()
            .expect("shared to tangent");
        assert_eq!(
            cp0.branch_tangents.get("cp-7->cp-0").copied(),
            Some(shared_from)
        );
        assert_eq!(
            cp1.branch_tangents.get("cp-1->cp-2").copied(),
            Some(shared_to)
        );
    }

    #[test]
    fn segment_and_junction_helpers_locate_graph_parts() {
        let project = TrackProject::default_project();
        assert!(project.segment_by_id("edge-0").is_some());
        assert_eq!(project.segment_index("edge-0"), Some(0));
        assert!(project.junction_by_id("control-0").is_some());
        assert_eq!(project.junction_index("control-0"), Some(0));
        assert_eq!(
            project
                .route_segments("procedural-drift-loop")
                .map(|segments| segments.len()),
            Some(8)
        );
        assert!(project.segment_by_id("missing").is_none());
        assert!(project.junction_by_id("missing").is_none());
        assert!(project.route_segments("missing").is_none());
    }

    #[test]
    fn move_junction_updates_position_and_rejects_invalid_input() {
        let mut project = TrackProject::default_project();
        project
            .move_junction("control-0", 5.0, 7.0)
            .expect("move junction");
        let junction = project.junction_by_id("control-0").expect("junction");
        assert_eq!(junction.position, [5.0, 7.0]);

        assert!(project.move_junction("missing", 0.0, 0.0).is_err());
        assert!(project.move_junction("control-0", f32::NAN, 0.0).is_err());
        let junction = project.junction_by_id("control-0").expect("junction");
        assert_eq!(junction.position, [5.0, 7.0]);
    }

    #[test]
    fn insert_and_delete_junction_round_trip() {
        let mut project = TrackProject::default_project();
        let first = project.insert_junction(12.0, 34.0);
        let second = project.insert_junction(56.0, 78.0);
        assert_ne!(first, second);
        assert_eq!(
            project.junction_by_id(&first).expect("junction").position,
            [12.0, 34.0]
        );

        assert!(project.delete_junction("missing").is_err());
        assert!(project.delete_junction("control-0").is_err());
        project.delete_junction(&first).expect("delete first");
        project.delete_junction(&second).expect("delete second");
        assert!(project.junction_by_id(&first).is_none());
        project.normalize().expect("project stays valid");
    }

    #[test]
    fn insert_and_delete_interior_segment_points() {
        let mut project = TrackProject::default_project();
        project.edges[0].points = vec![[1.0, 2.0], [3.0, 4.0]];

        let inserted = project
            .insert_segment_point("edge-0", 0, 5.0, 6.0)
            .expect("insert point");
        assert_eq!(inserted, 1);
        assert_eq!(
            project.segment_by_id("edge-0").expect("segment").points,
            vec![[1.0, 2.0], [5.0, 6.0], [3.0, 4.0]]
        );

        project
            .delete_segment_point("edge-0", inserted)
            .expect("delete point");
        assert_eq!(
            project.segment_by_id("edge-0").expect("segment").points,
            vec![[1.0, 2.0], [3.0, 4.0]]
        );

        let empty = project
            .insert_segment_point("edge-1", 0, 9.0, 8.0)
            .expect("insert into empty");
        assert_eq!(empty, 0);
        assert_eq!(
            project.segment_by_id("edge-1").expect("segment").points,
            vec![[9.0, 8.0]]
        );

        assert!(project
            .insert_segment_point("missing", 0, 0.0, 0.0)
            .is_err());
        assert!(project.delete_segment_point("missing", 0).is_err());
        assert!(project.delete_segment_point("edge-0", 99).is_err());
    }

    #[test]
    fn move_segment_point_updates_interior_point() {
        let mut project = TrackProject::default_project();
        project.edges[0].points = vec![[1.0, 2.0], [3.0, 4.0]];

        project
            .move_segment_point("edge-0", 1, 7.5, -2.5)
            .expect("move point");
        assert_eq!(
            project.segment_by_id("edge-0").expect("segment").points,
            vec![[1.0, 2.0], [7.5, -2.5]]
        );
        project.clone().normalize().expect("project stays valid");
    }

    #[test]
    fn move_segment_point_rejects_invalid_input_and_keeps_original() {
        let mut project = TrackProject::default_project();
        project.edges[0].points = vec![[1.0, 2.0], [3.0, 4.0]];

        assert!(project.move_segment_point("missing", 0, 0.0, 0.0).is_err());
        assert!(project.move_segment_point("edge-0", 99, 0.0, 0.0).is_err());
        assert!(project
            .move_segment_point("edge-0", 0, f32::NAN, 0.0)
            .is_err());
        assert!(project
            .move_segment_point("edge-0", 0, 0.0, f32::INFINITY)
            .is_err());
        assert_eq!(
            project.segment_by_id("edge-0").expect("segment").points,
            vec![[1.0, 2.0], [3.0, 4.0]]
        );
    }

    #[test]
    fn make_segment_unique_only_repoints_the_selected_route() {
        let mut project = fork_project();
        let segment = project
            .edges
            .iter_mut()
            .find(|segment| segment.id == "e1")
            .expect("shared segment");
        segment.points = vec![[10.0, 5.0], [20.0, 5.0]];
        segment.kerbs = vec![KerbSpan {
            start: 0.1,
            end: 0.4,
            side: 1,
        }];
        segment.prop_spans = vec![PropSpan {
            spline: "prop-near-left".to_string(),
            start: 0.2,
            end: 0.6,
            kind: PropKind::Blocks,
        }];

        let unique = project
            .make_segment_unique("short", "e1")
            .expect("make segment unique");
        assert_ne!(unique, "e1");

        let full_route = project.route_segments("full").expect("full route").to_vec();
        let short_route = project
            .route_segments("short")
            .expect("short route")
            .to_vec();
        assert!(full_route.contains(&"e1".to_string()));
        assert!(!short_route.contains(&"e1".to_string()));
        assert!(short_route.contains(&unique));

        let original = project.segment_by_id("e1").expect("original");
        let cloned = project.segment_by_id(&unique).expect("clone");
        assert_eq!(cloned.from, original.from);
        assert_eq!(cloned.to, original.to);
        assert_eq!(cloned.points, original.points);
        assert_eq!(cloned.from_tangent, original.from_tangent);
        assert_eq!(cloned.to_tangent, original.to_tangent);
        assert_eq!(cloned.kerbs, original.kerbs);
        assert_eq!(cloned.prop_spans, original.prop_spans);

        assert!(project.make_segment_unique("missing", "e1").is_err());
        assert!(project.make_segment_unique("full", "missing").is_err());
    }

    #[test]
    fn set_segment_kerbs_props_and_tangents() {
        let mut project = TrackProject::default_project();
        let kerbs = [KerbSpan {
            start: 0.1,
            end: 0.5,
            side: -1,
        }];
        project
            .set_segment_kerb_spans("edge-0", &kerbs)
            .expect("set kerbs");
        assert_eq!(
            project.segment_by_id("edge-0").expect("segment").kerbs,
            kerbs
        );

        let props = [PropSpan {
            spline: "prop-near-left".to_string(),
            start: 0.2,
            end: 0.7,
            kind: PropKind::Tyres,
        }];
        project
            .set_segment_prop_spans("edge-0", &props)
            .expect("set props");
        assert_eq!(
            project.segment_by_id("edge-0").expect("segment").prop_spans,
            props
        );

        project
            .set_segment_tangents("edge-0", Some([3.0, 4.0]), Some([0.0, 1.0]))
            .expect("set tangents");
        let segment = project.segment_by_id("edge-0").expect("segment");
        let from = segment.from_tangent.expect("from tangent");
        assert!((from[0] - 0.6).abs() < 1e-6);
        assert!((from[1] - 0.8).abs() < 1e-6);
        assert_eq!(segment.to_tangent, Some([0.0, 1.0]));

        project
            .set_segment_from_tangent("edge-0", Some([1.0, 0.0]))
            .expect("set from tangent");
        project
            .set_segment_to_tangent("edge-0", None)
            .expect("clear to tangent");
        let segment = project.segment_by_id("edge-0").expect("segment");
        assert_eq!(segment.from_tangent, Some([1.0, 0.0]));
        assert_eq!(segment.to_tangent, None);

        assert!(project.set_segment_kerb_spans("missing", &[]).is_err());
        assert!(project
            .set_segment_kerb_spans(
                "edge-0",
                &[KerbSpan {
                    start: 1.5,
                    end: 0.2,
                    side: 0,
                }],
            )
            .is_err());
        assert!(project
            .set_segment_prop_spans(
                "edge-0",
                &[PropSpan {
                    spline: "missing".to_string(),
                    start: 0.0,
                    end: 0.5,
                    kind: PropKind::Blocks,
                }],
            )
            .is_err());
        assert!(project
            .set_segment_tangents("edge-0", Some([0.0, 0.0]), None)
            .is_err());
        assert!(project
            .set_segment_tangents("missing", Some([1.0, 0.0]), None)
            .is_err());
        // A rejected mutation leaves the previous tangents in place.
        let segment = project.segment_by_id("edge-0").expect("segment");
        assert_eq!(segment.from_tangent, Some([1.0, 0.0]));
        assert_eq!(segment.to_tangent, None);
    }

    #[test]
    fn segment_and_route_centerlines_use_the_edge_sampler() {
        let project = TrackProject::default_project();
        let segment = project
            .segment_centerline("edge-0")
            .expect("segment centerline");
        assert!(segment.len() >= 2);
        let start = project
            .junction_by_id("control-0")
            .expect("start junction")
            .position;
        assert!((segment[0][0] - start[0]).abs() < 1e-4);
        assert!((segment[0][2] - start[1]).abs() < 1e-4);

        let route = project
            .route_centerline("procedural-drift-loop")
            .expect("route centerline");
        assert!(route.len() >= 3);

        assert!(project.segment_centerline("missing").is_err());
        assert!(project.route_centerline("missing").is_err());
    }
}
