use std::collections::{BTreeMap, BTreeSet};
use std::f32::consts::{PI, TAU};

use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};
use sim_types::{
    Aabb, AssetKind, AssetManifest, AssetStats, CameraSpec, EntryPoints, MaterialRef, MeshBuffers,
    MeshRef, SectorSpec, SpawnPoint, SurfaceSpec, TextureRef, TrackCollision, TrackMetadata,
    TrackVisual, ENGINE_FRAME, ENGINE_UNITS, MAGIC, SCHEMA_VERSION,
};

pub const TRACK_PROJECT_SCHEMA: &str = "lapster/track-project@5";
const SEGMENTS: usize = 120;
const SEGMENT_CENTERLINE_STEPS: usize = 24;
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

fn default_prop_offset() -> f32 {
    PROP_OFFSET
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct PropLine {
    pub start: f32,
    pub end: f32,
    pub side: i8,
    #[serde(default = "default_prop_offset")]
    pub offset: f32,
    pub kind: PropKind,
    /// Optional spacing override in metres for decoration-path props.
    #[serde(default)]
    pub spacing: Option<f32>,
    /// Optional distance offset into the decoration path, in metres.
    #[serde(default)]
    pub phase: f32,
}

pub struct EditorTrack {
    pub visual: TrackVisual,
    pub metadata: TrackMetadata,
    pub collision: TrackCollision,
    pub manifest: AssetManifest,
    pub kerb_spans: Vec<KerbSpan>,
    pub prop_lines: Vec<PropLine>,
}

/// The canonical venue graph. Junctions are shared endpoints, segments are
/// connected splines between junctions, layouts are closed loops of segment
/// ids, and paths are open segment chains.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TrackProject {
    pub schema: String,
    pub name: String,
    pub settings: VenueSettings,
    pub junctions: Vec<Junction>,
    pub segments: Vec<Segment>,
    pub layouts: Vec<LayoutProject>,
    #[serde(default)]
    pub paths: Vec<PathProject>,
    #[serde(default)]
    pub decorations: Vec<DecorationPath>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VenueSettings {
    pub width: f32,
    pub kerb: KerbSettings,
}

impl Default for VenueSettings {
    fn default() -> Self {
        Self {
            width: ASPHALT_HALF_WIDTH * 2.0,
            kerb: KerbSettings::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KerbSettings {
    pub width: f32,
    pub depth: f32,
    pub element_length: f32,
}

impl Default for KerbSettings {
    fn default() -> Self {
        Self {
            width: KERB_OUTER_RADIUS - KERB_INNER_RADIUS,
            depth: KERB_TOP_OFFSET - KERB_SURFACE_OFFSET,
            element_length: TARMAC_TILE_METERS / 3.5,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    #[serde(default)]
    pub values: BTreeMap<String, f32>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JunctionKind {
    #[default]
    Normal,
    StartFinish,
    Fork,
    PitEntry,
    PitExit,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Junction {
    pub id: String,
    pub position: [f32; 2],
    #[serde(default)]
    pub kind: JunctionKind,
    #[serde(default)]
    pub profile: Option<Profile>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Segment {
    pub id: String,
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub points: Vec<[f32; 2]>,
    #[serde(default)]
    pub from_tangent: Option<[f32; 2]>,
    #[serde(default)]
    pub to_tangent: Option<[f32; 2]>,
    #[serde(default)]
    pub start_profile: Option<Profile>,
    #[serde(default)]
    pub end_profile: Option<Profile>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StartFinish {
    pub segment_id: String,
    #[serde(default)]
    pub fraction: f32,
}

impl Default for StartFinish {
    fn default() -> Self {
        Self {
            segment_id: String::new(),
            fraction: 0.0,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LayoutProject {
    pub id: String,
    pub name: String,
    pub route: Vec<String>,
    #[serde(default)]
    pub start_finish: StartFinish,
    #[serde(default)]
    pub spawns: Vec<SpawnPoint>,
    #[serde(default)]
    pub sectors: Vec<SectorSpec>,
    #[serde(default)]
    pub cameras: Vec<CameraSpec>,
}

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
pub struct DecorationAnchor {
    pub segment_id: String,
    #[serde(default)]
    pub fraction: f32,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DecorationKind {
    #[default]
    Kerb,
    Prop,
}

/// A canonical decoration path over an ordered, contiguous segment chain.
/// Per-segment slices are derived at bake time; they are never serialized.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DecorationPath {
    pub id: String,
    pub kind: DecorationKind,
    #[serde(default)]
    pub side: i8,
    #[serde(default)]
    pub lane: Option<String>,
    pub segments: Vec<String>,
    #[serde(default)]
    pub start: Option<DecorationAnchor>,
    #[serde(default)]
    pub end: Option<DecorationAnchor>,
    #[serde(default)]
    pub elements: Vec<f32>,
    #[serde(default)]
    pub phase: f32,
    #[serde(default)]
    pub prop_kind: Option<PropKind>,
    #[serde(default)]
    pub spacing: Option<f32>,
}

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

    /// Builds a canonical one-loop project from ordered XZ control points.
    pub fn ring_project(name: &str, layout_id: &str, control_points: &[[f32; 2]]) -> Self {
        let junctions = control_points
            .iter()
            .enumerate()
            .map(|(index, position)| Junction {
                id: format!("junction-{index}"),
                position: *position,
                kind: JunctionKind::Normal,
                profile: None,
            })
            .collect::<Vec<_>>();
        let segments = junctions
            .iter()
            .enumerate()
            .map(|(index, from)| Segment {
                id: format!("segment-{index}"),
                from: from.id.clone(),
                to: junctions[(index + 1) % junctions.len()].id.clone(),
                points: Vec::new(),
                from_tangent: None,
                to_tangent: None,
                start_profile: None,
                end_profile: None,
            })
            .collect::<Vec<_>>();
        let route = segments
            .iter()
            .map(|segment| segment.id.clone())
            .collect::<Vec<_>>();
        let start_finish = StartFinish {
            segment_id: segments
                .first()
                .map(|segment| segment.id.clone())
                .unwrap_or_default(),
            fraction: 0.0,
        };
        Self {
            schema: TRACK_PROJECT_SCHEMA.to_string(),
            name: name.to_string(),
            settings: VenueSettings::default(),
            junctions,
            segments,
            layouts: vec![LayoutProject {
                id: layout_id.to_string(),
                name: name.to_string(),
                route,
                start_finish,
                spawns: Vec::new(),
                sectors: Vec::new(),
                cameras: Vec::new(),
            }],
            paths: Vec::new(),
            decorations: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn from_json(json: &str) -> Result<Self, String> {
        let value = serde_json::from_str::<serde_json::Value>(json)
            .map_err(|error| format!("invalid track project JSON: {error}"))?;
        let schema = value
            .get("schema")
            .and_then(|schema| schema.as_str())
            .unwrap_or_default();
        if schema != TRACK_PROJECT_SCHEMA {
            return Err(format!(
                "unsupported track project schema '{schema}'; expected {TRACK_PROJECT_SCHEMA}"
            ));
        }
        serde_json::from_value::<Self>(value)
            .map_err(|error| format!("invalid track project: {error}"))?
            .normalize()
    }

    #[allow(dead_code)]
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(&self.clone().normalize()?)
            .map_err(|error| format!("failed to serialize track project: {error}"))
    }

    pub fn normalize(mut self) -> Result<Self, String> {
        self.normalize_in_place()?;
        Ok(self)
    }

    fn normalize_in_place(&mut self) -> Result<(), String> {
        if self.schema != TRACK_PROJECT_SCHEMA {
            return Err(format!(
                "unsupported track project schema '{}'; expected {TRACK_PROJECT_SCHEMA}",
                self.schema
            ));
        }
        self.name = self.name.trim().to_string();
        if !self.settings.width.is_finite() || self.settings.width <= 0.0 {
            return Err("venue width must be finite and positive".to_string());
        }
        if !self.settings.kerb.width.is_finite()
            || !self.settings.kerb.depth.is_finite()
            || !self.settings.kerb.element_length.is_finite()
            || self.settings.kerb.width <= 0.0
            || self.settings.kerb.depth <= 0.0
            || self.settings.kerb.element_length <= 0.0
        {
            return Err("kerb settings must be finite and positive".to_string());
        }

        validate_unique_ids(
            self.junctions.iter().map(|junction| junction.id.as_str()),
            "junction",
        )?;
        validate_unique_ids(
            self.segments.iter().map(|segment| segment.id.as_str()),
            "segment",
        )?;
        validate_unique_ids(
            self.layouts.iter().map(|layout| layout.id.as_str()),
            "layout",
        )?;
        validate_unique_ids(self.paths.iter().map(|path| path.id.as_str()), "path")?;
        validate_unique_ids(
            self.decorations
                .iter()
                .map(|decoration| decoration.id.as_str()),
            "decoration",
        )?;

        if self.junctions.is_empty() {
            return Err("track project needs at least one junction".to_string());
        }
        for junction in &self.junctions {
            if junction.id.trim().is_empty() {
                return Err("junction ids must not be empty".to_string());
            }
            if !junction.position[0].is_finite() || !junction.position[1].is_finite() {
                return Err(format!(
                    "junction '{}' has a non-finite position",
                    junction.id
                ));
            }
            validate_profile(
                junction.profile.as_ref(),
                &format!("junction '{}'", junction.id),
            )?;
        }

        let junction_ids = self
            .junctions
            .iter()
            .map(|junction| junction.id.as_str())
            .collect::<BTreeSet<_>>();
        for segment in &mut self.segments {
            if segment.id.trim().is_empty() {
                return Err("segment ids must not be empty".to_string());
            }
            if segment.from == segment.to {
                return Err(format!("segment '{}' must not self-loop", segment.id));
            }
            for endpoint in [&segment.from, &segment.to] {
                if !junction_ids.contains(endpoint.as_str()) {
                    return Err(format!(
                        "segment '{}' references unknown junction '{endpoint}'",
                        segment.id
                    ));
                }
            }
            for point in &segment.points {
                if !point[0].is_finite() || !point[1].is_finite() {
                    return Err(format!(
                        "segment '{}' has a non-finite interior point",
                        segment.id
                    ));
                }
            }
            normalize_tangent(&mut segment.from_tangent, &segment.id, "from")?;
            normalize_tangent(&mut segment.to_tangent, &segment.id, "to")?;
            validate_profile(
                segment.start_profile.as_ref(),
                &format!("segment '{}' start profile", segment.id),
            )?;
            validate_profile(
                segment.end_profile.as_ref(),
                &format!("segment '{}' end profile", segment.id),
            )?;
        }

        if self.layouts.is_empty() {
            return Err("track project needs at least one layout".to_string());
        }
        let segment_ids = self
            .segments
            .iter()
            .map(|segment| segment.id.as_str())
            .collect::<BTreeSet<_>>();
        for layout in &self.layouts {
            if layout.id.trim().is_empty() {
                return Err("layout ids must not be empty".to_string());
            }
            if layout.route.is_empty() {
                return Err(format!("layout '{}' route must not be empty", layout.id));
            }
            let mut seen = BTreeSet::new();
            for (index, segment_id) in layout.route.iter().enumerate() {
                if !seen.insert(segment_id.as_str()) {
                    return Err(format!(
                        "layout '{}' route repeats segment '{segment_id}'",
                        layout.id
                    ));
                }
                let segment = self
                    .segments
                    .iter()
                    .find(|segment| segment.id == *segment_id)
                    .ok_or_else(|| {
                        format!(
                            "layout '{}' route references unknown segment '{segment_id}'",
                            layout.id
                        )
                    })?;
                let next_id = &layout.route[(index + 1) % layout.route.len()];
                let next = self
                    .segments
                    .iter()
                    .find(|segment| segment.id == *next_id)
                    .ok_or_else(|| {
                        format!(
                            "layout '{}' route references unknown segment '{next_id}'",
                            layout.id
                        )
                    })?;
                if segment.to != next.from {
                    return Err(format!(
                        "layout '{}' route is not a closed canonical chain: segment '{}' ends at '{}' but next segment '{}' starts at '{}'",
                        layout.id, segment.id, segment.to, next.id, next.from
                    ));
                }
            }
            if !segment_ids.contains(layout.start_finish.segment_id.as_str()) {
                return Err(format!(
                    "layout '{}' start/finish references unknown segment '{}'",
                    layout.id, layout.start_finish.segment_id
                ));
            }
            if !layout
                .route
                .iter()
                .any(|segment_id| segment_id == &layout.start_finish.segment_id)
            {
                return Err(format!(
                    "layout '{}' start/finish segment '{}' is missing from its route",
                    layout.id, layout.start_finish.segment_id
                ));
            }
            if !layout.start_finish.fraction.is_finite()
                || layout.start_finish.fraction < 0.0
                || layout.start_finish.fraction >= 1.0
            {
                return Err(format!(
                    "layout '{}' start/finish fraction must lie in [0, 1)",
                    layout.id
                ));
            }
        }

        for path in &self.paths {
            if path.id.trim().is_empty() {
                return Err("path ids must not be empty".to_string());
            }
            if path.route.is_empty() {
                return Err(format!("path '{}' route must not be empty", path.id));
            }
            for (index, segment_id) in path.route.iter().enumerate() {
                let segment = self
                    .segments
                    .iter()
                    .find(|segment| segment.id == *segment_id)
                    .ok_or_else(|| {
                        format!(
                            "path '{}' references unknown segment '{segment_id}'",
                            path.id
                        )
                    })?;
                if let Some(next_id) = path.route.get(index + 1) {
                    let next = self
                        .segments
                        .iter()
                        .find(|segment| segment.id == *next_id)
                        .ok_or_else(|| {
                            format!("path '{}' references unknown segment '{next_id}'", path.id)
                        })?;
                    if segment.to != next.from {
                        return Err(format!(
                            "path '{}' route is not an open canonical chain: segment '{}' ends at '{}' but next segment '{}' starts at '{}'",
                            path.id, segment.id, segment.to, next.id, next.from
                        ));
                    }
                }
            }
        }

        for decoration in &self.decorations {
            if decoration.id.trim().is_empty() {
                return Err("decoration ids must not be empty".to_string());
            }
            if decoration.segments.is_empty() {
                return Err(format!(
                    "decoration '{}' needs at least one segment",
                    decoration.id
                ));
            }
            for (index, segment_id) in decoration.segments.iter().enumerate() {
                let segment = self
                    .segments
                    .iter()
                    .find(|segment| segment.id == *segment_id)
                    .ok_or_else(|| {
                        format!(
                            "decoration '{}' references unknown segment '{segment_id}'",
                            decoration.id
                        )
                    })?;
                if let Some(next_id) = decoration.segments.get(index + 1) {
                    let next = self
                        .segments
                        .iter()
                        .find(|segment| segment.id == *next_id)
                        .ok_or_else(|| {
                            format!(
                                "decoration '{}' references unknown segment '{next_id}'",
                                decoration.id
                            )
                        })?;
                    if segment.to != next.from {
                        return Err(format!(
                            "decoration '{}' chain is not contiguous: segment '{}' ends at '{}' but next segment '{}' starts at '{}'",
                            decoration.id, segment.id, segment.to, next.id, next.from
                        ));
                    }
                }
            }
            for (label, anchor) in [("start", &decoration.start), ("end", &decoration.end)] {
                let Some(anchor) = anchor else {
                    continue;
                };
                if !decoration
                    .segments
                    .iter()
                    .any(|segment_id| segment_id == &anchor.segment_id)
                {
                    return Err(format!(
                        "decoration '{}' {label} anchor references segment '{}' outside its chain",
                        decoration.id, anchor.segment_id
                    ));
                }
                if !anchor.fraction.is_finite() || !(0.0..=1.0).contains(&anchor.fraction) {
                    return Err(format!(
                        "decoration '{}' {label} fraction must lie in [0, 1]",
                        decoration.id
                    ));
                }
            }
            for element in &decoration.elements {
                if !element.is_finite() || *element <= 0.0 {
                    return Err(format!(
                        "decoration '{}' element lengths must be finite and positive",
                        decoration.id
                    ));
                }
            }
            if !decoration.phase.is_finite() {
                return Err(format!(
                    "decoration '{}' phase must be finite",
                    decoration.id
                ));
            }
            if let Some(spacing) = decoration.spacing {
                if !spacing.is_finite() || spacing <= 0.0 {
                    return Err(format!(
                        "decoration '{}' spacing must be finite and positive",
                        decoration.id
                    ));
                }
            }
        }
        validate_decoration_coverage(&self.decorations)?;

        Ok(())
    }

    #[allow(dead_code)]
    pub fn default_layout_id(&self) -> &str {
        self.layouts
            .first()
            .map(|layout| layout.id.as_str())
            .unwrap_or_default()
    }

    pub fn junction_by_id(&self, id: &str) -> Option<&Junction> {
        self.junctions.iter().find(|junction| junction.id == id)
    }

    pub fn segment_by_id(&self, id: &str) -> Option<&Segment> {
        self.segments.iter().find(|segment| segment.id == id)
    }

    #[allow(dead_code)]
    pub fn layout_by_id(&self, id: &str) -> Option<&LayoutProject> {
        self.layouts.iter().find(|layout| layout.id == id)
    }

    pub fn layout_index(&self, id: &str) -> Option<usize> {
        self.layouts.iter().position(|layout| layout.id == id)
    }

    /// Moves a junction. Decoration anchors are fractions along segments, so
    /// they reparameterise automatically when the junction moves.
    #[allow(dead_code)]
    pub fn move_junction(&mut self, id: &str, x: f32, z: f32) -> Result<(), String> {
        if !x.is_finite() || !z.is_finite() {
            return Err("junction position must be finite".to_string());
        }
        let junction = self
            .junctions
            .iter_mut()
            .find(|junction| junction.id == id)
            .ok_or_else(|| format!("unknown junction id '{id}'"))?;
        junction.position = [x, z];
        self.normalize_in_place()
    }

    #[allow(dead_code)]
    pub fn set_junction_position(&mut self, id: &str, position: [f32; 2]) -> Result<(), String> {
        self.move_junction(id, position[0], position[1])
    }

    /// Splits a segment into two canonical segments. The first keeps the
    /// original id; the second gets a new id. Every layout, path, and
    /// decoration chain is updated and decoration anchors are rebased.
    #[allow(dead_code)]
    pub fn split_segment(&mut self, segment_id: &str, fraction: f32) -> Result<String, String> {
        self.normalize_in_place()?;
        if !fraction.is_finite() || fraction <= 1e-4 || fraction >= 1.0 - 1e-4 {
            return Err("segment split fraction must lie strictly inside (0, 1)".to_string());
        }
        let index = self
            .segments
            .iter()
            .position(|segment| segment.id == segment_id)
            .ok_or_else(|| format!("unknown segment id '{segment_id}'"))?;
        let original = self.segments[index].clone();
        let split_point = sample_segment_at(self, &original, fraction)?;
        let before = sample_segment_at(self, &original, (fraction - 1e-3).max(0.0))?;
        let after = sample_segment_at(self, &original, (fraction + 1e-3).min(0.999_999))?;
        let direction = (after - before).normalize_or_zero();
        let split_tangent = if direction.length_squared() > 1e-8 {
            Some([direction.x, direction.z])
        } else {
            None
        };

        let new_junction_id = unique_track_id(
            &format!("junction-split-{segment_id}"),
            self.junctions.iter().map(|junction| junction.id.as_str()),
        );
        let new_segment_id = unique_track_id(
            &format!("segment-split-{segment_id}"),
            self.segments.iter().map(|segment| segment.id.as_str()),
        );
        let cut = if original.points.is_empty() {
            0
        } else {
            (((original.points.len() + 1) as f32 * fraction).round() as usize)
                .clamp(0, original.points.len())
        };

        let junction_insert = self
            .junctions
            .iter()
            .position(|junction| junction.id == original.from)
            .map(|index| index + 1)
            .unwrap_or(self.junctions.len());
        self.junctions.insert(
            junction_insert,
            Junction {
                id: new_junction_id.clone(),
                position: [split_point.x, split_point.z],
                kind: JunctionKind::Normal,
                profile: None,
            },
        );

        let first = Segment {
            id: original.id.clone(),
            from: original.from.clone(),
            to: new_junction_id.clone(),
            points: original.points[..cut].to_vec(),
            from_tangent: original.from_tangent,
            to_tangent: split_tangent,
            start_profile: original.start_profile.clone(),
            end_profile: None,
        };
        let second = Segment {
            id: new_segment_id.clone(),
            from: new_junction_id,
            to: original.to.clone(),
            points: original.points[cut..].to_vec(),
            from_tangent: split_tangent,
            to_tangent: original.to_tangent,
            start_profile: None,
            end_profile: original.end_profile.clone(),
        };
        self.segments[index] = first;
        self.segments.insert(index + 1, second);

        for layout in &mut self.layouts {
            layout.route = replace_chain_segment(&layout.route, segment_id, &new_segment_id);
            rebase_start_finish(
                &mut layout.start_finish,
                segment_id,
                &new_segment_id,
                fraction,
            );
        }
        for path in &mut self.paths {
            path.route = replace_chain_segment(&path.route, segment_id, &new_segment_id);
        }
        for decoration in &mut self.decorations {
            decoration.segments =
                replace_chain_segment(&decoration.segments, segment_id, &new_segment_id);
            if let Some(anchor) = &mut decoration.start {
                rebase_decoration_anchor(anchor, segment_id, &new_segment_id, fraction);
            }
            if let Some(anchor) = &mut decoration.end {
                rebase_decoration_anchor(anchor, segment_id, &new_segment_id, fraction);
            }
        }

        self.normalize_in_place()?;
        Ok(new_segment_id)
    }

    /// Deletes an unreferenced segment. Segments used by layouts, paths, or
    /// decoration paths are blocked; decoration is never silently joined
    /// across the gap.
    #[allow(dead_code)]
    pub fn delete_segment(&mut self, id: &str) -> Result<(), String> {
        self.normalize_in_place()?;
        if !self.segments.iter().any(|segment| segment.id == id) {
            return Err(format!("unknown segment id '{id}'"));
        }
        if self.decorations.iter().any(|decoration| {
            decoration
                .segments
                .iter()
                .any(|segment_id| segment_id == id)
        }) {
            return Err(format!(
                "cannot delete segment '{id}': decoration paths reference it; split or delete the decoration paths first"
            ));
        }
        if self.layouts.iter().any(|layout| {
            layout.route.iter().any(|segment_id| segment_id == id)
                || layout.start_finish.segment_id == id
        }) || self
            .paths
            .iter()
            .any(|path| path.route.iter().any(|segment_id| segment_id == id))
        {
            return Err(format!(
                "cannot delete segment '{id}': it is used by a layout or path"
            ));
        }
        self.segments.retain(|segment| segment.id != id);
        self.normalize_in_place()
    }

    /// Moves one interior point of a segment.
    pub fn move_segment_point(
        &mut self,
        segment_id: &str,
        index: usize,
        x: f32,
        z: f32,
    ) -> Result<(), String> {
        if !x.is_finite() || !z.is_finite() {
            return Err("segment point position must be finite".to_string());
        }
        self.normalize_in_place()?;
        let mut candidate = self.clone();
        let segment = candidate
            .segments
            .iter_mut()
            .find(|segment| segment.id == segment_id)
            .ok_or_else(|| format!("unknown segment id '{segment_id}'"))?;
        if index >= segment.points.len() {
            return Err(format!(
                "segment '{segment_id}' interior point index {index} is out of range ({} points)",
                segment.points.len()
            ));
        }
        segment.points[index] = [x, z];
        candidate.normalize_in_place()?;
        *self = candidate;
        Ok(())
    }

    /// Inserts an interior point after `after_index`. The returned value is the
    /// index of the inserted point.
    pub fn insert_segment_point(
        &mut self,
        segment_id: &str,
        after_index: usize,
        x: f32,
        z: f32,
    ) -> Result<usize, String> {
        if !x.is_finite() || !z.is_finite() {
            return Err("segment point position must be finite".to_string());
        }
        self.normalize_in_place()?;
        let mut candidate = self.clone();
        let segment = candidate
            .segments
            .iter_mut()
            .find(|segment| segment.id == segment_id)
            .ok_or_else(|| format!("unknown segment id '{segment_id}'"))?;
        let index = after_index.saturating_add(1).min(segment.points.len());
        segment.points.insert(index, [x, z]);
        candidate.normalize_in_place()?;
        *self = candidate;
        Ok(index)
    }

    /// Deletes one interior point of a segment.
    pub fn delete_segment_point(&mut self, segment_id: &str, index: usize) -> Result<(), String> {
        self.normalize_in_place()?;
        let mut candidate = self.clone();
        let segment = candidate
            .segments
            .iter_mut()
            .find(|segment| segment.id == segment_id)
            .ok_or_else(|| format!("unknown segment id '{segment_id}'"))?;
        if index >= segment.points.len() {
            return Err(format!(
                "segment '{segment_id}' interior point index {index} is out of range ({} points)",
                segment.points.len()
            ));
        }
        segment.points.remove(index);
        candidate.normalize_in_place()?;
        *self = candidate;
        Ok(())
    }

    /// Sets or clears the explicit endpoint tangents of a segment.
    pub fn set_segment_tangents(
        &mut self,
        segment_id: &str,
        from_tangent: Option<[f32; 2]>,
        to_tangent: Option<[f32; 2]>,
    ) -> Result<(), String> {
        self.normalize_in_place()?;
        let mut candidate = self.clone();
        let segment = candidate
            .segments
            .iter_mut()
            .find(|segment| segment.id == segment_id)
            .ok_or_else(|| format!("unknown segment id '{segment_id}'"))?;
        segment.from_tangent = from_tangent;
        segment.to_tangent = to_tangent;
        candidate.normalize_in_place()?;
        *self = candidate;
        Ok(())
    }

    /// Samples one canonical segment across its whole length.
    pub fn segment_centerline(&self, segment_id: &str) -> Result<Vec<[f32; 3]>, String> {
        let project = self.clone().normalize()?;
        let segment = project
            .segment_by_id(segment_id)
            .ok_or_else(|| format!("unknown segment id '{segment_id}'"))?;
        let mut points = Vec::with_capacity(SEGMENT_CENTERLINE_STEPS + 1);
        for index in 0..=SEGMENT_CENTERLINE_STEPS {
            let local = index as f32 / SEGMENT_CENTERLINE_STEPS as f32;
            let mut point = sample_segment_at(&project, segment, local)?;
            point.y = terrain_height(point.x, point.z);
            points.push(point.to_array());
        }
        Ok(points)
    }

    /// Merges a segment with its outgoing neighbour. The shared junction must
    /// have exactly two incident segments, and every layout, path, and
    /// decoration chain must reference both segments adjacently or neither.
    pub fn merge_segment(&mut self, segment_id: &str) -> Result<(), String> {
        self.normalize_in_place()?;
        let index = self
            .segments
            .iter()
            .position(|segment| segment.id == segment_id)
            .ok_or_else(|| format!("unknown segment id '{segment_id}'"))?;
        let first = self.segments[index].clone();
        let shared = first.to.clone();
        let incident = self
            .segments
            .iter()
            .filter(|segment| segment.from == shared || segment.to == shared)
            .count();
        if incident != 2 {
            return Err(format!(
                "cannot merge segment '{}': junction '{}' must have exactly two incident segments, found {incident}",
                first.id, shared
            ));
        }
        let outgoing = self
            .segments
            .iter()
            .filter(|segment| segment.from == shared)
            .collect::<Vec<_>>();
        if outgoing.len() != 1 {
            return Err(format!(
                "cannot merge segment '{}': junction '{}' has an ambiguous outgoing segment",
                first.id, shared
            ));
        }
        let second = outgoing[0].clone();
        if second.id == first.id {
            return Err(format!(
                "cannot merge segment '{}': it is its own outgoing neighbour",
                first.id
            ));
        }

        let first_length = segment_length(self, &first, 24)?;
        let second_length = segment_length(self, &second, 24)?;
        if first_length + second_length <= 1e-6 {
            return Err(format!(
                "cannot merge segments '{}' and '{}': combined length is zero",
                first.id, second.id
            ));
        }

        let mut candidate = self.clone();
        for layout in &mut candidate.layouts {
            let label = format!("layout '{}' route", layout.id);
            if let Some(route) =
                merge_chain_ids(&layout.route, &first.id, &second.id, true, &label)?
            {
                layout.route = route;
            }
            rebase_merged_start_finish(
                &mut layout.start_finish,
                &first.id,
                &second.id,
                first_length,
                second_length,
            );
        }
        for path in &mut candidate.paths {
            let label = format!("path '{}' route", path.id);
            if let Some(route) = merge_chain_ids(&path.route, &first.id, &second.id, false, &label)?
            {
                path.route = route;
            }
        }
        for decoration in &mut candidate.decorations {
            let label = format!("decoration '{}' chain", decoration.id);
            if let Some(segments) =
                merge_chain_ids(&decoration.segments, &first.id, &second.id, false, &label)?
            {
                decoration.segments = segments;
            }
            if let Some(anchor) = &mut decoration.start {
                rebase_merged_anchor(anchor, &first.id, &second.id, first_length, second_length);
            }
            if let Some(anchor) = &mut decoration.end {
                rebase_merged_anchor(anchor, &first.id, &second.id, first_length, second_length);
            }
        }

        let merged = Segment {
            id: first.id.clone(),
            from: first.from.clone(),
            to: second.to.clone(),
            points: first
                .points
                .iter()
                .chain(second.points.iter())
                .copied()
                .collect(),
            from_tangent: first.from_tangent,
            to_tangent: second.to_tangent,
            start_profile: first.start_profile.clone(),
            end_profile: second.end_profile.clone(),
        };
        candidate.segments[index] = merged;
        let second_index = candidate
            .segments
            .iter()
            .position(|segment| segment.id == second.id)
            .expect("outgoing segment still present");
        candidate.segments.remove(second_index);
        candidate.junctions.retain(|junction| junction.id != shared);
        candidate.normalize_in_place()?;
        *self = candidate;
        Ok(())
    }
}

/// Removes `second_id` from a chain that references `first_id` and `second_id`
/// adjacently. `None` means the chain references neither segment.
fn merge_chain_ids(
    chain: &[String],
    first_id: &str,
    second_id: &str,
    closed: bool,
    label: &str,
) -> Result<Option<Vec<String>>, String> {
    let first_count = chain.iter().filter(|id| id.as_str() == first_id).count();
    let second_count = chain.iter().filter(|id| id.as_str() == second_id).count();
    if first_count > 1 || second_count > 1 {
        return Err(format!(
            "cannot merge segments '{first_id}' and '{second_id}': {label} references a segment more than once"
        ));
    }
    let first_index = chain.iter().position(|id| id == first_id);
    let second_index = chain.iter().position(|id| id == second_id);
    let (Some(first_index), Some(second_index)) = (first_index, second_index) else {
        if first_index.is_some() || second_index.is_some() {
            return Err(format!(
                "cannot merge segments '{first_id}' and '{second_id}': {label} references only one of them"
            ));
        }
        return Ok(None);
    };
    let adjacent = if closed {
        !chain.is_empty() && (first_index + 1) % chain.len() == second_index
    } else {
        first_index + 1 == second_index
    };
    if !adjacent {
        return Err(format!(
            "cannot merge segments '{first_id}' and '{second_id}': {label} does not reference them adjacently"
        ));
    }
    let mut merged = chain.to_vec();
    merged.remove(second_index);
    Ok(Some(merged))
}

fn rebase_merged_anchor(
    anchor: &mut DecorationAnchor,
    first_id: &str,
    second_id: &str,
    first_length: f32,
    second_length: f32,
) {
    let total = first_length + second_length;
    if total <= 1e-6 {
        return;
    }
    if anchor.segment_id == first_id {
        anchor.fraction = (anchor.fraction * first_length / total).clamp(0.0, 1.0);
    } else if anchor.segment_id == second_id {
        anchor.fraction =
            ((first_length + anchor.fraction * second_length) / total).clamp(0.0, 1.0);
        anchor.segment_id = first_id.to_string();
    }
}

fn rebase_merged_start_finish(
    start_finish: &mut StartFinish,
    first_id: &str,
    second_id: &str,
    first_length: f32,
    second_length: f32,
) {
    let total = first_length + second_length;
    if total <= 1e-6 {
        return;
    }
    if start_finish.segment_id == first_id {
        start_finish.fraction =
            (start_finish.fraction * first_length / total).clamp(0.0, 0.999_999);
    } else if start_finish.segment_id == second_id {
        start_finish.fraction =
            ((first_length + start_finish.fraction * second_length) / total).clamp(0.0, 0.999_999);
        start_finish.segment_id = first_id.to_string();
    }
}

#[allow(dead_code)]
fn unique_track_id<'a>(base: &str, existing: impl Iterator<Item = &'a str>) -> String {
    let existing = existing.collect::<BTreeSet<_>>();
    if !existing.contains(base) {
        return base.to_string();
    }
    for suffix in 1.. {
        let candidate = format!("{base}-{suffix}");
        if !existing.contains(candidate.as_str()) {
            return candidate;
        }
    }
    unreachable!("unbounded id search")
}

#[allow(dead_code)]
fn replace_chain_segment(route: &[String], old_id: &str, new_id: &str) -> Vec<String> {
    let mut updated = Vec::with_capacity(route.len() + 1);
    for segment_id in route {
        updated.push(segment_id.clone());
        if segment_id == old_id {
            updated.push(new_id.to_string());
        }
    }
    updated
}

#[allow(dead_code)]
fn rebase_decoration_anchor(
    anchor: &mut DecorationAnchor,
    old_id: &str,
    new_id: &str,
    split_fraction: f32,
) {
    if anchor.segment_id != old_id {
        return;
    }
    if anchor.fraction < split_fraction {
        anchor.fraction = (anchor.fraction / split_fraction).clamp(0.0, 1.0);
    } else {
        anchor.segment_id = new_id.to_string();
        anchor.fraction =
            ((anchor.fraction - split_fraction) / (1.0 - split_fraction)).clamp(0.0, 1.0);
    }
}

#[allow(dead_code)]
fn rebase_start_finish(
    start_finish: &mut StartFinish,
    old_id: &str,
    new_id: &str,
    split_fraction: f32,
) {
    if start_finish.segment_id != old_id {
        return;
    }
    if start_finish.fraction < split_fraction {
        start_finish.fraction = (start_finish.fraction / split_fraction).clamp(0.0, 1.0);
    } else {
        start_finish.segment_id = new_id.to_string();
        start_finish.fraction =
            ((start_finish.fraction - split_fraction) / (1.0 - split_fraction)).clamp(0.0, 1.0);
    }
}

fn validate_unique_ids<'a>(ids: impl Iterator<Item = &'a str>, label: &str) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for id in ids {
        if !seen.insert(id) {
            return Err(format!("duplicate {label} id '{id}'"));
        }
    }
    Ok(())
}

/// Per-segment coverage of one decoration path, expressed as fractions along
/// each segment's canonical direction.
fn decoration_coverage(
    decoration: &DecorationPath,
) -> Result<BTreeMap<String, (f32, f32)>, String> {
    let len = decoration.segments.len();
    let mut start_index = 0usize;
    let mut start_fraction = 0.0;
    if let Some(anchor) = &decoration.start {
        start_index = decoration
            .segments
            .iter()
            .position(|segment_id| segment_id == &anchor.segment_id)
            .ok_or_else(|| {
                format!(
                    "decoration '{}' start anchor references segment '{}' outside its chain",
                    decoration.id, anchor.segment_id
                )
            })?;
        start_fraction = anchor.fraction;
    }
    while start_fraction >= 1.0 - 1e-6 && start_index + 1 < len {
        start_index += 1;
        start_fraction = 0.0;
    }
    if start_index >= len || start_fraction >= 1.0 - 1e-6 {
        return Err(format!(
            "decoration '{}' has an empty start range",
            decoration.id
        ));
    }

    let (end_index, end_fraction) = match &decoration.end {
        Some(anchor) => {
            let index = decoration
                .segments
                .iter()
                .position(|segment_id| segment_id == &anchor.segment_id)
                .ok_or_else(|| {
                    format!(
                        "decoration '{}' end anchor references segment '{}' outside its chain",
                        decoration.id, anchor.segment_id
                    )
                })?;
            (index, anchor.fraction)
        }
        None => (len, 1.0),
    };
    if end_index < start_index
        || (end_index == start_index && end_fraction <= start_fraction + 1e-6)
    {
        return Err(format!(
            "decoration '{}' start anchor must precede its end anchor",
            decoration.id
        ));
    }

    let mut coverage = BTreeMap::new();
    if end_index >= len {
        for index in start_index..len {
            let start = if index == start_index {
                start_fraction
            } else {
                0.0
            };
            coverage.insert(decoration.segments[index].clone(), (start, 1.0));
        }
    } else if end_index == start_index {
        coverage.insert(
            decoration.segments[start_index].clone(),
            (start_fraction, end_fraction),
        );
    } else {
        coverage.insert(
            decoration.segments[start_index].clone(),
            (start_fraction, 1.0),
        );
        for index in start_index + 1..end_index {
            coverage.insert(decoration.segments[index].clone(), (0.0, 1.0));
        }
        if end_fraction > 1e-6 {
            coverage.insert(decoration.segments[end_index].clone(), (0.0, end_fraction));
        }
    }
    Ok(coverage)
}

fn decorations_share_lane(a: &DecorationPath, b: &DecorationPath) -> bool {
    a.kind == b.kind && (a.side < 0) == (b.side < 0) && a.lane == b.lane
}

fn validate_decoration_coverage(decorations: &[DecorationPath]) -> Result<(), String> {
    let mut coverages = Vec::with_capacity(decorations.len());
    for decoration in decorations {
        coverages.push(decoration_coverage(decoration)?);
    }
    for first in 0..decorations.len() {
        for second in first + 1..decorations.len() {
            if !decorations_share_lane(&decorations[first], &decorations[second]) {
                continue;
            }
            for (segment_id, (start, end)) in &coverages[first] {
                let Some((other_start, other_end)) = coverages[second].get(segment_id) else {
                    continue;
                };
                if *start < *other_end - 1e-5 && *other_start < *end - 1e-5 {
                    return Err(format!(
                        "decorations '{}' and '{}' overlap on segment '{}'",
                        decorations[first].id, decorations[second].id, segment_id
                    ));
                }
            }
        }
    }
    Ok(())
}

fn validate_profile(profile: Option<&Profile>, label: &str) -> Result<(), String> {
    let Some(profile) = profile else {
        return Ok(());
    };
    for (key, value) in &profile.values {
        if key.trim().is_empty() {
            return Err(format!("{label} profile has an empty key"));
        }
        if !value.is_finite() {
            return Err(format!("{label} profile value for '{key}' must be finite"));
        }
    }
    Ok(())
}

fn normalize_tangent(
    tangent: &mut Option<[f32; 2]>,
    segment_id: &str,
    label: &str,
) -> Result<(), String> {
    let Some(tangent) = tangent else {
        return Ok(());
    };
    if !tangent[0].is_finite() || !tangent[1].is_finite() {
        return Err(format!(
            "segment '{segment_id}' {label} tangent must be finite"
        ));
    }
    let length = tangent[0].hypot(tangent[1]);
    if length <= 1e-6 {
        return Err(format!(
            "segment '{segment_id}' {label} tangent must be non-zero"
        ));
    }
    tangent[0] /= length;
    tangent[1] /= length;
    Ok(())
}

pub fn control_points_with_heights(points: &[[f32; 3]]) -> Vec<[f32; 3]> {
    points
        .iter()
        .map(|point| [point[0], terrain_height(point[0], point[2]), point[2]])
        .collect()
}

fn segmented_ring(count: usize, segments: usize, gap: f32) -> Vec<(f32, f32)> {
    if count < 2 || segments == 0 {
        return Vec::new();
    }
    let length = count as f32 / segments as f32;
    (0..segments)
        .filter_map(|index| {
            let start = index as f32 * length + gap * 0.5;
            let end = (index + 1) as f32 * length - gap * 0.5;
            (end - start > 1e-4).then_some((start, end))
        })
        .collect()
}

fn default_prop_lines(points: &[Vec3]) -> Vec<PropLine> {
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
            offset: PROP_OFFSET,
            kind: PropKind::Blocks,
            spacing: None,
            phase: 0.0,
        });
        lines.push(PropLine {
            start,
            end,
            side: -outside_side,
            offset: PROP_OFFSET,
            kind: PropKind::Tyres,
            spacing: None,
            phase: 0.0,
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
                offset: PROP_FAR_OFFSET,
                kind: PropKind::Trees,
                spacing: None,
                phase: 0.0,
            });
        }
    }
    lines
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

fn junction_position(project: &TrackProject, id: &str) -> Result<Vec3, String> {
    let junction = project
        .junction_by_id(id)
        .ok_or_else(|| format!("unknown junction id '{id}'"))?;
    Ok(Vec3::new(junction.position[0], 0.0, junction.position[1]))
}

/// Finds the far junction beyond a degree-two neighbor so a segment can keep
/// Catmull-Rom continuity with the rest of the graph.
fn segment_neighbor_position(
    project: &TrackProject,
    segment: &Segment,
    junction_id: &str,
) -> Option<Vec3> {
    let mut neighbors = project.segments.iter().filter(|other| {
        other.id != segment.id && (other.from == junction_id || other.to == junction_id)
    });
    let other = neighbors.next()?;
    if neighbors.next().is_some() {
        return None;
    }
    let far = if other.from == junction_id {
        &other.to
    } else {
        &other.from
    };
    project
        .junction_by_id(far)
        .map(|junction| Vec3::new(junction.position[0], 0.0, junction.position[1]))
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

fn cubic_hermite(p0: Vec3, p1: Vec3, m0: Vec3, m1: Vec3, t: f32) -> Vec3 {
    let t2 = t * t;
    let t3 = t2 * t;
    let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
    let h10 = t3 - 2.0 * t2 + t;
    let h01 = -2.0 * t3 + 3.0 * t2;
    let h11 = t3 - t2;
    p0 * h00 + m0 * h10 + p1 * h01 + m1 * h11
}

/// Samples one canonical segment at `local` in `[0, 1)`. The result depends
/// only on the segment, its junction endpoints, and its graph neighbors.
fn sample_segment_at(
    project: &TrackProject,
    segment: &Segment,
    local: f32,
) -> Result<Vec3, String> {
    let from = junction_position(project, &segment.from)?;
    let to = junction_position(project, &segment.to)?;
    let mut controls = Vec::with_capacity(segment.points.len() + 2);
    controls.push(from);
    controls.extend(
        segment
            .points
            .iter()
            .map(|point| Vec3::new(point[0], 0.0, point[1])),
    );
    controls.push(to);
    let spans = controls.len() - 1;
    let local = local.clamp(0.0, 0.999_999);
    let explicit_from = tangent_vector(segment.from_tangent);
    let explicit_to = tangent_vector(segment.to_tangent);

    if spans == 1 && explicit_from.is_none() && explicit_to.is_none() {
        let p0 = segment_neighbor_position(project, segment, &segment.from)
            .unwrap_or_else(|| from + (from - to));
        let p3 = segment_neighbor_position(project, segment, &segment.to)
            .unwrap_or_else(|| to + (to - from));
        return Ok(catmull_rom(p0, from, to, p3, local));
    }

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

    let scaled = (local * spans as f32).min(spans as f32 - 1e-6);
    let span = (scaled.floor() as usize).min(spans - 1);
    let t = scaled - span as f32;
    let p0 = controls[span];
    let p1 = controls[span + 1];
    let chord = p0.distance(p1).max(1.0);
    Ok(cubic_hermite(
        p0,
        p1,
        tangents[span] * chord,
        tangents[span + 1] * chord,
        t,
    ))
}

fn layout_ordered_segments<'a>(
    project: &'a TrackProject,
    layout: &LayoutProject,
) -> Result<Vec<&'a Segment>, String> {
    let mut segments = Vec::with_capacity(layout.route.len());
    for (index, segment_id) in layout.route.iter().enumerate() {
        let segment = project.segment_by_id(segment_id).ok_or_else(|| {
            format!(
                "layout '{}' references unknown segment '{segment_id}'",
                layout.id
            )
        })?;
        let next_id = &layout.route[(index + 1) % layout.route.len()];
        let next = project.segment_by_id(next_id).ok_or_else(|| {
            format!(
                "layout '{}' references unknown segment '{next_id}'",
                layout.id
            )
        })?;
        if segment.to != next.from {
            return Err(format!(
                "layout '{}' route is not a closed canonical chain: segment '{}' ends at '{}' but next segment '{}' starts at '{}'",
                layout.id, segment.id, segment.to, next.id, next.from
            ));
        }
        segments.push(segment);
    }
    Ok(segments)
}

fn segment_length(project: &TrackProject, segment: &Segment, steps: usize) -> Result<f32, String> {
    let mut previous = sample_segment_at(project, segment, 0.0)?;
    let mut length = 0.0;
    for index in 1..=steps {
        let point = sample_segment_at(project, segment, index as f32 / steps as f32)?;
        length += previous.distance(point);
        previous = point;
    }
    Ok(length)
}

/// Builds the centerline for one layout from its canonical segments. The route
/// is sampled with the same station distribution as the original single-loop
/// builder so the default venue is geometrically identical to `main`.
fn sample_layout_loop(
    project: &TrackProject,
    layout: &LayoutProject,
) -> Result<(Vec<Vec3>, f32), String> {
    let segments = layout_ordered_segments(project, layout)?;
    let count = segments.len();
    let mut points = Vec::with_capacity(SEGMENTS);
    for index in 0..SEGMENTS {
        let global = index as f32 / SEGMENTS as f32 * count as f32;
        let ordinal = (global.floor() as usize) % count;
        let local = global - global.floor();
        points.push(sample_segment_at(project, segments[ordinal], local)?);
    }
    limit_centerline_curvature(&mut points);
    for point in &mut points {
        point.y = terrain_height(point.x, point.z);
    }

    let start_index = layout
        .route
        .iter()
        .position(|segment_id| segment_id == &layout.start_finish.segment_id)
        .ok_or_else(|| {
            format!(
                "layout '{}' start/finish segment '{}' is missing from its route",
                layout.id, layout.start_finish.segment_id
            )
        })?;
    let mut total_length = 0.0;
    let mut start_distance = 0.0;
    let mut start_segment_length = 0.0;
    for (index, segment) in segments.iter().enumerate() {
        let length = segment_length(project, segment, 24)?;
        if index < start_index {
            start_distance += length;
        }
        if index == start_index {
            start_segment_length = length;
        }
        total_length += length;
    }
    let start_finish = if total_length > 1e-4 {
        (start_distance + start_segment_length * layout.start_finish.fraction) / total_length
    } else {
        0.0
    }
    .clamp(0.0, 0.999_999);
    Ok((points, start_finish))
}

#[allow(dead_code)]
pub fn layout_centerline(
    project: &TrackProject,
    layout: &LayoutProject,
) -> Result<Vec<[f32; 3]>, String> {
    let (points, _) = sample_layout_loop(project, layout)?;
    Ok(points.iter().map(|point| point.to_array()).collect())
}

#[derive(Clone, Debug, PartialEq)]
struct KerbElementRun {
    start: f32,
    end: f32,
    side: i8,
    elements: Vec<f32>,
    phase: f32,
}

struct RouteArc {
    cumulative: Vec<f32>,
    total: f32,
}

impl RouteArc {
    fn new(points: &[Vec3]) -> Self {
        let mut cumulative = Vec::with_capacity(points.len() + 1);
        cumulative.push(0.0);
        let mut total = 0.0;
        for index in 0..points.len() {
            total += points[index].distance(points[(index + 1) % points.len()]);
            cumulative.push(total);
        }
        Self { cumulative, total }
    }

    fn distance_at_station(&self, station: f32) -> f32 {
        if self.cumulative.len() < 2 {
            return 0.0;
        }
        let limit = self.cumulative.len() as f32 - 1.0;
        let station = station.clamp(0.0, limit);
        let base = station.floor() as usize;
        if base + 1 >= self.cumulative.len() {
            return self.total;
        }
        let t = station - base as f32;
        self.cumulative[base] + (self.cumulative[base + 1] - self.cumulative[base]) * t
    }

    fn station_at_distance(&self, distance: f32) -> f32 {
        if self.cumulative.len() < 2 || self.total <= 1e-6 {
            return 0.0;
        }
        let distance = distance.clamp(0.0, self.total);
        let mut low = 0;
        let mut high = self.cumulative.len() - 1;
        while low + 1 < high {
            let mid = (low + high) / 2;
            if self.cumulative[mid] <= distance {
                low = mid;
            } else {
                high = mid;
            }
        }
        let length = self.cumulative[high] - self.cumulative[low];
        let t = if length > 1e-6 {
            (distance - self.cumulative[low]) / length
        } else {
            0.0
        };
        low as f32 + t
    }
}

#[derive(Default)]
struct BakedRouteDecorations {
    kerb_runs: Vec<KerbElementRun>,
    kerb_spans: Vec<KerbSpan>,
    prop_lines: Vec<PropLine>,
    has_kerbs: bool,
    has_props: bool,
}

fn decoration_prop_offset(lane: Option<&str>, kind: PropKind) -> f32 {
    let default = if kind == PropKind::Trees {
        PROP_FAR_OFFSET
    } else {
        PROP_OFFSET
    };
    lane.and_then(|lane| match lane {
        "near" => Some(PROP_OFFSET),
        "far" => Some(PROP_FAR_OFFSET),
        other => other
            .parse::<f32>()
            .ok()
            .filter(|offset| offset.is_finite() && *offset > 0.0)
            .map(|offset| offset.clamp(MIN_PROP_OFFSET, MAX_PROP_OFFSET)),
    })
    .unwrap_or(default)
}

/// Returns the route station range covered by a decoration path for one
/// layout. `None` means the layout does not use this decoration chain.
fn decoration_route_range(
    layout: &LayoutProject,
    decoration: &DecorationPath,
    route_scale: f32,
) -> Option<(f32, f32)> {
    let route = &layout.route;
    let route_len = route.len();
    if route_len == 0 || decoration.segments.is_empty() {
        return None;
    }
    let start_index = route
        .iter()
        .position(|segment_id| segment_id == &decoration.segments[0])?;
    for (offset, segment_id) in decoration.segments.iter().enumerate() {
        if route[(start_index + offset) % route_len] != *segment_id {
            return None;
        }
    }

    let (start_chain_index, start_fraction) = decoration
        .start
        .as_ref()
        .and_then(|anchor| {
            decoration
                .segments
                .iter()
                .position(|segment_id| segment_id == &anchor.segment_id)
                .map(|index| (index, anchor.fraction))
        })
        .unwrap_or((0, 0.0));
    let start_position = start_index as f32 + start_chain_index as f32 + start_fraction;

    let end_position = match &decoration.end {
        Some(anchor) => {
            let chain_index = decoration
                .segments
                .iter()
                .position(|segment_id| segment_id == &anchor.segment_id)?;
            start_index as f32 + chain_index as f32 + anchor.fraction
        }
        None => start_index as f32 + decoration.segments.len() as f32,
    };
    if end_position <= start_position + 1e-4 {
        return None;
    }
    if end_position - start_position > decoration.segments.len() as f32 + 1e-4 {
        return None;
    }
    Some((start_position * route_scale, end_position * route_scale))
}

fn push_kerb_run(
    decorations: &mut BakedRouteDecorations,
    arc: &RouteArc,
    start: f32,
    end: f32,
    side: i8,
    elements: &[f32],
    phase: f32,
) {
    let limit = arc.cumulative.len().saturating_sub(1) as f32;
    let mut start = start.clamp(0.0, limit);
    let mut end = end.clamp(0.0, limit);
    if end - start <= 1e-4 {
        return;
    }
    let mut phase = phase;
    while end > limit + 1e-4 {
        let first_length = arc.distance_at_station(limit) - arc.distance_at_station(start);
        if limit - start > 1e-4 {
            decorations.kerb_spans.push(KerbSpan {
                start,
                end: limit,
                side,
            });
            decorations.kerb_runs.push(KerbElementRun {
                start,
                end: limit,
                side,
                elements: elements.to_vec(),
                phase,
            });
        }
        start = 0.0;
        end -= limit;
        phase += first_length;
    }
    if end - start > 1e-4 {
        decorations.kerb_spans.push(KerbSpan { start, end, side });
        decorations.kerb_runs.push(KerbElementRun {
            start,
            end,
            side,
            elements: elements.to_vec(),
            phase,
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn push_prop_line(
    decorations: &mut BakedRouteDecorations,
    arc: &RouteArc,
    start: f32,
    end: f32,
    side: i8,
    offset: f32,
    kind: PropKind,
    spacing: Option<f32>,
    phase: f32,
) {
    let limit = arc.cumulative.len().saturating_sub(1) as f32;
    let mut start = start.clamp(0.0, limit);
    let mut end = end.clamp(0.0, limit);
    if end - start <= 1e-4 {
        return;
    }
    let mut phase = phase;
    while end > limit + 1e-4 {
        let first_length = arc.distance_at_station(limit) - arc.distance_at_station(start);
        if limit - start > 1e-4 {
            decorations.prop_lines.push(PropLine {
                start,
                end: limit,
                side,
                offset,
                kind,
                spacing,
                phase,
            });
        }
        start = 0.0;
        end -= limit;
        phase += first_length;
    }
    if end - start > 1e-4 {
        decorations.prop_lines.push(PropLine {
            start,
            end,
            side,
            offset,
            kind,
            spacing,
            phase,
        });
    }
}

/// Junction clearance in metres at a route position. Degree-two junctions are
/// continuous road; fork and pit junctions get a bake-time clearance so kerbs
/// and props do not poke into the junction mouth.
fn decoration_endpoint_clearance(
    project: &TrackProject,
    layout: &LayoutProject,
    route_position: f32,
) -> f32 {
    let route_len = layout.route.len();
    if route_len == 0 {
        return 0.0;
    }
    let rounded = route_position.round();
    if (route_position - rounded).abs() > 1e-4 {
        return 0.0;
    }
    let index = (rounded as i64).rem_euclid(route_len as i64) as usize;
    let Some(segment) = project.segment_by_id(&layout.route[index]) else {
        return 0.0;
    };
    let Some(junction) = project.junction_by_id(&segment.from) else {
        return 0.0;
    };
    match junction.kind {
        JunctionKind::Fork => 1.0,
        JunctionKind::PitEntry | JunctionKind::PitExit => 2.0,
        JunctionKind::Normal | JunctionKind::StartFinish => {
            let degree = project
                .segments
                .iter()
                .filter(|segment| segment.from == junction.id || segment.to == junction.id)
                .count();
            if degree > 2 {
                1.0
            } else {
                0.0
            }
        }
    }
}

/// Bakes canonical decoration paths into route-local slices for one layout.
/// Slices are derived at bake time and are never written back to the project.
fn bake_route_decorations(
    project: &TrackProject,
    layout: &LayoutProject,
    points: &[Vec3],
) -> Result<BakedRouteDecorations, String> {
    let arc = RouteArc::new(points);
    let route_scale = if layout.route.is_empty() {
        0.0
    } else {
        SEGMENTS as f32 / layout.route.len() as f32
    };
    let mut decorations = BakedRouteDecorations::default();
    for decoration in &project.decorations {
        let Some((start, end)) = decoration_route_range(layout, decoration, route_scale) else {
            continue;
        };
        let limit = points.len() as f32;
        let (start, end) = if route_scale > 1e-6 && end <= limit + 1e-4 {
            let start_clearance =
                decoration_endpoint_clearance(project, layout, start / route_scale);
            let end_clearance = decoration_endpoint_clearance(project, layout, end / route_scale);
            let start = if start_clearance > 0.0 {
                arc.station_at_distance(arc.distance_at_station(start) + start_clearance)
            } else {
                start
            };
            let end = if end_clearance > 0.0 {
                let guarded = arc.distance_at_station(end) - end_clearance;
                arc.station_at_distance(guarded.max(arc.distance_at_station(start)))
            } else {
                end
            };
            (start, end)
        } else {
            (start, end)
        };
        if end - start <= 1e-4 {
            continue;
        }
        let side = if decoration.side < 0 { -1 } else { 1 };
        match decoration.kind {
            DecorationKind::Kerb => {
                decorations.has_kerbs = true;
                let elements = if decoration.elements.is_empty() {
                    vec![project.settings.kerb.element_length.max(0.05)]
                } else {
                    decoration.elements.clone()
                };
                push_kerb_run(
                    &mut decorations,
                    &arc,
                    start,
                    end,
                    side,
                    &elements,
                    decoration.phase,
                );
            }
            DecorationKind::Prop => {
                decorations.has_props = true;
                let kind = decoration.prop_kind.unwrap_or(PropKind::Blocks);
                if kind == PropKind::Off {
                    continue;
                }
                let offset = decoration_prop_offset(decoration.lane.as_deref(), kind);
                let spacing = decoration
                    .spacing
                    .filter(|spacing| spacing.is_finite() && *spacing > 1e-4);
                push_prop_line(
                    &mut decorations,
                    &arc,
                    start,
                    end,
                    side,
                    offset,
                    kind,
                    spacing,
                    decoration.phase,
                );
            }
        }
    }
    decorations.kerb_spans = sanitize_kerb_spans(&decorations.kerb_spans, points.len());
    Ok(decorations)
}

fn append_kerb_element(
    points: &[Vec3],
    start: f32,
    end: f32,
    side: f32,
    buffers: &mut MeshBuffers,
) {
    if end - start <= 1e-4 {
        return;
    }
    let mut station = start;
    let mut previous_end = None;
    while station < end - 1e-4 {
        let next = (station.floor() + 1.0).min(end);
        let (point, binormal) = track_frame(points, station);
        let (next_point, next_binormal) = track_frame(points, next);
        let (start_edge, end_edge) =
            append_kerb(buffers, point, next_point, binormal, next_binormal, side);
        if let Some(previous_end) = previous_end {
            append_kerb_riser(buffers, previous_end, start_edge, next_point - point);
        }
        previous_end = Some(end_edge);
        station = next;
    }
}

fn append_decorated_kerbs(
    points: &[Vec3],
    runs: &[KerbElementRun],
    kerb_red: &mut MeshBuffers,
    kerb_white: &mut MeshBuffers,
) {
    let arc = RouteArc::new(points);
    for run in runs {
        let start = run.start.clamp(0.0, points.len() as f32);
        let end = run.end.clamp(0.0, points.len() as f32);
        if end - start <= 1e-4 || run.elements.is_empty() {
            continue;
        }
        let cycle_total = run.elements.iter().sum::<f32>();
        if cycle_total <= 1e-4 {
            continue;
        }
        let mut cursor = arc.distance_at_station(start);
        let end_distance = arc.distance_at_station(end);
        let mut phase = run.phase.rem_euclid(cycle_total);
        let mut element_index = 0;
        let mut length = run.elements[0];
        while phase >= length - 1e-4 {
            phase -= length;
            element_index += 1;
            length = run.elements[element_index % run.elements.len()];
        }
        let mut remaining = (length - phase).max(0.0);
        while cursor < end_distance - 1e-4 {
            if remaining <= 1e-4 {
                element_index += 1;
                remaining = run.elements[element_index % run.elements.len()];
                continue;
            }
            let element_length = remaining.min(end_distance - cursor);
            let next = cursor + element_length;
            let element_start = arc.station_at_distance(cursor);
            let element_end = arc.station_at_distance(next);
            let buffers = if element_index % 2 == 0 {
                &mut *kerb_red
            } else {
                &mut *kerb_white
            };
            append_kerb_element(points, element_start, element_end, run.side as f32, buffers);
            cursor = next;
            element_index += 1;
            remaining = run.elements[element_index % run.elements.len()];
        }
    }
}

pub fn track_from_project_with_default(
    project: &TrackProject,
    default_layout: Option<&str>,
) -> Result<EditorTrack, String> {
    let project = project.clone().normalize()?;
    let index = default_layout
        .and_then(|id| project.layout_index(id))
        .unwrap_or(0);
    let layout = project
        .layouts
        .get(index)
        .ok_or_else(|| "track project needs at least one layout".to_string())?;
    let (points, start_finish) = sample_layout_loop(&project, layout)?;
    let baked = bake_route_decorations(&project, layout, &points)?;
    let kerb_spans = if baked.has_kerbs {
        baked.kerb_spans
    } else {
        sanitize_kerb_spans(&default_kerb_spans(), points.len())
    };
    let kerb_runs = if baked.has_kerbs {
        Some(baked.kerb_runs)
    } else {
        None
    };
    let prop_lines = if baked.has_props {
        baked.prop_lines
    } else {
        default_prop_lines(&points)
    };
    build_editor_track(
        points,
        kerb_spans,
        prop_lines,
        TrackBuildMetadata {
            name: layout.name.clone(),
            manifest_id: layout.id.clone(),
            start_finish,
            sectors: layout.sectors.clone(),
            cameras: layout.cameras.clone(),
            spawns: layout.spawns.clone(),
            kerb_runs,
        },
    )
}

#[allow(dead_code)]
pub fn pack_from_project(project: &TrackProject) -> Result<Vec<u8>, String> {
    pack_from_project_with_default(project, None)
}

#[allow(dead_code)]
pub fn pack_from_project_with_default(
    project: &TrackProject,
    default_layout: Option<&str>,
) -> Result<Vec<u8>, String> {
    let track = track_from_project_with_default(project, default_layout)?;
    pack_from_track(&track)
}

fn spawn_at_fraction(points: &[Vec3], fraction: f32) -> SpawnPoint {
    let count = points.len();
    let fraction = if fraction.is_finite() {
        fraction.clamp(0.0, 0.999_999)
    } else {
        0.0
    };
    let (position, tangent) = if fraction <= 0.0 {
        (points[0], points[1] - points[0])
    } else {
        let total = points
            .iter()
            .enumerate()
            .map(|(index, point)| point.distance(points[(index + 1) % count]))
            .sum::<f32>();
        let target = total * fraction;
        let mut travelled = 0.0;
        let mut found = (points[0], points[1] - points[0]);
        for index in 0..count {
            let next = points[(index + 1) % count];
            let length = points[index].distance(next);
            if length > 1e-6 && travelled + length >= target {
                let t = ((target - travelled) / length).clamp(0.0, 1.0);
                found = (points[index].lerp(next, t), next - points[index]);
                break;
            }
            travelled += length;
        }
        found
    };
    let tangent = tangent.normalize_or_zero();
    let rotation = if tangent.length_squared() > 1e-6 {
        Quat::from_rotation_y(PI) * Quat::from_rotation_arc(Vec3::NEG_Z, tangent)
    } else {
        Quat::IDENTITY
    };
    SpawnPoint {
        name: "spawn".to_string(),
        position: position.to_array(),
        rotation_xyzw: rotation.to_array(),
    }
}

struct TrackBuildMetadata {
    name: String,
    manifest_id: String,
    start_finish: f32,
    sectors: Vec<SectorSpec>,
    cameras: Vec<CameraSpec>,
    spawns: Vec<SpawnPoint>,
    kerb_runs: Option<Vec<KerbElementRun>>,
}

fn build_editor_track(
    points: Vec<Vec3>,
    kerb_spans: Vec<KerbSpan>,
    prop_lines: Vec<PropLine>,
    metadata: TrackBuildMetadata,
) -> Result<EditorTrack, String> {
    if points.len() < 3 {
        return Err("procedural track needs at least three samples".to_string());
    }
    let TrackBuildMetadata {
        name,
        manifest_id,
        start_finish,
        sectors,
        cameras,
        spawns,
        kerb_runs,
    } = metadata;

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
    let TrackRibbon {
        buffers: track_buffers,
        surface_ids: track_surface_ids,
    } = track_ribbon(&points);
    append_collision_surfaces(&mut collision, &track_buffers, |index| {
        track_surface_ids
            .get(index)
            .copied()
            .unwrap_or(TRACK_SURFACE)
    });
    let terrain = terrain_grid(&points);
    let mountains = mountain_grid();
    push_mesh(&mut visual, "asphalt_dither", 1, track_buffers, false);
    let racing_line = racing_line_ribbon(&points, &racing_line_offsets(&points));
    push_mesh(
        &mut visual,
        "racing_line",
        RACING_LINE_MATERIAL,
        racing_line,
        false,
    );
    push_mesh(&mut visual, "terrain", 2, terrain, false);
    push_mesh(&mut visual, "mountains", 8, mountains, false);

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

    let count = points.len();
    if let Some(runs) = &kerb_runs {
        append_decorated_kerbs(&points, runs, &mut kerb_red, &mut kerb_white);
    } else {
        for span in &kerb_spans {
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
                let (point, binormal) = track_frame(&points, segment_start);
                let (next, next_binormal) = track_frame(&points, segment_end);
                let kerb = if index % 2 == 0 {
                    &mut kerb_red
                } else {
                    &mut kerb_white
                };
                let (start_edge, end_edge) =
                    append_kerb(kerb, point, next, binormal, next_binormal, span.side as f32);
                if let Some(previous_end) = previous_end {
                    append_kerb_riser(kerb, previous_end, start_edge, next - point);
                }
                previous_end = Some(end_edge);
            }
        }
    }

    append_prop_lines(
        &points,
        &prop_lines,
        &mut barriers,
        &mut tires_blue,
        &mut tires_white,
        &mut tree_trunks_high,
        &mut tree_foliage_high,
        &mut tree_trunks_low,
        &mut tree_foliage_low,
        &mut tree_cards,
    );
    append_prop_walls(&points, &prop_lines, &mut prop_walls);

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

    let spawns = if spawns.is_empty() {
        vec![spawn_at_fraction(&points, start_finish)]
    } else {
        spawns
    };
    let camera_y = terrain_height(0.0, 0.0);
    let cameras = if cameras.is_empty() {
        vec![CameraSpec {
            name: "overview".to_string(),
            position: [0.0, camera_y + 45.0, 65.0],
            target: [0.0, camera_y, 0.0],
            fov_y: 45.0_f32.to_radians(),
        }]
    } else {
        cameras
    };
    let sectors = if sectors.is_empty() {
        vec![SectorSpec {
            name: "loop".to_string(),
            start_fraction: 0.0,
            end_fraction: 1.0,
        }]
    } else {
        sectors
    };
    let length = points
        .iter()
        .enumerate()
        .map(|(index, point)| point.distance(points[(index + 1) % points.len()]))
        .sum::<f32>();
    let metadata = TrackMetadata {
        name,
        length,
        spawns,
        pit_boxes: Vec::new(),
        cameras,
        sectors,
        surfaces: vec![
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
        ],
    };

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
        kerb_spans,
        prop_lines,
    })
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

fn limit_centerline_curvature(points: &mut [Vec3]) {
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
            too_tight |= radii[index] < MIN_TURN_RADIUS;
        }
        if !too_tight {
            return;
        }

        previous.copy_from_slice(points);
        for index in 0..count {
            if radii[index] >= MIN_TURN_RADIUS {
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

struct TrackRibbon {
    buffers: MeshBuffers,
    surface_ids: Vec<u16>,
}

fn track_ribbon(points: &[Vec3]) -> TrackRibbon {
    let mut buffers = empty_buffers();
    let mut surface_ids = Vec::new();
    let count = points.len();
    let radii = [
        -OUTER_EDGE_RADIUS,
        -ASPHALT_HALF_WIDTH,
        ASPHALT_HALF_WIDTH,
        OUTER_EDGE_RADIUS,
    ];
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

fn racing_line_offsets(points: &[Vec3]) -> Vec<f32> {
    let count = points.len();
    if count < 3 {
        return vec![0.0; count];
    }

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
        target[index] = RACING_LINE_MAX_OFFSET * strength * curvature.signum();
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
    for offset in &mut offsets {
        *offset = offset.clamp(-RACING_LINE_MAX_OFFSET, RACING_LINE_MAX_OFFSET);
    }
    offsets
}

fn racing_line_ribbon(points: &[Vec3], offsets: &[f32]) -> MeshBuffers {
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
        let left = point - binormal * ASPHALT_HALF_WIDTH;
        let right = point + binormal * ASPHALT_HALF_WIDTH;
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
            // The asphalt band is a flat quad between its -6 m and +6 m edge
            // vertices. Interpolate those edge heights so the line follows the
            // rendered surface instead of the underlying terrain curve.
            let t = ((lateral + ASPHALT_HALF_WIDTH) / (2.0 * ASPHALT_HALF_WIDTH)).clamp(0.0, 1.0);
            let y = left_y + (right_y - left_y) * t + RACING_LINE_LIFT;
            buffers.positions.push([position.x, y, position.z]);
            buffers.normals.push(terrain_normal(position.x, position.z));
            let v = ((lateral + OUTER_EDGE_RADIUS) / (2.0 * OUTER_EDGE_RADIUS)) * TARMAC_UV_ACROSS;
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

fn terrain_grid(points: &[Vec3]) -> MeshBuffers {
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
    let hole_radius_squared = 9.5_f32 * 9.5;
    for iz in 0..TERRAIN_CELLS {
        for ix in 0..TERRAIN_CELLS {
            let cell_x = -half + (ix as f32 + 0.5) * step;
            let cell_z = -half + (iz as f32 + 0.5) * step;
            let under_track = points.iter().any(|point| {
                let dx = point.x - cell_x;
                let dz = point.z - cell_z;
                dx * dx + dz * dz < hole_radius_squared
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
    let default_spacing = match line.kind {
        PropKind::Blocks => BLOCK_SPACING,
        PropKind::Tyres => TYRE_SPACING,
        PropKind::Trees => TREE_SPACING,
        PropKind::Off => return Vec::new(),
    };
    let spacing = line
        .spacing
        .filter(|spacing| spacing.is_finite() && *spacing > 1e-4)
        .unwrap_or(default_spacing);
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
    let mut distance = if line.phase.is_finite() {
        line.phase.rem_euclid(spacing)
    } else {
        0.0
    };
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

fn append_kerb(
    buffers: &mut MeshBuffers,
    start: Vec3,
    end: Vec3,
    start_binormal: Vec3,
    end_binormal: Vec3,
    side: f32,
) -> ([Vec3; 2], [Vec3; 2]) {
    let inner_start = start + start_binormal * (KERB_INNER_RADIUS * side);
    let inner_end = end + end_binormal * (KERB_INNER_RADIUS * side);
    let outer_start = start + start_binormal * (KERB_OUTER_RADIUS * side);
    let outer_end = end + end_binormal * (KERB_OUTER_RADIUS * side);
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

    fn project_centerline(project: &TrackProject) -> Vec<Vec3> {
        let layout = project.layouts.first().expect("layout");
        layout_centerline(project, layout)
            .expect("centerline")
            .into_iter()
            .map(Vec3::from_array)
            .collect()
    }

    fn default_centerline() -> Vec<Vec3> {
        project_centerline(&TrackProject::default_project())
    }

    fn ring_project_centerline(control_points: &[[f32; 3]]) -> Vec<Vec3> {
        let points = control_points
            .iter()
            .map(|point| [point[0], point[2]])
            .collect::<Vec<_>>();
        project_centerline(&TrackProject::ring_project("test", "test", &points))
    }

    fn default_editor_track() -> EditorTrack {
        track_from_project_with_default(&TrackProject::default_project(), None)
            .expect("default editor track")
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
    fn default_project_round_trips_through_json() {
        let project = TrackProject::default_project();
        let json = serde_json::to_string(&project).expect("serialize project");
        let parsed = TrackProject::from_json(&json).expect("parse project");
        let expected = project.clone().normalize().expect("normalize project");
        assert_eq!(parsed, expected);
        assert_eq!(expected.schema, TRACK_PROJECT_SCHEMA);
        assert_eq!(expected.junctions.len(), CONTROL_POINTS.len());
        assert_eq!(expected.segments.len(), CONTROL_POINTS.len());
        assert_eq!(expected.layouts.len(), 1);
        assert_eq!(expected.layouts[0].route.len(), CONTROL_POINTS.len());
        assert!(expected.paths.is_empty());
        assert!(expected.decorations.is_empty());
    }

    #[test]
    fn graph_centerline_is_deterministic() {
        let project = TrackProject::default_project();
        let layout = &project.layouts[0];
        let first = layout_centerline(&project, layout).expect("first");
        let second = layout_centerline(&project, layout).expect("second");
        assert_eq!(first, second);
    }

    #[test]
    fn graph_validation_rejects_dangling_and_open_routes() {
        let mut dangling = TrackProject::default_project();
        dangling.segments[0].to = "missing".to_string();
        assert!(dangling.normalize().is_err());

        let mut open = TrackProject::default_project();
        open.layouts[0].route.pop();
        assert!(open.normalize().is_err());

        let mut bad_start = TrackProject::default_project();
        bad_start.layouts[0].start_finish.fraction = 1.0;
        assert!(bad_start.normalize().is_err());

        let mut duplicate = TrackProject::default_project();
        duplicate.junctions[1].id = duplicate.junctions[0].id.clone();
        assert!(duplicate.normalize().is_err());
    }

    #[test]
    fn graph_metadata_and_start_finish_come_from_the_layout() {
        let mut project = TrackProject::default_project();
        project.layouts[0].name = "Venue Loop".to_string();
        project.layouts[0].start_finish.segment_id = "segment-3".to_string();
        project.layouts[0].start_finish.fraction = 0.5;
        let track = track_from_project_with_default(&project, None).expect("graph track");
        assert_eq!(track.metadata.name, "Venue Loop");
        assert_eq!(track.manifest.id, "procedural-drift-loop");
        let expected = layout_centerline(&project, &project.layouts[0]).expect("centerline");
        let spawn = &track.metadata.spawns[0];
        assert!(expected.iter().any(|point| Vec3::from_array(*point)
            .distance(Vec3::from_array(spawn.position))
            < 1.0));
    }

    fn kerb_decoration(
        id: &str,
        segments: &[&str],
        start: Option<(&str, f32)>,
        end: Option<(&str, f32)>,
        side: i8,
        elements: Vec<f32>,
    ) -> DecorationPath {
        DecorationPath {
            id: id.to_string(),
            kind: DecorationKind::Kerb,
            side,
            lane: None,
            segments: segments.iter().map(|segment| segment.to_string()).collect(),
            start: start.map(|(segment_id, fraction)| DecorationAnchor {
                segment_id: segment_id.to_string(),
                fraction,
            }),
            end: end.map(|(segment_id, fraction)| DecorationAnchor {
                segment_id: segment_id.to_string(),
                fraction,
            }),
            elements,
            phase: 0.0,
            prop_kind: None,
            spacing: None,
        }
    }

    fn prop_decoration(
        id: &str,
        segments: &[&str],
        side: i8,
        lane: &str,
        kind: PropKind,
        spacing: f32,
        phase: f32,
    ) -> DecorationPath {
        DecorationPath {
            id: id.to_string(),
            kind: DecorationKind::Prop,
            side,
            lane: Some(lane.to_string()),
            segments: segments.iter().map(|segment| segment.to_string()).collect(),
            start: None,
            end: None,
            elements: Vec::new(),
            phase,
            prop_kind: Some(kind),
            spacing: Some(spacing),
        }
    }

    #[test]
    fn decoration_paths_bake_across_segment_boundaries_without_duplication() {
        let mut project = TrackProject::default_project();
        let mut decoration = kerb_decoration(
            "kerb-a",
            &["segment-1", "segment-2"],
            Some(("segment-1", 0.25)),
            Some(("segment-2", 0.75)),
            1,
            vec![1.5, 2.5],
        );
        decoration.phase = 0.4;
        project.decorations.push(decoration);
        let project = project.normalize().expect("normalize project");
        let (points, _) = sample_layout_loop(&project, &project.layouts[0]).expect("points");
        let baked = bake_route_decorations(&project, &project.layouts[0], &points)
            .expect("bake decorations");
        assert!(baked.has_kerbs);
        assert_eq!(baked.kerb_runs.len(), 1);
        assert_eq!(baked.kerb_spans.len(), 1);
        let run = &baked.kerb_runs[0];
        let segment_span = SEGMENTS as f32 / CONTROL_POINTS.len() as f32;
        assert!(
            run.end - run.start > segment_span * 1.4,
            "expected the run to cross the segment boundary"
        );

        let track = track_from_project_with_default(&project, None).expect("track");
        let mesh = |name: &str| {
            track
                .visual
                .meshes
                .iter()
                .position(|mesh| mesh.name == name)
                .and_then(|index| track.visual.mesh_buffers.get(index))
                .unwrap_or_else(|| panic!("missing {name} mesh"))
        };
        assert!(!mesh("kerb_red").positions.is_empty());
        assert!(!mesh("kerb_white").positions.is_empty());
        let again = track_from_project_with_default(&project, None).expect("deterministic track");
        assert_eq!(track.visual, again.visual);
        assert_eq!(track.collision, again.collision);
    }

    #[test]
    fn decoration_props_keep_spacing_and_phase_across_segments() {
        let mut project = TrackProject::default_project();
        project.decorations.push(prop_decoration(
            "props-a",
            &["segment-0", "segment-1"],
            1,
            "far",
            PropKind::Trees,
            6.0,
            2.0,
        ));
        let project = project.normalize().expect("normalize project");
        let (points, _) = sample_layout_loop(&project, &project.layouts[0]).expect("points");
        let baked = bake_route_decorations(&project, &project.layouts[0], &points)
            .expect("bake decorations");
        assert!(baked.has_props);
        assert_eq!(baked.prop_lines.len(), 1);
        let line = &baked.prop_lines[0];
        assert_eq!(line.spacing, Some(6.0));
        assert!((line.phase - 2.0).abs() < 1e-5);
        let placements = prop_placements(&points, line);
        assert!(placements.len() > 2);
        for pair in placements.windows(2) {
            let distance = pair[0].0.distance(pair[1].0);
            assert!(
                (distance - 6.0).abs() < 0.2,
                "expected continuous 6 m prop spacing, got {distance}"
            );
        }

        let track = track_from_project_with_default(&project, None).expect("track");
        assert!(track
            .visual
            .meshes
            .iter()
            .any(|mesh| mesh.name == "tree_trunks_high"));
        let again = track_from_project_with_default(&project, None).expect("deterministic track");
        assert_eq!(track.visual, again.visual);
    }

    #[test]
    fn overlapping_decorations_are_rejected_only_on_the_same_lane() {
        let mut project = TrackProject::default_project();
        project.decorations.push(kerb_decoration(
            "kerb-a",
            &["segment-0"],
            None,
            None,
            1,
            Vec::new(),
        ));
        project.decorations.push(kerb_decoration(
            "kerb-b",
            &["segment-0"],
            None,
            None,
            1,
            Vec::new(),
        ));
        assert!(project.clone().normalize().is_err());

        project.decorations[1].side = -1;
        assert!(project.normalize().is_ok());
    }

    #[test]
    fn decoration_anchors_must_run_in_chain_order() {
        let mut project = TrackProject::default_project();
        project.decorations.push(kerb_decoration(
            "kerb-bad",
            &["segment-0", "segment-1"],
            Some(("segment-1", 0.5)),
            Some(("segment-0", 0.5)),
            1,
            Vec::new(),
        ));
        assert!(project.normalize().is_err());
    }

    #[test]
    fn fork_and_pit_junctions_clear_decoration_endpoints_at_bake_time() {
        let mut project = TrackProject::default_project();
        assert_eq!(
            decoration_endpoint_clearance(&project, &project.layouts[0], 0.0),
            0.0
        );
        project.junctions[0].kind = JunctionKind::Fork;
        assert_eq!(
            decoration_endpoint_clearance(&project, &project.layouts[0], 0.0),
            1.0
        );
        project.junctions[0].kind = JunctionKind::PitExit;
        assert_eq!(
            decoration_endpoint_clearance(&project, &project.layouts[0], 0.0),
            2.0
        );
        assert_eq!(
            decoration_endpoint_clearance(&project, &project.layouts[0], 0.5),
            0.0
        );
    }

    #[test]
    fn splitting_a_segment_rebases_decoration_chains_and_anchors() {
        let mut project = TrackProject::default_project();
        project.decorations.push(kerb_decoration(
            "kerb-a",
            &["segment-1", "segment-2"],
            Some(("segment-1", 0.25)),
            Some(("segment-2", 0.75)),
            1,
            vec![2.0],
        ));
        project = project.normalize().expect("normalize project");
        let new_segment = project.split_segment("segment-1", 0.5).expect("split");
        assert_eq!(
            project.decorations[0].segments,
            ["segment-1", new_segment.as_str(), "segment-2"]
        );
        let start = project.decorations[0].start.as_ref().expect("start");
        assert_eq!(start.segment_id, "segment-1");
        assert!((start.fraction - 0.5).abs() < 1e-5);
        let end = project.decorations[0].end.as_ref().expect("end");
        assert_eq!(end.segment_id, "segment-2");
        assert!((end.fraction - 0.75).abs() < 1e-5);

        track_from_project_with_default(&project, None).expect("track after split");

        let second = project
            .split_segment("segment-2", 0.5)
            .expect("second split");
        let end = project.decorations[0].end.as_ref().expect("end");
        assert_eq!(end.segment_id, second);
        assert!((end.fraction - 0.5).abs() < 1e-5);
        track_from_project_with_default(&project, None).expect("track after second split");
    }

    #[test]
    fn moving_a_junction_reparameterises_decorations_without_rewriting_paths() {
        let mut project = TrackProject::default_project();
        project.decorations.push(kerb_decoration(
            "kerb-a",
            &["segment-0"],
            None,
            None,
            1,
            vec![2.0],
        ));
        let before = track_from_project_with_default(&project, None).expect("before move");
        let path_before = project.decorations[0].clone();
        project
            .move_junction("junction-1", 48.0, -42.0)
            .expect("move");
        let after = track_from_project_with_default(&project, None).expect("after move");
        assert_ne!(before.visual, after.visual);
        assert_eq!(project.decorations[0], path_before);
    }

    #[test]
    fn deleting_a_decorated_segment_is_blocked() {
        let mut project = TrackProject::default_project();
        project.decorations.push(kerb_decoration(
            "kerb-a",
            &["segment-1"],
            None,
            None,
            1,
            Vec::new(),
        ));
        assert!(project.delete_segment("segment-1").is_err());

        let mut spare = TrackProject::default_project();
        spare.junctions.push(Junction {
            id: "junction-spare-a".to_string(),
            position: [80.0, 80.0],
            kind: JunctionKind::Normal,
            profile: None,
        });
        spare.junctions.push(Junction {
            id: "junction-spare-b".to_string(),
            position: [90.0, 90.0],
            kind: JunctionKind::Normal,
            profile: None,
        });
        spare.segments.push(Segment {
            id: "segment-spare".to_string(),
            from: "junction-spare-a".to_string(),
            to: "junction-spare-b".to_string(),
            points: Vec::new(),
            from_tangent: None,
            to_tangent: None,
            start_profile: None,
            end_profile: None,
        });
        spare = spare.normalize().expect("spare project");
        spare.delete_segment("segment-spare").expect("delete spare");
        assert!(!spare
            .segments
            .iter()
            .any(|segment| segment.id == "segment-spare"));
    }

    #[test]
    fn project_with_decorations_round_trips_through_json() {
        let mut project = TrackProject::default_project();
        project.decorations.push(kerb_decoration(
            "kerb-a",
            &["segment-1", "segment-2"],
            Some(("segment-1", 0.25)),
            Some(("segment-2", 0.75)),
            1,
            vec![1.5, 2.5],
        ));
        project.decorations.push(prop_decoration(
            "props-a",
            &["segment-3"],
            1,
            "far",
            PropKind::Trees,
            6.0,
            2.0,
        ));
        project.paths.push(PathProject {
            id: "pit-path".to_string(),
            kind: "pit".to_string(),
            route: vec!["segment-4".to_string(), "segment-5".to_string()],
            boxes: Vec::new(),
        });

        let json = project.to_json().expect("serialize project");
        let parsed = TrackProject::from_json(&json).expect("parse project");
        assert_eq!(parsed, project.clone().normalize().expect("normalize"));
        assert_eq!(parsed.decorations.len(), 2);
        assert_eq!(parsed.paths.len(), 1);
        assert_eq!(parsed.decorations[0].segments, ["segment-1", "segment-2"]);
    }

    #[test]
    fn split_then_merge_restores_route_closure_and_decoration_continuity() {
        let mut project = TrackProject::default_project();
        project.decorations.push(kerb_decoration(
            "kerb-a",
            &["segment-1", "segment-2"],
            Some(("segment-1", 0.25)),
            Some(("segment-2", 0.75)),
            1,
            vec![2.0],
        ));
        project = project.normalize().expect("normalize project");
        let original_route = project.layouts[0].route.clone();
        let original_segments = project.segments.len();

        let new_segment = project.split_segment("segment-1", 0.5).expect("split");
        assert_eq!(
            project.decorations[0].segments,
            ["segment-1", new_segment.as_str(), "segment-2"]
        );
        assert_eq!(project.layouts[0].route.len(), original_route.len() + 1);

        project.merge_segment("segment-1").expect("merge");
        assert_eq!(project.layouts[0].route, original_route);
        assert_eq!(project.segments.len(), original_segments);
        assert_eq!(project.decorations[0].segments, ["segment-1", "segment-2"]);
        let start = project.decorations[0].start.as_ref().expect("start");
        assert_eq!(start.segment_id, "segment-1");
        let end = project.decorations[0].end.as_ref().expect("end");
        assert_eq!(end.segment_id, "segment-2");
        track_from_project_with_default(&project, None).expect("track after merge");
    }

    #[test]
    fn interior_segment_points_can_be_inserted_moved_and_deleted() {
        let mut project = TrackProject::default_project();
        let before = project.segment_centerline("segment-0").expect("centerline");

        let index = project
            .insert_segment_point("segment-0", 0, 10.0, -10.0)
            .expect("insert");
        assert_eq!(index, 0);
        assert_eq!(
            project.segment_by_id("segment-0").expect("segment").points,
            vec![[10.0, -10.0]]
        );

        project
            .move_segment_point("segment-0", 0, 12.0, -8.0)
            .expect("move");
        assert_eq!(
            project.segment_by_id("segment-0").expect("segment").points,
            vec![[12.0, -8.0]]
        );
        assert!(project
            .move_segment_point("segment-0", 1, 0.0, 0.0)
            .is_err());

        project
            .delete_segment_point("segment-0", 0)
            .expect("delete");
        assert!(project
            .segment_by_id("segment-0")
            .expect("segment")
            .points
            .is_empty());
        assert!(project.delete_segment_point("segment-0", 0).is_err());
        let after = project.segment_centerline("segment-0").expect("centerline");
        assert_eq!(before, after);
    }

    #[test]
    fn segment_tangents_can_be_set_and_cleared() {
        let mut project = TrackProject::default_project();
        project
            .set_segment_tangents("segment-0", Some([1.0, 0.0]), Some([0.0, 2.0]))
            .expect("set tangents");
        let segment = project.segment_by_id("segment-0").expect("segment");
        assert_eq!(segment.from_tangent, Some([1.0, 0.0]));
        assert_eq!(segment.to_tangent, Some([0.0, 1.0]));
        track_from_project_with_default(&project, None).expect("track after tangents");

        project
            .set_segment_tangents("segment-0", None, None)
            .expect("clear tangents");
        let segment = project.segment_by_id("segment-0").expect("segment");
        assert_eq!(segment.from_tangent, None);
        assert_eq!(segment.to_tangent, None);

        let before = project
            .segment_by_id("segment-0")
            .cloned()
            .expect("segment");
        assert!(project
            .set_segment_tangents("segment-0", Some([0.0, 0.0]), None)
            .is_err());
        assert_eq!(
            project
                .segment_by_id("segment-0")
                .cloned()
                .expect("segment"),
            before
        );
    }

    #[test]
    fn merge_rejects_ambiguous_junctions_and_one_sided_decorations() {
        let mut ambiguous = TrackProject::default_project();
        ambiguous.junctions.push(Junction {
            id: "junction-spur".to_string(),
            position: [70.0, 70.0],
            kind: JunctionKind::Normal,
            profile: None,
        });
        ambiguous.segments.push(Segment {
            id: "segment-spur".to_string(),
            from: ambiguous.segments[0].to.clone(),
            to: "junction-spur".to_string(),
            points: Vec::new(),
            from_tangent: None,
            to_tangent: None,
            start_profile: None,
            end_profile: None,
        });
        let mut ambiguous = ambiguous.normalize().expect("ambiguous project");
        assert!(ambiguous.merge_segment("segment-0").is_err());
        assert!(ambiguous.segment_by_id("segment-0").is_some());

        let mut one_sided = TrackProject::default_project();
        one_sided.decorations.push(kerb_decoration(
            "kerb-a",
            &["segment-0"],
            None,
            None,
            1,
            Vec::new(),
        ));
        let mut one_sided = one_sided.normalize().expect("one-sided project");
        assert!(one_sided.merge_segment("segment-0").is_err());
        assert_eq!(one_sided.decorations[0].segments, ["segment-0"]);
        assert!(one_sided.segment_by_id("segment-0").is_some());
    }

    #[test]
    fn bake_is_deterministic_after_segment_edits() {
        let mut project = TrackProject::default_project();
        project
            .insert_segment_point("segment-2", 0, 4.0, -6.0)
            .expect("insert");
        project
            .move_junction("junction-3", 42.0, 4.0)
            .expect("move junction");
        project
            .set_segment_tangents("segment-2", Some([1.0, 0.0]), None)
            .expect("tangents");
        project = project.normalize().expect("normalize");

        let first = track_from_project_with_default(&project, None).expect("first bake");
        let second = track_from_project_with_default(&project, None).expect("second bake");
        assert_eq!(first.visual, second.visual);
        assert_eq!(first.collision, second.collision);
        assert_eq!(first.metadata, second.metadata);
        assert_eq!(first.manifest, second.manifest);

        let centerline = project
            .segment_centerline("segment-2")
            .expect("segment centerline");
        assert_eq!(centerline.len(), SEGMENT_CENTERLINE_STEPS + 1);
    }

    #[test]
    fn spawn_faces_increasing_track_direction_after_drive_flip() {
        let track = default_editor_track();
        let spawn = &track.metadata.spawns[0];
        let rotation = Quat::from_xyzw(
            spawn.rotation_xyzw[0],
            spawn.rotation_xyzw[1],
            spawn.rotation_xyzw[2],
            spawn.rotation_xyzw[3],
        );
        let drive_rotation = Quat::from_rotation_y(PI) * rotation;
        let forward = drive_rotation * Vec3::NEG_Z;
        let points = default_centerline();
        let tangent = (points[1] - points[0]).normalize_or_zero();
        assert!(forward.dot(tangent) > 0.99);
    }

    #[test]
    fn editor_track_has_a_mountain_backdrop() {
        let track = default_editor_track();
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
        let track = default_editor_track();
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
        for points in [
            default_centerline(),
            ring_project_centerline(&TIGHT_CONTROL_POINTS),
        ] {
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
        let points = default_centerline();
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
        let points = default_centerline();
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
        let track = default_editor_track();
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
        let track = default_editor_track();
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

        let points = default_centerline();
        let track = default_editor_track();
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
        let track = default_editor_track();
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

        let points = default_centerline();
        let track = default_editor_track();
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

        let points = default_centerline();
        let track = default_editor_track();
        assert!(track
            .metadata
            .surfaces
            .iter()
            .any(|surface| surface.name == "kerb"));
        let collider = TrackCollider::from_collision(&track.collision, &track.metadata);

        let span = *track.kerb_spans.first().expect("kerb span");
        let station = (span.start + span.end) * 0.5;
        let (point, binormal) = track_frame(&points, station);
        let sample = point + binormal * (6.4 * span.side as f32);
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
        let points = default_centerline();
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
    fn low_speed_turn_can_cross_the_kerb() {
        use sim_core::{Controls, Vehicle};

        let mut project = TrackProject::default_project();
        let route = project.layouts[0].route.clone();
        for (index, side) in [-1i8, 1i8].into_iter().enumerate() {
            project.decorations.push(DecorationPath {
                id: format!("kerb-{index}"),
                kind: DecorationKind::Kerb,
                side,
                lane: None,
                segments: route.clone(),
                start: None,
                end: None,
                elements: vec![project.settings.kerb.element_length],
                phase: 0.0,
                prop_kind: None,
                spacing: None,
            });
        }
        project.decorations.push(DecorationPath {
            id: "props-off".to_string(),
            kind: DecorationKind::Prop,
            side: 1,
            lane: Some("far".to_string()),
            segments: route,
            start: None,
            end: None,
            elements: Vec::new(),
            phase: 0.0,
            prop_kind: Some(PropKind::Off),
            spacing: None,
        });
        let track = track_from_project_with_default(&project, None).expect("editor track");
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
        let points = ring_project_centerline(&TIGHT_CONTROL_POINTS);
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
        let points = ring_project_centerline(&TIGHT_CONTROL_POINTS);
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
        let mut project = TrackProject::default_project();
        let segments = project.layouts[0].route.clone();
        let segment_refs = segments.iter().map(String::as_str).collect::<Vec<_>>();
        project.decorations.push(prop_decoration(
            "trees-a",
            &segment_refs,
            1,
            "far",
            PropKind::Trees,
            TREE_SPACING,
            0.0,
        ));
        let project = project.normalize().expect("normalize project");
        let track = track_from_project_with_default(&project, None).expect("editor track");
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
        let cards = mesh("tree_cards");
        assert_eq!(cards.uvs.len(), cards.positions.len(), "tree_cards uvs");
        let points = project_centerline(&project);
        let placements = track
            .prop_lines
            .iter()
            .map(|line| prop_placements(&points, line).len())
            .sum::<usize>();
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
        let card_triangles = triangles("tree_cards");
        assert!(
            card_triangles <= placements * 8,
            "card tree budget exceeded: {card_triangles} tris for {placements} trees"
        );
        assert!(track.collision.surface_ids.contains(&WALL_SURFACE));
        assert!(track
            .visual
            .meshes
            .iter()
            .all(|mesh| mesh.name != "barriers" && !mesh.name.starts_with("tires_")));
        let again = track_from_project_with_default(&project, None).expect("deterministic track");
        assert_eq!(track.visual, again.visual);
    }

    #[test]
    fn fractional_prop_range_follows_the_offset_path() {
        let points = default_centerline();
        let line = PropLine {
            start: 10.25,
            end: 12.75,
            side: 1,
            offset: PROP_FAR_OFFSET,
            kind: PropKind::Blocks,
            spacing: None,
            phase: 0.0,
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
        let points = default_centerline();
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
        let points = default_centerline();
        assert!(minimum_radius(&points) >= MIN_TURN_RADIUS - 0.01);
        assert_eq!(offset_crossings(&points, OUTER_EDGE_RADIUS), 0);
        assert_eq!(offset_crossings(&points, -OUTER_EDGE_RADIUS), 0);
        assert_eq!(offset_crossings(&points, PROP_OFFSET), 0);
        assert_eq!(offset_crossings(&points, -PROP_OFFSET), 0);
        assert_eq!(offset_crossings(&points, PROP_FAR_OFFSET), 0);
        assert_eq!(offset_crossings(&points, -PROP_FAR_OFFSET), 0);
    }
}
