use std::collections::{BTreeMap, BTreeSet};
use std::f32::consts::{PI, TAU};

use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};
use sim_types::{
    Aabb, AssetKind, AssetManifest, AssetStats, CameraSpec, EntryPoints, MaterialRef, MeshBuffers,
    MeshRef, SectorSpec, SpawnPoint, SurfaceSpec, TextureRef, TrackCollision, TrackMetadata,
    TrackVisual, ENGINE_FRAME, ENGINE_UNITS, MAGIC, SCHEMA_VERSION,
};

pub const TRACK_PROJECT_SCHEMA: &str = "lapster/track-project@6";
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
const BLOCK_SPACING: f32 = 2.0;
const TYRE_SPACING: f32 = 0.8;
const TREE_SPACING: f32 = 4.5;
const DEFAULT_NEAR_PROP_SEGMENTS: usize = 4;
const DEFAULT_NEAR_PROP_GAP: f32 = 2.0;
const DEFAULT_TREE_RING_SEGMENTS: usize = 6;
const DEFAULT_TREE_RING_GAP: f32 = 4.0;
/// Length of the capped branch created by `make_junction`, in metres.
const JUNCTION_BRANCH_LENGTH: f32 = 14.0;
/// Fixed sample count for deriving a junction's nearest contact point.
const JUNCTION_CONTACT_SAMPLES: usize = 256;
/// Fixed sample count for baking sections that are not on the active route.
const EXTRA_SECTION_SEGMENTS: usize = 32;
/// How far the flat loose-end cap extends past the section endpoint.
const LOOSE_END_CAP_LENGTH: f32 = 1.5;
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

/// A lane guide rail for the web decoration painter. Rails are derived from
/// the same baked prop lines and kerb radii as the renderer so the editor does
/// not hard-code lane offsets.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GuideRail {
    pub kind: String,
    pub side: i8,
    pub offset: f32,
    pub lane: Option<String>,
}

fn guide_rail_lane(offset: f32) -> String {
    if (offset - PROP_OFFSET).abs() < 1e-4 {
        "near".to_string()
    } else if (offset - PROP_FAR_OFFSET).abs() < 1e-4 {
        "far".to_string()
    } else {
        offset.to_string()
    }
}

/// Builds deterministic editor guide rails: two mid-kerb rails followed by one
/// rail per unique non-`Off` `(side, offset)` prop pair.
pub fn guide_rails(prop_lines: &[PropLine]) -> Vec<GuideRail> {
    let mid_kerb = (KERB_INNER_RADIUS + KERB_OUTER_RADIUS) * 0.5;
    let mut rails = vec![
        GuideRail {
            kind: "kerb".to_string(),
            side: -1,
            offset: mid_kerb,
            lane: None,
        },
        GuideRail {
            kind: "kerb".to_string(),
            side: 1,
            offset: mid_kerb,
            lane: None,
        },
    ];
    let mut prop_pairs: Vec<(i8, f32)> = Vec::new();
    for line in prop_lines {
        if line.kind == PropKind::Off {
            continue;
        }
        let pair = (line.side, line.offset);
        if !prop_pairs.contains(&pair) {
            prop_pairs.push(pair);
        }
    }
    prop_pairs.sort_by(|a, b| {
        a.1.partial_cmp(&b.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
    for (side, offset) in prop_pairs {
        rails.push(GuideRail {
            kind: "prop".to_string(),
            side,
            offset,
            lane: Some(guide_rail_lane(offset)),
        });
    }
    rails
}

pub struct EditorTrack {
    pub visual: TrackVisual,
    pub metadata: TrackMetadata,
    pub collision: TrackCollision,
    pub manifest: AssetManifest,
    pub kerb_spans: Vec<KerbSpan>,
    pub prop_lines: Vec<PropLine>,
}

/// The canonical `@6` venue graph. Points are global, sections reference point
/// ids, and layout/path routes are ordered legs over sections.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TrackProject {
    pub schema: String,
    pub name: String,
    pub settings: VenueSettings,
    #[serde(default)]
    pub points: Vec<Point>,
    #[serde(default)]
    pub sections: Vec<Section>,
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

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JunctionKind {
    #[default]
    Normal,
    Fork,
    PitEntry,
    PitExit,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PointEnd {
    #[default]
    Cap,
    Open,
    Barrier,
}

/// How a control point relates to the road surface.
///
/// `Anchor` points lie on the curve and clamp the B-spline; `Handle` points
/// shape the curve from off it.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PointKind {
    Anchor,
    #[default]
    Handle,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PointJunction {
    #[serde(default)]
    pub kind: JunctionKind,
}

/// A global graph point. Points are never duplicated between sections.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Point {
    pub id: String,
    pub position: [f32; 2],
    #[serde(default)]
    pub kind: PointKind,
    #[serde(default)]
    pub junction: Option<PointJunction>,
    #[serde(default)]
    pub end: Option<PointEnd>,
}

/// One spline. The first point of a closed section is its parameter seam.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Section {
    pub id: String,
    #[serde(default)]
    pub closed: bool,
    pub points: Vec<String>,
}

/// A route anchor is either a global point id or a closed section's seam.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Anchor {
    Seam,
    Point(String),
}

impl serde::Serialize for Anchor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(match self {
            Self::Seam => "seam",
            Self::Point(id) => id,
        })
    }
}

impl<'de> serde::Deserialize<'de> for Anchor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(if value == "seam" {
            Self::Seam
        } else {
            Self::Point(value)
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Leg {
    pub section_id: String,
    pub entry: Anchor,
    pub exit: Anchor,
    #[serde(default)]
    pub wrap: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LayoutProject {
    pub id: String,
    pub name: String,
    pub route: Vec<Leg>,
    pub start_finish: String,
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
    pub route: Vec<Leg>,
    #[serde(default)]
    pub boxes: Vec<SpawnPoint>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DecorationKind {
    #[default]
    Kerb,
    Prop,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DecorationLeg {
    pub section_id: String,
    #[serde(default)]
    pub entry_fraction: f32,
    #[serde(default = "default_exit_fraction")]
    pub exit_fraction: f32,
    #[serde(default)]
    pub wrap: bool,
}

fn default_exit_fraction() -> f32 {
    1.0
}

/// A canonical decoration over an ordered, contiguous chain of section
/// fractions. Per-section slices are derived at bake time.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DecorationPath {
    pub id: String,
    pub kind: DecorationKind,
    #[serde(default)]
    pub side: i8,
    #[serde(default)]
    pub lane: Option<String>,
    pub chain: Vec<DecorationLeg>,
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
    /// The built-in single-layout venue: one closed loop with no junctions.
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

    /// Builds a one-section closed-loop project from ordered XZ control points.
    pub fn ring_project(name: &str, layout_id: &str, control_points: &[[f32; 2]]) -> Self {
        let points = control_points
            .iter()
            .enumerate()
            .map(|(index, position)| Point {
                id: format!("point-{index}"),
                position: *position,
                kind: PointKind::Handle,
                junction: None,
                end: None,
            })
            .collect::<Vec<_>>();
        let point_ids = points
            .iter()
            .map(|point| point.id.clone())
            .collect::<Vec<_>>();
        Self {
            schema: TRACK_PROJECT_SCHEMA.to_string(),
            name: name.to_string(),
            settings: VenueSettings::default(),
            points,
            sections: vec![Section {
                id: "section-0".to_string(),
                closed: true,
                points: point_ids.clone(),
            }],
            layouts: vec![LayoutProject {
                id: layout_id.to_string(),
                name: name.to_string(),
                route: vec![Leg {
                    section_id: "section-0".to_string(),
                    entry: Anchor::Seam,
                    exit: Anchor::Seam,
                    wrap: true,
                }],
                start_finish: point_ids.first().cloned().unwrap_or_default(),
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

        validate_unique_ids(self.points.iter().map(|point| point.id.as_str()), "point")?;
        validate_unique_ids(
            self.sections.iter().map(|section| section.id.as_str()),
            "section",
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

        if self.points.is_empty() {
            return Err("track project needs at least one point".to_string());
        }
        for point in &self.points {
            if point.id.trim().is_empty() {
                return Err("point ids must not be empty".to_string());
            }
            if point.id == "seam" {
                return Err("point id 'seam' is reserved".to_string());
            }
            if !point.position[0].is_finite() || !point.position[1].is_finite() {
                return Err(format!("point '{}' has a non-finite position", point.id));
            }
        }

        // Authored points are handles. A junction is a handle with a derived
        // contact point on its host section, and start/finish contact lines are
        // likewise derived from the handle. Only open-section endpoints are
        // coerced to anchors; a junction endpoint stays a handle because its
        // attached section is clamped at the derived contact instead.
        for section in &self.sections {
            if section.closed {
                continue;
            }
            for point_id in [section.points.first(), section.points.last()]
                .into_iter()
                .flatten()
            {
                if let Some(point) = self.points.iter_mut().find(|point| point.id == *point_id) {
                    if point.junction.is_none() {
                        point.kind = PointKind::Anchor;
                    }
                }
            }
        }

        if self.sections.is_empty() {
            return Err("track project needs at least one section".to_string());
        }
        for section in &mut self.sections {
            if section.id.trim().is_empty() {
                return Err("section ids must not be empty".to_string());
            }
            let minimum = if section.closed { 3 } else { 2 };
            if section.points.len() < minimum {
                return Err(format!(
                    "section '{}' needs at least {minimum} points",
                    section.id
                ));
            }
            let mut seen = BTreeSet::new();
            for point_id in &section.points {
                if !self.points.iter().any(|point| point.id == *point_id) {
                    return Err(format!(
                        "section '{}' references unknown point '{point_id}'",
                        section.id
                    ));
                }
                if !seen.insert(point_id.as_str()) {
                    return Err(format!(
                        "section '{}' references point '{point_id}' more than once",
                        section.id
                    ));
                }
            }
        }

        let mut memberships = BTreeMap::<&str, usize>::new();
        for section in &self.sections {
            for point_id in &section.points {
                *memberships.entry(point_id.as_str()).or_default() += 1;
            }
        }
        for point in &self.points {
            if point.junction.is_some()
                && memberships.get(point.id.as_str()).copied().unwrap_or(0) < 2
            {
                return Err(format!(
                    "point '{}' has a junction role but is shared by fewer than two sections",
                    point.id
                ));
            }
        }

        if self.layouts.is_empty() {
            return Err("track project needs at least one layout".to_string());
        }
        for layout in &self.layouts {
            if layout.id.trim().is_empty() {
                return Err("layout ids must not be empty".to_string());
            }
            if layout.route.is_empty() {
                return Err(format!("layout '{}' route must not be empty", layout.id));
            }
            for leg in &layout.route {
                let section = self.section_by_id(&leg.section_id).ok_or_else(|| {
                    format!(
                        "layout '{}' route references unknown section '{}'",
                        layout.id, leg.section_id
                    )
                })?;
                validate_leg(self, section, leg, &format!("layout '{}'", layout.id))?;
            }
            for index in 0..layout.route.len() {
                let leg = &layout.route[index];
                let next = &layout.route[(index + 1) % layout.route.len()];
                let section = self
                    .section_by_id(&leg.section_id)
                    .expect("layout section validated");
                let next_section = self
                    .section_by_id(&next.section_id)
                    .expect("layout section validated");
                let exit = anchor_point_id(section, &leg.exit)?;
                let entry = anchor_point_id(next_section, &next.entry)?;
                if exit != entry {
                    return Err(format!(
                        "layout '{}' route is not a contiguous closed loop: leg '{}' exits at '{}' but next leg '{}' enters at '{}'",
                        layout.id, leg.section_id, exit, next.section_id, entry
                    ));
                }
            }
            if layout.start_finish.trim().is_empty() {
                return Err(format!(
                    "layout '{}' start/finish must reference a point",
                    layout.id
                ));
            }
            if self.point_by_id(&layout.start_finish).is_none() {
                return Err(format!(
                    "layout '{}' start/finish references unknown point '{}'",
                    layout.id, layout.start_finish
                ));
            }
            let start_in_route = layout.route.iter().any(|leg| {
                self.section_by_id(&leg.section_id)
                    .map(|section| section.points.iter().any(|id| id == &layout.start_finish))
                    .unwrap_or(false)
            });
            if !start_in_route {
                return Err(format!(
                    "layout '{}' start/finish point '{}' is not on a section in its route",
                    layout.id, layout.start_finish
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
            for leg in &path.route {
                let section = self.section_by_id(&leg.section_id).ok_or_else(|| {
                    format!(
                        "path '{}' route references unknown section '{}'",
                        path.id, leg.section_id
                    )
                })?;
                validate_leg(self, section, leg, &format!("path '{}'", path.id))?;
            }
            for index in 1..path.route.len() {
                let previous = &path.route[index - 1];
                let leg = &path.route[index];
                let previous_section = self
                    .section_by_id(&previous.section_id)
                    .expect("path section validated");
                let section = self
                    .section_by_id(&leg.section_id)
                    .expect("path section validated");
                let exit = anchor_point_id(previous_section, &previous.exit)?;
                let entry = anchor_point_id(section, &leg.entry)?;
                if exit != entry {
                    return Err(format!(
                        "path '{}' route is not contiguous: leg '{}' exits at '{}' but next leg '{}' enters at '{}'",
                        path.id, previous.section_id, exit, leg.section_id, entry
                    ));
                }
            }
        }

        for decoration in &self.decorations {
            if decoration.id.trim().is_empty() {
                return Err("decoration ids must not be empty".to_string());
            }
            if decoration.chain.is_empty() {
                return Err(format!(
                    "decoration '{}' needs at least one section leg",
                    decoration.id
                ));
            }
            for leg in &decoration.chain {
                let section = self.section_by_id(&leg.section_id).ok_or_else(|| {
                    format!(
                        "decoration '{}' references unknown section '{}'",
                        decoration.id, leg.section_id
                    )
                })?;
                for (label, fraction) in
                    [("entry", leg.entry_fraction), ("exit", leg.exit_fraction)]
                {
                    if !fraction.is_finite() || !(0.0..=1.0).contains(&fraction) {
                        return Err(format!(
                            "decoration '{}' {label} fraction must lie in [0, 1]",
                            decoration.id
                        ));
                    }
                }
                if leg.wrap && !section.closed {
                    return Err(format!(
                        "decoration '{}' wraps a seam on open section '{}'",
                        decoration.id, section.id
                    ));
                }
                if !leg.wrap && leg.entry_fraction > leg.exit_fraction + 1e-5 {
                    return Err(format!(
                        "decoration '{}' entry fraction must not follow its exit fraction",
                        decoration.id
                    ));
                }
            }
            for index in 1..decoration.chain.len() {
                let previous = &decoration.chain[index - 1];
                let leg = &decoration.chain[index];
                let previous_section = self
                    .section_by_id(&previous.section_id)
                    .expect("decoration section validated");
                let section = self
                    .section_by_id(&leg.section_id)
                    .expect("decoration section validated");
                if previous.section_id == leg.section_id {
                    if (previous.exit_fraction - leg.entry_fraction).abs() > 1e-4 {
                        return Err(format!(
                            "decoration '{}' chain is not contiguous within section '{}'",
                            decoration.id, leg.section_id
                        ));
                    }
                } else {
                    if (previous.exit_fraction - 1.0).abs() > 1e-4
                        || leg.entry_fraction.abs() > 1e-4
                    {
                        return Err(format!(
                            "decoration '{}' chain must leave a section at fraction 1 and enter the next at 0",
                            decoration.id
                        ));
                    }
                    let exit = section_end_point_id(previous_section);
                    let next_entry = section
                        .points
                        .first()
                        .map(String::as_str)
                        .unwrap_or_default();
                    if exit != next_entry {
                        return Err(format!(
                            "decoration '{}' chain crosses sections '{}' and '{}' that do not share a point",
                            decoration.id, previous_section.id, section.id
                        ));
                    }
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

    pub fn point_by_id(&self, id: &str) -> Option<&Point> {
        self.points.iter().find(|point| point.id == id)
    }

    pub fn section_by_id(&self, id: &str) -> Option<&Section> {
        self.sections.iter().find(|section| section.id == id)
    }

    #[allow(dead_code)]
    pub fn layout_by_id(&self, id: &str) -> Option<&LayoutProject> {
        self.layouts.iter().find(|layout| layout.id == id)
    }

    pub fn layout_index(&self, id: &str) -> Option<usize> {
        self.layouts.iter().position(|layout| layout.id == id)
    }

    /// Moves a global point. Every section that references it moves with it.
    pub fn move_point(&mut self, id: &str, x: f32, z: f32) -> Result<(), String> {
        if !x.is_finite() || !z.is_finite() {
            return Err("point position must be finite".to_string());
        }
        self.normalize_in_place()?;
        let mut candidate = self.clone();
        let point = candidate
            .points
            .iter_mut()
            .find(|point| point.id == id)
            .ok_or_else(|| format!("unknown point id '{id}'"))?;
        point.position = [x, z];
        candidate.normalize_in_place()?;
        *self = candidate;
        Ok(())
    }

    /// Inserts a new global point after `after_index` in a section and returns
    /// the new point id.
    pub fn insert_point(
        &mut self,
        section_id: &str,
        after_index: usize,
        x: f32,
        z: f32,
    ) -> Result<String, String> {
        if !x.is_finite() || !z.is_finite() {
            return Err("point position must be finite".to_string());
        }
        self.normalize_in_place()?;
        let mut candidate = self.clone();
        let new_id = unique_track_id(
            &format!("point-insert-{section_id}"),
            candidate.points.iter().map(|point| point.id.as_str()),
        );
        {
            let section = candidate
                .sections
                .iter_mut()
                .find(|section| section.id == section_id)
                .ok_or_else(|| format!("unknown section id '{section_id}'"))?;
            let index = after_index.saturating_add(1).min(section.points.len());
            section.points.insert(index, new_id.clone());
        }
        candidate.points.push(Point {
            id: new_id.clone(),
            position: [x, z],
            kind: PointKind::Handle,
            junction: None,
            end: None,
        });
        candidate.normalize_in_place()?;
        *self = candidate;
        Ok(new_id)
    }

    /// Deletes an unreferenced, non-junction, non-start/finish point.
    pub fn delete_point(&mut self, id: &str) -> Result<(), String> {
        self.normalize_in_place()?;
        let mut candidate = self.clone();
        let point = candidate
            .point_by_id(id)
            .cloned()
            .ok_or_else(|| format!("unknown point id '{id}'"))?;
        if point.junction.is_some() {
            return Err(format!(
                "cannot delete point '{id}': it has a junction role"
            ));
        }
        let memberships = candidate
            .sections
            .iter()
            .filter(|section| section.points.iter().any(|point_id| point_id == id))
            .count();
        if memberships > 1 {
            return Err(format!(
                "cannot delete point '{id}': it is shared by {memberships} sections"
            ));
        }
        if candidate
            .layouts
            .iter()
            .any(|layout| layout.start_finish == id)
        {
            return Err(format!(
                "cannot delete point '{id}': it is a layout start/finish"
            ));
        }
        for section in &mut candidate.sections {
            let Some(index) = section.points.iter().position(|point_id| point_id == id) else {
                continue;
            };
            let minimum = if section.closed { 3 } else { 2 };
            if section.points.len() <= minimum {
                return Err(format!(
                    "cannot delete point '{id}': section '{}' would fall below its minimum point count",
                    section.id
                ));
            }
            section.points.remove(index);
        }
        candidate.points.retain(|point| point.id != id);
        candidate.normalize_in_place()?;
        *self = candidate;
        Ok(())
    }

    /// Adds the junction role to an interior host-section point and appends a
    /// short open capped branch. The host section is never split.
    pub fn make_junction(&mut self, point_id: &str) -> Result<String, String> {
        self.normalize_in_place()?;
        let mut candidate = self.clone();
        let point = candidate
            .point_by_id(point_id)
            .cloned()
            .ok_or_else(|| format!("unknown point id '{point_id}'"))?;
        if point.junction.is_some() {
            return Err(format!("point '{point_id}' already has a junction role"));
        }
        if candidate
            .layouts
            .iter()
            .any(|layout| layout.start_finish == point_id)
        {
            return Err(format!(
                "cannot make point '{point_id}' a junction: it is a layout start/finish"
            ));
        }
        if junction_host_section(&candidate, point_id).is_none() {
            return Err(format!(
                "cannot make point '{point_id}' a junction: it must be an interior point of a host section"
            ));
        }
        let contact = junction_contact(&candidate, point_id)?
            .ok_or_else(|| format!("cannot derive a contact point for junction '{point_id}'"))?;

        let mut tangent = Vec3::new(contact.tangent.x, 0.0, contact.tangent.z);
        if tangent.length_squared() < 1e-8 {
            tangent = Vec3::X;
        }
        tangent = tangent.normalize_or_zero();
        let mut perpendicular = Vec3::new(-tangent.z, 0.0, tangent.x).normalize_or_zero();
        if perpendicular.length_squared() < 1e-8 {
            perpendicular = Vec3::Z;
        }
        let end = contact.position + perpendicular * JUNCTION_BRANCH_LENGTH;
        if !end.x.is_finite() || !end.z.is_finite() {
            return Err(format!(
                "cannot make point '{point_id}' a junction: branch geometry is not finite"
            ));
        }

        let branch_id = unique_track_id(
            &format!("section-branch-{point_id}"),
            candidate.sections.iter().map(|section| section.id.as_str()),
        );
        let loose_id = unique_track_id(
            &format!("point-branch-{point_id}"),
            candidate.points.iter().map(|point| point.id.as_str()),
        );
        if let Some(point) = candidate
            .points
            .iter_mut()
            .find(|point| point.id == point_id)
        {
            point.junction = Some(PointJunction {
                kind: JunctionKind::Normal,
            });
        }
        candidate.points.push(Point {
            id: loose_id.clone(),
            position: [end.x, end.z],
            kind: PointKind::Handle,
            junction: None,
            end: Some(PointEnd::Cap),
        });
        candidate.sections.push(Section {
            id: branch_id.clone(),
            closed: false,
            points: vec![point_id.to_string(), loose_id],
        });
        candidate.normalize_in_place()?;
        *self = candidate;
        Ok(branch_id)
    }

    /// Sets a layout's start/finish point.
    pub fn set_start_finish(&mut self, layout_id: &str, point_id: &str) -> Result<(), String> {
        self.normalize_in_place()?;
        let mut candidate = self.clone();
        let layout = candidate
            .layouts
            .iter_mut()
            .find(|layout| layout.id == layout_id)
            .ok_or_else(|| format!("unknown layout id '{layout_id}'"))?;
        layout.start_finish = point_id.to_string();
        candidate.normalize_in_place()?;
        *self = candidate;
        Ok(())
    }

    /// Samples one section across its whole parameter range.
    pub fn section_centerline(&self, section_id: &str) -> Result<Vec<[f32; 3]>, String> {
        let project = self.clone().normalize()?;
        let section = project
            .section_by_id(section_id)
            .ok_or_else(|| format!("unknown section id '{section_id}'"))?;
        let mut points = Vec::with_capacity(SEGMENTS + 1);
        for index in 0..=SEGMENTS {
            let local = index as f32 / SEGMENTS as f32;
            let mut point = sample_section_at(&project, section, local)?;
            point.y = terrain_height(point.x, point.z);
            points.push(point.to_array());
        }
        Ok(points)
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

/// Indices of the section's on-curve anchors, in authored order.
///
/// Open-section endpoints that carry a junction role are anchors for that
/// section even though the shared point stays a handle on its host section.
/// The attached section is clamped at the junction's derived contact point.
fn section_anchor_indices(project: &TrackProject, section: &Section) -> Vec<usize> {
    let last = section.points.len().saturating_sub(1);
    section
        .points
        .iter()
        .enumerate()
        .filter_map(|(index, point_id)| {
            let point = project.point_by_id(point_id)?;
            let junction_endpoint =
                !section.closed && point.junction.is_some() && (index == 0 || index == last);
            (point.kind == PointKind::Anchor || junction_endpoint).then_some(index)
        })
        .collect()
}

fn section_point_is_interior(section: &Section, index: usize) -> bool {
    section.points.len() >= 3 && index > 0 && index + 1 < section.points.len()
}

/// The first authored section in which `point_id` is an interior point.
fn junction_host_section<'a>(
    project: &'a TrackProject,
    point_id: &str,
) -> Option<(&'a Section, usize)> {
    project.sections.iter().find_map(|section| {
        let index = section
            .points
            .iter()
            .position(|candidate| candidate == point_id)?;
        section_point_is_interior(section, index).then_some((section, index))
    })
}

#[derive(Clone, Copy, Debug)]
struct JunctionContact {
    position: Vec3,
    tangent: Vec3,
}

/// Derives the nearest on-road contact point and tangent for a junction
/// handle from its host section. The host section is evaluated without
/// substituting junction endpoints so this cannot recurse.
fn junction_contact(
    project: &TrackProject,
    point_id: &str,
) -> Result<Option<JunctionContact>, String> {
    let Some((host, _)) = junction_host_section(project, point_id) else {
        return Ok(None);
    };
    let handle = point_position(project, point_id)?;
    let curve = section_curve_raw(project, host)?;
    let distance_squared = |parameter: f32| {
        let point = curve.eval(parameter);
        (point.x - handle.x).powi(2) + (point.z - handle.z).powi(2)
    };

    let samples = JUNCTION_CONTACT_SAMPLES;
    let mut best_parameter = 0.0;
    let mut best_distance = f32::INFINITY;
    for index in 0..=samples {
        let parameter = index as f32 / samples as f32;
        let distance = distance_squared(parameter);
        if distance < best_distance {
            best_distance = distance;
            best_parameter = parameter;
        }
    }

    // Refine the sample bracket with a deterministic ternary search.
    let step = 1.0 / samples as f32;
    let mut low = (best_parameter - step).max(0.0);
    let mut high = (best_parameter + step).min(1.0);
    for _ in 0..40 {
        let first = low + (high - low) / 3.0;
        let second = high - (high - low) / 3.0;
        if distance_squared(first) < distance_squared(second) {
            high = second;
        } else {
            low = first;
        }
    }
    let parameter = (low + high) * 0.5;
    let mut position = curve.eval(parameter);
    position.y = 0.0;
    let epsilon = 1e-3;
    let before = curve.eval((parameter - epsilon).max(0.0));
    let after = curve.eval((parameter + epsilon).min(1.0));
    let mut tangent = Vec3::new(after.x - before.x, 0.0, after.z - before.z);
    if tangent.length_squared() < 1e-10 {
        tangent = Vec3::X;
    }
    Ok(Some(JunctionContact {
        position,
        tangent: tangent.normalize_or_zero(),
    }))
}

fn anchor_point_id<'a>(section: &'a Section, anchor: &'a Anchor) -> Result<&'a str, String> {
    match anchor {
        Anchor::Seam => section
            .points
            .first()
            .map(String::as_str)
            .ok_or_else(|| format!("section '{}' has no seam point", section.id)),
        Anchor::Point(id) => Ok(id.as_str()),
    }
}

fn section_end_point_id(section: &Section) -> &str {
    if section.closed {
        section.points.first().map(String::as_str).unwrap_or("")
    } else {
        section.points.last().map(String::as_str).unwrap_or("")
    }
}

/// Maps a route anchor to the section's `[0, 1]` parameter.
///
/// Anchors split a section into one span per anchor pair, and every span takes
/// the same parameter width. Handles are not on-curve, so a route that refers
/// to a handle is mapped by its authored index for backwards compatibility;
/// callers should prefer anchors for route endpoints.
fn anchor_parameter(
    project: &TrackProject,
    section: &Section,
    anchor: &Anchor,
) -> Result<f32, String> {
    match anchor {
        Anchor::Seam => {
            if !section.closed {
                return Err(format!(
                    "section '{}' is open and has no seam anchor",
                    section.id
                ));
            }
            Ok(0.0)
        }
        Anchor::Point(id) => {
            let index = section
                .points
                .iter()
                .position(|point_id| point_id == id)
                .ok_or_else(|| {
                    format!(
                        "section '{}' route anchor references point '{}' outside the section",
                        section.id, id
                    )
                })?;
            let anchors = section_anchor_indices(project, section);
            if anchors.is_empty() {
                // Periodic closed loop: each control point owns one segment.
                return Ok(index as f32 / section.points.len() as f32);
            }
            if let Some(order) = anchors
                .iter()
                .position(|anchor_index| *anchor_index == index)
            {
                let span_count = if section.closed {
                    anchors.len()
                } else {
                    anchors.len().saturating_sub(1)
                };
                if span_count == 0 {
                    return Err(format!("section '{}' has no span", section.id));
                }
                return Ok(order as f32 / span_count as f32);
            }
            // Handle route endpoints are not on-curve; keep an index-based
            // parameter so validator diagnostics still refer to a location.
            Ok(index as f32 / section.points.len().max(1) as f32)
        }
    }
}

fn validate_leg(
    project: &TrackProject,
    section: &Section,
    leg: &Leg,
    label: &str,
) -> Result<(), String> {
    for (name, anchor) in [("entry", &leg.entry), ("exit", &leg.exit)] {
        match anchor {
            Anchor::Seam => {
                if !section.closed {
                    return Err(format!(
                        "{label} route references the seam of open section '{}'",
                        section.id
                    ));
                }
            }
            Anchor::Point(id) => {
                if project.point_by_id(id).is_none() {
                    return Err(format!("{label} route references unknown point '{id}'"));
                }
                if !section.points.iter().any(|point_id| point_id == id) {
                    return Err(format!(
                        "{label} route anchor point '{id}' is not on section '{}'",
                        section.id
                    ));
                }
            }
        }
        if leg.wrap && !section.closed {
            return Err(format!(
                "{label} route cannot wrap open section '{}'",
                section.id
            ));
        }
        let _ = name;
    }
    let start = anchor_parameter(project, section, &leg.entry)?;
    let mut end = anchor_parameter(project, section, &leg.exit)?;
    if section.closed && leg.wrap && end <= start + 1e-6 {
        end += 1.0;
    }
    if end - start <= 1e-5 {
        return Err(format!(
            "{label} route leg over section '{}' is empty",
            section.id
        ));
    }
    Ok(())
}

fn decoration_leg_intervals(leg: &DecorationLeg) -> Vec<(String, f32, f32)> {
    let mut intervals = Vec::new();
    if leg.wrap {
        if (leg.entry_fraction - leg.exit_fraction).abs() <= 1e-5 {
            intervals.push((leg.section_id.clone(), 0.0, 1.0));
        } else {
            intervals.push((leg.section_id.clone(), leg.entry_fraction, 1.0));
            intervals.push((leg.section_id.clone(), 0.0, leg.exit_fraction));
        }
    } else {
        intervals.push((
            leg.section_id.clone(),
            leg.entry_fraction,
            leg.exit_fraction,
        ));
    }
    intervals.retain(|(_, start, end)| end - start > 1e-5);
    intervals
}

fn decorations_share_lane(a: &DecorationPath, b: &DecorationPath) -> bool {
    a.kind == b.kind && (a.side < 0) == (b.side < 0) && a.lane == b.lane
}

fn validate_decoration_coverage(decorations: &[DecorationPath]) -> Result<(), String> {
    let mut coverages = Vec::with_capacity(decorations.len());
    for decoration in decorations {
        let mut intervals = Vec::new();
        for leg in &decoration.chain {
            intervals.extend(decoration_leg_intervals(leg));
        }
        coverages.push(intervals);
    }
    for first in 0..decorations.len() {
        for second in first + 1..decorations.len() {
            if !decorations_share_lane(&decorations[first], &decorations[second]) {
                continue;
            }
            for (section_id, start, end) in &coverages[first] {
                for (other_section, other_start, other_end) in &coverages[second] {
                    if section_id != other_section {
                        continue;
                    }
                    if *start < *other_end - 1e-5 && *other_start < *end - 1e-5 {
                        return Err(format!(
                            "decorations '{}' and '{}' overlap on section '{}'",
                            decorations[first].id, decorations[second].id, section_id
                        ));
                    }
                }
            }
        }
    }
    Ok(())
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

fn point_position(project: &TrackProject, id: &str) -> Result<Vec3, String> {
    let point = project
        .point_by_id(id)
        .ok_or_else(|| format!("unknown point id '{id}'"))?;
    Ok(Vec3::new(point.position[0], 0.0, point.position[1]))
}

fn section_positions(project: &TrackProject, section: &Section) -> Result<Vec<Vec3>, String> {
    section
        .points
        .iter()
        .map(|point_id| point_position(project, point_id))
        .collect()
}

/// One section's B-spline control structure.
///
/// Exact C1 continuity at shared anchors is not yet enforced: each span is
/// clamped independently by its tripled anchors, so the tangents on either
/// side of an anchor are not explicitly matched.
enum SectionCurve {
    /// Closed section with no anchors: every control point shapes a periodic
    /// loop.
    Periodic { controls: Vec<Vec3> },
    /// Anchor-to-anchor spans. Each span triples its anchors so the curve is
    /// clamped onto them; interior handles remain off-curve.
    Spans { spans: Vec<Vec<Vec3>> },
}

/// Control list for one span between anchors `from` and `to`.
///
/// The list is `[A, A, A, handles..., B, B, B]`, which evaluates to a straight
/// span when there are no handles. `to` may precede `from` to wrap a closed
/// section.
fn span_controls(positions: &[Vec3], from: usize, to: usize) -> Vec<Vec3> {
    let count = positions.len();
    let mut controls = Vec::with_capacity(6);
    controls.extend([positions[from], positions[from], positions[from]]);
    let mut index = (from + 1) % count;
    while index != to {
        controls.push(positions[index]);
        index = (index + 1) % count;
    }
    controls.extend([positions[to], positions[to], positions[to]]);
    controls
}

fn section_curve_raw(project: &TrackProject, section: &Section) -> Result<SectionCurve, String> {
    section_curve_internal(project, section, false)
}

fn section_curve(project: &TrackProject, section: &Section) -> Result<SectionCurve, String> {
    section_curve_internal(project, section, true)
}

fn section_curve_internal(
    project: &TrackProject,
    section: &Section,
    substitute_junction_endpoints: bool,
) -> Result<SectionCurve, String> {
    let mut positions = section_positions(project, section)?;
    if positions.len() < 2 {
        return Err(format!("section '{}' cannot be sampled", section.id));
    }
    if substitute_junction_endpoints && !section.closed {
        for index in [0, section.points.len().saturating_sub(1)] {
            let Some(point_id) = section.points.get(index) else {
                continue;
            };
            let Some(point) = project.point_by_id(point_id) else {
                continue;
            };
            if point.junction.is_none() {
                continue;
            }
            if let Some(contact) = junction_contact(project, point_id)? {
                positions[index] = contact.position;
            }
        }
    }
    let anchors = section_anchor_indices(project, section);
    if section.closed && anchors.is_empty() {
        return Ok(SectionCurve::Periodic {
            controls: positions,
        });
    }

    let mut spans = Vec::new();
    if section.closed {
        for (order, &from) in anchors.iter().enumerate() {
            let to = anchors[(order + 1) % anchors.len()];
            spans.push(span_controls(&positions, from, to));
        }
    } else {
        for pair in anchors.windows(2) {
            spans.push(span_controls(&positions, pair[0], pair[1]));
        }
    }
    if spans.is_empty() {
        return Err(format!("section '{}' has no anchor spans", section.id));
    }
    Ok(SectionCurve::Spans { spans })
}

fn cubic_bspline_segment(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, u: f32) -> Vec3 {
    let u2 = u * u;
    let u3 = u2 * u;
    let b0 = (1.0 - u).powi(3) / 6.0;
    let b1 = (3.0 * u3 - 6.0 * u2 + 4.0) / 6.0;
    let b2 = (-3.0 * u3 + 3.0 * u2 + 3.0 * u + 1.0) / 6.0;
    let b3 = u3 / 6.0;
    p0 * b0 + p1 * b1 + p2 * b2 + p3 * b3
}

/// Evaluates one clamped control list at local `t` in `[0, 1]`.
fn evaluate_span(controls: &[Vec3], t: f32) -> Vec3 {
    let count = controls.len();
    if count < 4 {
        return controls.first().copied().unwrap_or(Vec3::ZERO);
    }
    let t = t.clamp(0.0, 1.0);
    let segments = count - 3;
    let scaled = t * segments as f32;
    let index = (scaled.floor() as usize).min(count - 4);
    let u = scaled - index as f32;
    cubic_bspline_segment(
        controls[index],
        controls[index + 1],
        controls[index + 2],
        controls[index + 3],
        u,
    )
}

fn evaluate_periodic(controls: &[Vec3], t: f32) -> Vec3 {
    let count = controls.len();
    if count == 0 {
        return Vec3::ZERO;
    }
    let t = t.rem_euclid(1.0);
    let scaled = t * count as f32;
    let index = (scaled.floor() as usize) % count;
    let u = scaled - scaled.floor();
    cubic_bspline_segment(
        controls[(index + count - 1) % count],
        controls[index],
        controls[(index + 1) % count],
        controls[(index + 2) % count],
        u,
    )
}

impl SectionCurve {
    fn eval(&self, t: f32) -> Vec3 {
        match self {
            Self::Periodic { controls } => evaluate_periodic(controls, t),
            Self::Spans { spans } => {
                let t = t.clamp(0.0, 1.0);
                let scaled = t * spans.len() as f32;
                let index = (scaled.floor() as usize).min(spans.len() - 1);
                let u = scaled - index as f32;
                evaluate_span(&spans[index], u)
            }
        }
    }
}

/// Samples one section at `local` in `[0, 1]` with a cubic B-spline. The
/// returned height is left at zero; callers apply the terrain height.
fn sample_section_at(
    project: &TrackProject,
    section: &Section,
    local: f32,
) -> Result<Vec3, String> {
    let curve = section_curve(project, section)?;
    let mut point = curve.eval(local);
    point.y = 0.0;
    Ok(point)
}

#[derive(Clone, Copy)]
struct RouteLegSpan<'a> {
    section: &'a Section,
    start: f32,
    end: f32,
}

fn route_leg_spans<'a>(
    project: &'a TrackProject,
    layout: &LayoutProject,
) -> Result<Vec<RouteLegSpan<'a>>, String> {
    let mut spans = Vec::with_capacity(layout.route.len());
    for (index, leg) in layout.route.iter().enumerate() {
        let section = project.section_by_id(&leg.section_id).ok_or_else(|| {
            format!(
                "layout '{}' route references unknown section '{}'",
                layout.id, leg.section_id
            )
        })?;
        let start = anchor_parameter(project, section, &leg.entry)?;
        let mut end = anchor_parameter(project, section, &leg.exit)?;
        if section.closed && leg.wrap && end <= start + 1e-6 {
            end += 1.0;
        }
        if end - start <= 1e-5 {
            return Err(format!(
                "layout '{}' route leg {} over section '{}' is empty",
                layout.id, index, section.id
            ));
        }
        spans.push(RouteLegSpan {
            section,
            start,
            end,
        });
    }
    Ok(spans)
}

/// Fraction of the route sample nearest the layout's start/finish point.
///
/// The start/finish point may be an off-curve handle, so its contact point is
/// the sampled route point closest to the handle in XZ. The returned fraction
/// is the contact station's fraction in `[0, 1)`; the line orientation follows
/// the route tangent at that station.
fn layout_start_fraction(
    project: &TrackProject,
    layout: &LayoutProject,
    points: &[Vec3],
) -> Result<f32, String> {
    if points.is_empty() {
        return Err(format!("layout '{}' route has zero length", layout.id));
    }
    let handle = point_position(project, &layout.start_finish)?;
    let mut nearest_index = 0;
    let mut nearest_distance = f32::INFINITY;
    for (index, point) in points.iter().enumerate() {
        let dx = point.x - handle.x;
        let dz = point.z - handle.z;
        let distance = dx * dx + dz * dz;
        if distance < nearest_distance {
            nearest_distance = distance;
            nearest_index = index;
        }
    }
    Ok(nearest_index as f32 / points.len() as f32)
}

/// Builds the centerline for one layout from its route legs. The default
/// full-loop leg produces the same number of stations as the original loop.
fn sample_layout_loop(
    project: &TrackProject,
    layout: &LayoutProject,
) -> Result<(Vec<Vec3>, f32), String> {
    let spans = route_leg_spans(project, layout)?;
    if spans.is_empty() {
        return Err(format!("layout '{}' route must not be empty", layout.id));
    }
    let total = spans.iter().map(|span| span.end - span.start).sum::<f32>();
    if total <= 1e-6 {
        return Err(format!("layout '{}' route has zero length", layout.id));
    }

    let mut points = Vec::with_capacity(SEGMENTS);
    for index in 0..SEGMENTS {
        let route_parameter = index as f32 / SEGMENTS as f32 * total;
        let mut accumulated = 0.0;
        let mut chosen = spans.len() - 1;
        let mut local = 0.0;
        for (span_index, span) in spans.iter().enumerate() {
            let length = span.end - span.start;
            if route_parameter < accumulated + length || span_index == spans.len() - 1 {
                chosen = span_index;
                local = if length > 1e-6 {
                    (route_parameter - accumulated) / length
                } else {
                    0.0
                };
                break;
            }
            accumulated += length;
        }
        let span = &spans[chosen];
        let parameter = span.start + (span.end - span.start) * local.clamp(0.0, 0.999_999);
        let parameter = if span.section.closed {
            parameter.rem_euclid(1.0)
        } else {
            parameter.clamp(0.0, 1.0)
        };
        points.push(sample_section_at(project, span.section, parameter)?);
    }
    for point in &mut points {
        point.y = terrain_height(point.x, point.z);
    }

    let start_finish = layout_start_fraction(project, layout, &points)?;
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

/// Maps one decoration leg onto the route parameter range of one route span.
/// The returned interval is relative to the span's route start, and may exceed
/// the span length when the leg wraps a closed section's seam.
fn decoration_leg_span_interval(leg: &DecorationLeg, length: f32) -> Option<(f32, f32)> {
    let start = length * leg.entry_fraction;
    let mut end = length * leg.exit_fraction;
    if leg.wrap && end <= start + 1e-4 {
        end += length;
    }
    (end > start + 1e-4).then_some((start, end))
}

/// Continues a decoration chain from `leg_index` at absolute route parameter
/// `cursor`, following route spans in cyclic order. Returns the absolute route
/// parameter of the chain's end when every leg maps contiguously.
#[allow(clippy::too_many_arguments)]
fn match_decoration_chain(
    chain: &[DecorationLeg],
    spans: &[RouteLegSpan<'_>],
    bases: &[f32],
    lengths: &[f32],
    total: f32,
    leg_index: usize,
    previous_span: usize,
    cursor: f32,
) -> Option<f32> {
    if leg_index >= chain.len() {
        return Some(cursor);
    }
    let leg = &chain[leg_index];
    let tolerance = 1e-3 * total.max(1.0);
    for offset in 0..spans.len() {
        let span_index = (previous_span + offset) % spans.len();
        if spans[span_index].section.id != leg.section_id {
            continue;
        }
        let Some((relative_start, relative_end)) =
            decoration_leg_span_interval(leg, lengths[span_index])
        else {
            continue;
        };
        let shift = ((cursor - bases[span_index] - relative_start) / total).round();
        let absolute_start = bases[span_index] + shift * total + relative_start;
        if (absolute_start - cursor).abs() > tolerance {
            continue;
        }
        let absolute_end = bases[span_index] + shift * total + relative_end;
        if absolute_end <= absolute_start + 1e-4 {
            continue;
        }
        if let Some(end) = match_decoration_chain(
            chain,
            spans,
            bases,
            lengths,
            total,
            leg_index + 1,
            span_index,
            absolute_end,
        ) {
            return Some(end);
        }
    }
    None
}

/// Returns the route station range covered by a decoration chain.
///
/// Every leg is mapped onto the matching route leg occurrence, so a chain may
/// span sections, revisit one section, or wrap a closed section's seam while
/// still producing one continuous range. `None` means the chain is empty. A
/// chain that does not match the active route is an error rather than a silent
/// drop.
fn decoration_route_range(
    spans: &[RouteLegSpan<'_>],
    decoration: &DecorationPath,
    route_scale: f32,
) -> Result<Option<(f32, f32)>, String> {
    if spans.is_empty() {
        return Err(format!(
            "decoration '{}' cannot be baked without a route",
            decoration.id
        ));
    }
    let first = decoration.chain.first().ok_or_else(|| {
        format!(
            "decoration '{}' needs at least one section leg",
            decoration.id
        )
    })?;

    let lengths = spans
        .iter()
        .map(|span| span.end - span.start)
        .collect::<Vec<_>>();
    let mut bases = Vec::with_capacity(lengths.len());
    let mut total = 0.0;
    for length in &lengths {
        bases.push(total);
        total += *length;
    }
    if total <= 1e-6 {
        return Err(format!(
            "decoration '{}' cannot be baked on a zero-length route",
            decoration.id
        ));
    }

    let mut section_on_route = false;
    let mut has_interval = false;
    for span_index in 0..spans.len() {
        if spans[span_index].section.id != first.section_id {
            continue;
        }
        section_on_route = true;
        let Some((relative_start, relative_end)) =
            decoration_leg_span_interval(first, lengths[span_index])
        else {
            continue;
        };
        has_interval = true;
        let start = bases[span_index] + relative_start;
        let end = bases[span_index] + relative_end;
        if let Some(chain_end) = match_decoration_chain(
            &decoration.chain,
            spans,
            &bases,
            &lengths,
            total,
            1,
            span_index,
            end,
        ) {
            return Ok(Some((start * route_scale, chain_end * route_scale)));
        }
    }

    if !section_on_route {
        return Err(format!(
            "decoration '{}' references section '{}' outside the active route",
            decoration.id, first.section_id
        ));
    }
    if !has_interval {
        return Ok(None);
    }
    Err(format!(
        "decoration '{}' chain is not contiguous on the active route",
        decoration.id
    ))
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
    let mut end = end.max(0.0);
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
    let mut end = end.max(0.0);
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

fn point_junction_clearance(project: &TrackProject, point_id: &str) -> f32 {
    let memberships = project
        .sections
        .iter()
        .filter(|section| section.points.iter().any(|id| id == point_id))
        .count();
    let kind = project
        .point_by_id(point_id)
        .and_then(|point| point.junction.as_ref())
        .map(|junction| junction.kind);
    match kind {
        Some(JunctionKind::Fork) => 1.0,
        Some(JunctionKind::PitEntry | JunctionKind::PitExit) => 2.0,
        Some(JunctionKind::Normal) | None => {
            if memberships > 2 {
                1.0
            } else {
                0.0
            }
        }
    }
}

fn decoration_endpoint_clearance(
    project: &TrackProject,
    decoration: &DecorationPath,
    start: bool,
) -> f32 {
    let Some(leg) = (if start {
        decoration.chain.first()
    } else {
        decoration.chain.last()
    }) else {
        return 0.0;
    };
    let Some(section) = project.section_by_id(&leg.section_id) else {
        return 0.0;
    };
    let fraction = if start {
        leg.entry_fraction
    } else {
        leg.exit_fraction
    };
    let endpoint = if fraction <= 1e-4 {
        section.points.first().map(String::as_str)
    } else if fraction >= 1.0 - 1e-4 {
        Some(section_end_point_id(section))
    } else {
        None
    };
    endpoint
        .map(|point_id| point_junction_clearance(project, point_id))
        .unwrap_or(0.0)
}

/// Bakes canonical decoration chains into route-local slices for one layout.
/// Slices are derived at bake time and are never written back to the project.
fn bake_route_decorations(
    project: &TrackProject,
    layout: &LayoutProject,
    points: &[Vec3],
) -> Result<BakedRouteDecorations, String> {
    let arc = RouteArc::new(points);
    let spans = route_leg_spans(project, layout)?;
    let total = spans.iter().map(|span| span.end - span.start).sum::<f32>();
    let route_scale = if total <= 1e-6 {
        0.0
    } else {
        points.len() as f32 / total
    };
    let mut decorations = BakedRouteDecorations::default();
    for decoration in &project.decorations {
        let Some((start, end)) = decoration_route_range(&spans, decoration, route_scale)? else {
            continue;
        };
        let limit = points.len() as f32;
        let (start, end) = if route_scale > 1e-6 && end <= limit + 1e-4 {
            let start_clearance = decoration_endpoint_clearance(project, decoration, true);
            let end_clearance = decoration_endpoint_clearance(project, decoration, false);
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
    validate_loose_end_treatments(&project)?;
    let (points, start_finish) = sample_layout_loop(&project, layout)?;
    let extras = bake_extra_sections(&project, layout)?;
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
        extras,
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
    extras: BakedExtraSections,
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

    if !extras.ribbon.buffers.positions.is_empty() {
        append_collision_surfaces(&mut collision, &extras.ribbon.buffers, |index| {
            extras
                .ribbon
                .surface_ids
                .get(index)
                .copied()
                .unwrap_or(TRACK_SURFACE)
        });
        push_mesh(
            &mut visual,
            "branch_asphalt",
            1,
            extras.ribbon.buffers,
            false,
        );
    }
    if !extras.caps.positions.is_empty() {
        append_collision(&mut collision, &extras.caps, TRACK_SURFACE);
        push_mesh(&mut visual, "loose_end_caps", 1, extras.caps, false);
    }

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

struct TrackRibbon {
    buffers: MeshBuffers,
    surface_ids: Vec<u16>,
}

fn track_ribbon(points: &[Vec3]) -> TrackRibbon {
    ribbon_from_points(points, true)
}

fn ribbon_from_points(points: &[Vec3], closed: bool) -> TrackRibbon {
    let mut buffers = empty_buffers();
    let mut surface_ids = Vec::new();
    let count = points.len();
    if count < 2 {
        return TrackRibbon {
            buffers,
            surface_ids,
        };
    }
    let radii = [
        -OUTER_EDGE_RADIUS,
        -ASPHALT_HALF_WIDTH,
        ASPHALT_HALF_WIDTH,
        OUTER_EDGE_RADIUS,
    ];
    let surfaces = [DIRT_SURFACE, TRACK_SURFACE, DIRT_SURFACE];
    let segment_count = if closed { count } else { count - 1 };
    let total_length = (0..segment_count)
        .map(|index| points[index].distance(points[(index + 1) % count]))
        .sum::<f32>();
    let tiles_along = (total_length / TARMAC_TILE_METERS).round().max(1.0);
    let mut distance = 0.0;
    for index in 0..=segment_count {
        let sample = if closed { index % count } else { index };
        let point = points[sample];
        let tangent = if closed {
            (points[(sample + 1) % count] - point).normalize_or_zero()
        } else if sample + 1 < count {
            (points[sample + 1] - point).normalize_or_zero()
        } else {
            (point - points[sample - 1]).normalize_or_zero()
        };
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
        if index < segment_count {
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
        if index < segment_count {
            distance += point.distance(points[(sample + 1) % count]);
        }
    }
    TrackRibbon {
        buffers,
        surface_ids,
    }
}

fn append_track_ribbon(dst: &mut TrackRibbon, src: TrackRibbon) {
    if src.buffers.positions.is_empty() {
        return;
    }
    let offset = dst.buffers.positions.len() as u32;
    dst.buffers.positions.extend(src.buffers.positions);
    dst.buffers.normals.extend(src.buffers.normals);
    dst.buffers.uvs.extend(src.buffers.uvs);
    dst.buffers.colors.extend(src.buffers.colors);
    dst.buffers
        .indices
        .extend(src.buffers.indices.into_iter().map(|index| index + offset));
    dst.surface_ids.extend(src.surface_ids);
}

struct BakedExtraSections {
    ribbon: TrackRibbon,
    caps: MeshBuffers,
}

fn sample_extra_section(project: &TrackProject, section: &Section) -> Result<Vec<Vec3>, String> {
    let segments = EXTRA_SECTION_SEGMENTS;
    let count = if section.closed {
        segments
    } else {
        segments + 1
    };
    let mut points = Vec::with_capacity(count);
    for index in 0..count {
        let parameter = index as f32 / segments as f32;
        let mut point = sample_section_at(project, section, parameter)?;
        point.y = terrain_height(point.x, point.z);
        points.push(point);
    }
    Ok(points)
}

fn loose_end_treatment(point: &Point) -> PointEnd {
    point.end.unwrap_or(PointEnd::Cap)
}

fn append_loose_end_cap(buffers: &mut MeshBuffers, points: &[Vec3], at_start: bool) {
    let count = points.len();
    if count < 2 {
        return;
    }
    let point = if at_start {
        points[0]
    } else {
        points[count - 1]
    };
    let neighbor = if at_start {
        points[1]
    } else {
        points[count - 2]
    };
    let mut tangent = Vec3::new(point.x - neighbor.x, 0.0, point.z - neighbor.z);
    if tangent.length_squared() < 1e-8 {
        tangent = Vec3::X;
    }
    let tangent = tangent.normalize_or_zero();
    let binormal = Vec3::new(-tangent.z, 0.0, tangent.x).normalize_or_zero();
    let near_left = point - binormal * OUTER_EDGE_RADIUS;
    let near_right = point + binormal * OUTER_EDGE_RADIUS;
    let far_left = point + tangent * LOOSE_END_CAP_LENGTH - binormal * OUTER_EDGE_RADIUS;
    let far_right = point + tangent * LOOSE_END_CAP_LENGTH + binormal * OUTER_EDGE_RADIUS;
    let lift = |position: Vec3| {
        let y = terrain_height(position.x, position.z) + SURFACE_HEIGHT_OFFSET + 0.01;
        [position.x, y, position.z]
    };
    push_quad(
        buffers,
        [
            lift(near_left),
            lift(near_right),
            lift(far_right),
            lift(far_left),
        ],
        [0.0, 1.0, 0.0],
    );
}

fn validate_loose_end_treatments(project: &TrackProject) -> Result<(), String> {
    for section in &project.sections {
        if section.closed {
            continue;
        }
        for point_id in [section.points.first(), section.points.last()]
            .into_iter()
            .flatten()
        {
            let Some(point) = project.point_by_id(point_id) else {
                continue;
            };
            if point.junction.is_some() {
                continue;
            }
            let treatment = loose_end_treatment(point);
            if treatment != PointEnd::Cap {
                return Err(format!(
                    "loose end '{}' uses an unsupported treatment; only 'cap' is supported",
                    point.id
                ));
            }
        }
    }
    Ok(())
}

fn bake_extra_sections(
    project: &TrackProject,
    layout: &LayoutProject,
) -> Result<BakedExtraSections, String> {
    let route_sections = layout
        .route
        .iter()
        .map(|leg| leg.section_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut ribbon = TrackRibbon {
        buffers: empty_buffers(),
        surface_ids: Vec::new(),
    };
    let mut caps = empty_buffers();
    for section in &project.sections {
        if route_sections.contains(section.id.as_str()) {
            continue;
        }
        let points = sample_extra_section(project, section)?;
        append_track_ribbon(&mut ribbon, ribbon_from_points(&points, section.closed));
        if section.closed {
            continue;
        }
        for (at_start, point_id) in [
            (true, section.points.first()),
            (false, section.points.last()),
        ] {
            let Some(point_id) = point_id else {
                continue;
            };
            let Some(point) = project.point_by_id(point_id) else {
                continue;
            };
            if point.junction.is_some() {
                continue;
            }
            if loose_end_treatment(point) == PointEnd::Cap {
                append_loose_end_cap(&mut caps, &points, at_start);
            }
        }
    }
    Ok(BakedExtraSections { ribbon, caps })
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

    fn kerb_decoration(
        id: &str,
        section_id: &str,
        entry: f32,
        exit: f32,
        side: i8,
        elements: Vec<f32>,
    ) -> DecorationPath {
        DecorationPath {
            id: id.to_string(),
            kind: DecorationKind::Kerb,
            side,
            lane: None,
            chain: vec![DecorationLeg {
                section_id: section_id.to_string(),
                entry_fraction: entry,
                exit_fraction: exit,
                wrap: false,
            }],
            elements,
            phase: 0.0,
            prop_kind: None,
            spacing: None,
        }
    }

    fn prop_decoration(
        id: &str,
        section_id: &str,
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
            chain: vec![DecorationLeg {
                section_id: section_id.to_string(),
                entry_fraction: 0.0,
                exit_fraction: 1.0,
                wrap: false,
            }],
            elements: Vec::new(),
            phase,
            prop_kind: Some(kind),
            spacing: Some(spacing),
        }
    }

    fn chain_leg(section_id: &str, entry: f32, exit: f32, wrap: bool) -> DecorationLeg {
        DecorationLeg {
            section_id: section_id.to_string(),
            entry_fraction: entry,
            exit_fraction: exit,
            wrap,
        }
    }

    fn test_point(id: &str, position: [f32; 2]) -> Point {
        Point {
            id: id.to_string(),
            position,
            kind: PointKind::Handle,
            junction: None,
            end: None,
        }
    }

    /// Two open sections that close into one loop: A from p0 to p2, B from p2
    /// back to p0.
    fn two_section_loop_project() -> TrackProject {
        TrackProject {
            schema: TRACK_PROJECT_SCHEMA.to_string(),
            name: "Two Section Loop".to_string(),
            settings: VenueSettings::default(),
            points: vec![
                test_point("p0", [0.0, 0.0]),
                test_point("p1", [20.0, 0.0]),
                test_point("p2", [40.0, 0.0]),
                test_point("p3", [40.0, 20.0]),
                test_point("p4", [0.0, 20.0]),
            ],
            sections: vec![
                Section {
                    id: "section-a".to_string(),
                    closed: false,
                    points: vec!["p0".to_string(), "p1".to_string(), "p2".to_string()],
                },
                Section {
                    id: "section-b".to_string(),
                    closed: false,
                    points: vec![
                        "p2".to_string(),
                        "p3".to_string(),
                        "p4".to_string(),
                        "p0".to_string(),
                    ],
                },
            ],
            layouts: vec![LayoutProject {
                id: "two-section".to_string(),
                name: "Two Section Loop".to_string(),
                route: vec![
                    Leg {
                        section_id: "section-a".to_string(),
                        entry: Anchor::Point("p0".to_string()),
                        exit: Anchor::Point("p2".to_string()),
                        wrap: false,
                    },
                    Leg {
                        section_id: "section-b".to_string(),
                        entry: Anchor::Point("p2".to_string()),
                        exit: Anchor::Point("p0".to_string()),
                        wrap: false,
                    },
                ],
                start_finish: "p0".to_string(),
                spawns: Vec::new(),
                sectors: Vec::new(),
                cameras: Vec::new(),
            }],
            paths: Vec::new(),
            decorations: Vec::new(),
        }
    }

    /// A three-section closed loop whose first route leg is a partial section-A
    /// leg (p0 to p1) rather than the whole section (p0 to p2).
    fn partial_route_project() -> TrackProject {
        let mut project = TrackProject {
            schema: TRACK_PROJECT_SCHEMA.to_string(),
            name: "Partial Route Loop".to_string(),
            settings: VenueSettings::default(),
            points: vec![
                test_point("p0", [0.0, 0.0]),
                test_point("p1", [20.0, 0.0]),
                test_point("p2", [40.0, 0.0]),
                test_point("p3", [40.0, 20.0]),
                test_point("p4", [0.0, 20.0]),
            ],
            sections: vec![
                Section {
                    id: "section-a".to_string(),
                    closed: false,
                    points: vec!["p0".to_string(), "p1".to_string(), "p2".to_string()],
                },
                Section {
                    id: "section-b".to_string(),
                    closed: false,
                    points: vec!["p1".to_string(), "p3".to_string(), "p2".to_string()],
                },
                Section {
                    id: "section-c".to_string(),
                    closed: false,
                    points: vec!["p2".to_string(), "p4".to_string(), "p0".to_string()],
                },
            ],
            layouts: vec![LayoutProject {
                id: "partial-route".to_string(),
                name: "Partial Route Loop".to_string(),
                route: vec![
                    Leg {
                        section_id: "section-a".to_string(),
                        entry: Anchor::Point("p0".to_string()),
                        exit: Anchor::Point("p1".to_string()),
                        wrap: false,
                    },
                    Leg {
                        section_id: "section-b".to_string(),
                        entry: Anchor::Point("p1".to_string()),
                        exit: Anchor::Point("p2".to_string()),
                        wrap: false,
                    },
                    Leg {
                        section_id: "section-c".to_string(),
                        entry: Anchor::Point("p2".to_string()),
                        exit: Anchor::Point("p0".to_string()),
                        wrap: false,
                    },
                ],
                start_finish: "p0".to_string(),
                spawns: Vec::new(),
                sectors: Vec::new(),
                cameras: Vec::new(),
            }],
            paths: Vec::new(),
            decorations: Vec::new(),
        };
        project
            .points
            .iter_mut()
            .find(|point| point.id == "p1")
            .expect("p1")
            .kind = PointKind::Anchor;
        project
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
        assert_eq!(expected.points.len(), CONTROL_POINTS.len());
        assert_eq!(expected.sections.len(), 1);
        assert!(expected.sections[0].closed);
        assert_eq!(expected.sections[0].points.len(), CONTROL_POINTS.len());
        assert_eq!(expected.layouts.len(), 1);
        assert_eq!(expected.layouts[0].route.len(), 1);
        assert!(expected.paths.is_empty());
        assert!(expected.decorations.is_empty());
    }

    #[test]
    fn default_project_is_one_closed_section_with_no_junctions() {
        let project = TrackProject::default_project()
            .normalize()
            .expect("normalize");
        assert!(project.points.iter().all(|point| point.junction.is_none()));
        assert_eq!(
            project
                .points
                .iter()
                .filter(|point| point.kind == PointKind::Anchor)
                .count(),
            0,
            "the default loop must have no anchors"
        );
        assert_eq!(
            project
                .points
                .iter()
                .filter(|point| point.kind == PointKind::Handle)
                .count(),
            CONTROL_POINTS.len()
        );
        assert_eq!(project.sections.len(), 1);
        assert!(project.sections[0].closed);
        let leg = &project.layouts[0].route[0];
        assert_eq!(leg.section_id, "section-0");
        assert_eq!(leg.entry, Anchor::Seam);
        assert_eq!(leg.exit, Anchor::Seam);
        assert!(leg.wrap);
        assert_eq!(project.layouts[0].start_finish, "point-0");
        assert_eq!(
            project
                .point_by_id("point-0")
                .expect("start finish point")
                .kind,
            PointKind::Handle
        );
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
    fn start_finish_handle_is_off_the_curve_with_its_contact_on_the_route() {
        let project = TrackProject::default_project()
            .normalize()
            .expect("normalize");
        let layout = &project.layouts[0];
        let handle = project
            .point_by_id(&layout.start_finish)
            .expect("start finish point");
        assert_eq!(handle.kind, PointKind::Handle);
        let handle_position = Vec3::new(handle.position[0], 0.0, handle.position[1]);
        let xz_distance = |point: &Vec3| {
            ((point.x - handle_position.x).powi(2) + (point.z - handle_position.z).powi(2)).sqrt()
        };

        let (points, start_finish) = sample_layout_loop(&project, layout).expect("loop");
        let nearest = points.iter().map(xz_distance).fold(f32::INFINITY, f32::min);
        assert!(
            nearest > 1.0,
            "start/finish handle should sit more than 1 m off the route, closest was {nearest} m"
        );

        assert!(start_finish.is_finite());
        assert!((0.0..1.0).contains(&start_finish));

        let contact_index = (start_finish * points.len() as f32).round() as usize % points.len();
        let contact = points[contact_index];
        let contact_distance = xz_distance(&contact);
        assert!(
            (contact_distance - nearest).abs() < 1e-4,
            "contact station {contact_index} is {contact_distance} m from the handle, nearest is {nearest} m"
        );

        let section = project.section_by_id("section-0").expect("section");
        assert!(section_anchor_indices(&project, section).is_empty());
        let section_start = sample_section_at(&project, section, 0.0).expect("start");
        let section_end = sample_section_at(&project, section, 1.0).expect("end");
        assert!(
            section_start.distance(section_end) < 1e-6,
            "periodic seam mismatch: {section_start:?} vs {section_end:?}"
        );
    }

    #[test]
    fn handles_may_sit_off_the_curve() {
        let project = TrackProject::default_project()
            .normalize()
            .expect("normalize");
        let section = project.section_by_id("section-0").expect("section");
        let samples = (0..200)
            .map(|index| {
                sample_section_at(&project, section, index as f32 / 200.0).expect("sample")
            })
            .collect::<Vec<_>>();
        let minimum_handle_distance = project
            .points
            .iter()
            .filter(|point| point.kind == PointKind::Handle)
            .map(|handle| {
                let position = Vec3::new(handle.position[0], 0.0, handle.position[1]);
                samples
                    .iter()
                    .map(|sample| sample.distance(position))
                    .fold(f32::INFINITY, f32::min)
            })
            .fold(f32::INFINITY, f32::min);
        assert!(
            minimum_handle_distance > 1.0,
            "expected a default handle more than 1 m off the curve, closest was {minimum_handle_distance} m"
        );
    }

    #[test]
    fn periodic_closed_section_without_anchors_is_deterministic() {
        let control_points = [
            [-30.0, -20.0],
            [0.0, -35.0],
            [30.0, -25.0],
            [40.0, 0.0],
            [0.0, 25.0],
            [-40.0, 5.0],
        ];
        let mut project = TrackProject::ring_project("periodic", "periodic", &control_points);
        for point in &mut project.points {
            point.kind = PointKind::Handle;
        }
        let section = project.section_by_id("section-0").expect("section");
        assert!(section_anchor_indices(&project, section).is_empty());

        let start = sample_section_at(&project, section, 0.0).expect("start");
        let end = sample_section_at(&project, section, 1.0).expect("end");
        assert!(
            start.distance(end) < 1e-6,
            "periodic seam mismatch: {start:?} vs {end:?}"
        );
        for index in 0..=20 {
            let parameter = index as f32 / 20.0;
            let first = sample_section_at(&project, section, parameter).expect("first");
            let second = sample_section_at(&project, section, parameter).expect("second");
            assert_eq!(first, second);
        }
    }

    #[test]
    fn route_and_section_centerlines_overlap() {
        let project = TrackProject::default_project()
            .normalize()
            .expect("normalize");
        let route = layout_centerline(&project, &project.layouts[0]).expect("layout centerline");
        let section = project.section_by_id("section-0").expect("section");
        assert_eq!(route.len(), SEGMENTS);
        let mut max_distance = 0.0f32;
        for (index, route_point) in route.iter().enumerate() {
            let parameter = index as f32 / SEGMENTS as f32;
            let mut sampled = sample_section_at(&project, section, parameter).expect("sample");
            sampled.y = terrain_height(sampled.x, sampled.z);
            max_distance = max_distance.max(sampled.distance(Vec3::from_array(*route_point)));
        }
        assert!(
            max_distance < 1e-3,
            "route and section centerlines diverge by {max_distance} m"
        );
    }

    #[test]
    fn graph_validation_rejects_dangling_and_bad_routes() {
        let mut dangling = TrackProject::default_project();
        dangling.sections[0].points[0] = "missing".to_string();
        assert!(dangling.normalize().is_err());

        let mut open = TrackProject::default_project();
        open.layouts[0].route.clear();
        assert!(open.normalize().is_err());

        let mut bad_start = TrackProject::default_project();
        bad_start.layouts[0].start_finish = "missing".to_string();
        assert!(bad_start.normalize().is_err());

        let mut bad_route = TrackProject::default_project();
        bad_route.layouts[0].route[0].exit = Anchor::Point("point-2".to_string());
        assert!(bad_route.normalize().is_err());

        let mut duplicate = TrackProject::default_project();
        duplicate.points[1].id = duplicate.points[0].id.clone();
        assert!(duplicate.normalize().is_err());
    }

    #[test]
    fn graph_metadata_and_start_finish_come_from_the_layout() {
        let mut project = TrackProject::default_project();
        project.layouts[0].name = "Venue Loop".to_string();
        project
            .set_start_finish("procedural-drift-loop", "point-3")
            .expect("start finish");
        let track = track_from_project_with_default(&project, None).expect("graph track");
        assert_eq!(track.metadata.name, "Venue Loop");
        assert_eq!(track.manifest.id, "procedural-drift-loop");
        let expected = layout_centerline(&project, &project.layouts[0]).expect("centerline");
        let spawn = &track.metadata.spawns[0];
        assert!(expected.iter().any(|point| Vec3::from_array(*point)
            .distance(Vec3::from_array(spawn.position))
            < 1.0));
    }

    #[test]
    fn shared_points_round_trip_and_move_every_referencing_section() {
        let mut project = TrackProject::default_project();
        project.points.push(Point {
            id: "point-branch".to_string(),
            position: [0.0, 0.0],
            kind: PointKind::Handle,
            junction: None,
            end: Some(PointEnd::Cap),
        });
        project.sections.push(Section {
            id: "section-branch".to_string(),
            closed: false,
            points: vec!["point-0".to_string(), "point-branch".to_string()],
        });
        project.points[0].junction = Some(PointJunction {
            kind: JunctionKind::Normal,
        });
        project = project.normalize().expect("shared point project");
        let json = project.to_json().expect("json");
        let parsed = TrackProject::from_json(&json).expect("parse");
        assert_eq!(parsed.points.len(), CONTROL_POINTS.len() + 1);
        assert_eq!(
            parsed
                .sections
                .iter()
                .filter(|section| section.points.iter().any(|id| id == "point-0"))
                .count(),
            2
        );

        let mut moved = parsed;
        moved.move_point("point-0", 12.0, -9.0).expect("move");
        assert_eq!(
            moved.section_by_id("section-0").expect("section").points[0],
            "point-0"
        );
        assert_eq!(
            moved
                .section_by_id("section-branch")
                .expect("branch")
                .points[0],
            "point-0"
        );
        assert_eq!(
            moved.point_by_id("point-0").expect("point").position,
            [12.0, -9.0]
        );
    }

    #[test]
    fn junction_roles_require_two_section_memberships() {
        let mut project = TrackProject::default_project();
        project.points[0].junction = Some(PointJunction {
            kind: JunctionKind::Fork,
        });
        assert!(project.normalize().is_err());
    }

    #[test]
    fn seams_are_parameter_origins_and_open_sections_have_no_seam() {
        let project = TrackProject::default_project()
            .normalize()
            .expect("normalize");
        let section = project.section_by_id("section-0").expect("section");
        assert_eq!(
            anchor_parameter(&project, section, &Anchor::Seam).expect("seam"),
            0.0
        );
        assert_eq!(
            anchor_parameter(&project, section, &Anchor::Point("point-0".to_string()))
                .expect("point"),
            0.0
        );
        let (points, start_finish) =
            sample_layout_loop(&project, &project.layouts[0]).expect("loop");
        assert!(start_finish.is_finite());
        assert!((0.0..1.0).contains(&start_finish));
        let nearest =
            layout_start_fraction(&project, &project.layouts[0], &points).expect("nearest contact");
        assert_eq!(start_finish, nearest);

        let mut open = project.clone();
        open.sections[0].closed = false;
        assert!(open.normalize().is_err());
    }

    #[test]
    fn closed_sections_require_three_points_and_open_sections_two() {
        let mut closed = TrackProject::default_project();
        closed.points.truncate(2);
        closed.sections[0].points.truncate(2);
        assert!(closed.normalize().is_err());

        let mut open = TrackProject::default_project();
        open.points.push(Point {
            id: "point-loose".to_string(),
            position: [40.0, 40.0],
            kind: PointKind::Handle,
            junction: None,
            end: Some(PointEnd::Cap),
        });
        open.sections.push(Section {
            id: "section-loose".to_string(),
            closed: false,
            points: vec!["point-4".to_string(), "point-loose".to_string()],
        });
        let open = open.normalize().expect("loose end is valid");
        let centerline = open
            .section_centerline("section-loose")
            .expect("loose centerline");
        assert_eq!(centerline.len(), SEGMENTS + 1);
    }

    #[test]
    fn loose_end_sections_are_accepted_without_junction_roles() {
        let mut project = TrackProject::default_project();
        project.points.push(Point {
            id: "point-loose".to_string(),
            position: [40.0, 40.0],
            kind: PointKind::Handle,
            junction: None,
            end: Some(PointEnd::Cap),
        });
        project.sections.push(Section {
            id: "section-loose".to_string(),
            closed: false,
            points: vec!["point-4".to_string(), "point-loose".to_string()],
        });
        project = project.normalize().expect("loose end project");
        assert!(project.point_by_id("point-loose").is_some());
        assert!(project
            .point_by_id("point-loose")
            .expect("point")
            .junction
            .is_none());
        track_from_project_with_default(&project, None).expect("bake still works");
    }

    #[test]
    fn default_loop_bakes_packs_and_is_deterministic() {
        let project = TrackProject::default_project();
        let track = track_from_project_with_default(&project, None).expect("bake");
        assert!(!track.visual.meshes.is_empty());
        assert!(track.collision.surface_ids.contains(&TRACK_SURFACE));
        let bytes = pack_from_project(&project).expect("pack");
        let pack = sim_asset::Pack::from_bytes(&bytes).expect("parse pack");
        assert_eq!(pack.kind(), sim_types::AssetKind::Track);
        assert!(pack.chunk_bytes(sim_types::ChunkKind::Visual).is_ok());
        assert!(pack.chunk_bytes(sim_types::ChunkKind::Collision).is_ok());
        assert!(pack.chunk_bytes(sim_types::ChunkKind::Metadata).is_ok());
        let again = track_from_project_with_default(&project, None).expect("bake again");
        assert_eq!(track.visual, again.visual);
        assert_eq!(track.collision, again.collision);
        assert_eq!(track.metadata, again.metadata);
    }

    #[test]
    fn decoration_chains_bake_across_section_fractions() {
        let mut project = TrackProject::default_project();
        let mut decoration = kerb_decoration("kerb-a", "section-0", 0.0, 1.0, 1, vec![1.5, 2.5]);
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
        assert!(
            run.end - run.start > SEGMENTS as f32 * 0.9,
            "expected the run to cover the loop"
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
    fn decoration_props_keep_spacing_and_phase_across_sections() {
        let mut project = TrackProject::default_project();
        project.decorations.push(prop_decoration(
            "props-a",
            "section-0",
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
        assert!(prop_placements(&points, line).len() > 2);
        // On a curved offset path the Euclidean gap between stations is
        // shorter than the spacing measured along the path, so verify spacing
        // on a synthetic straight centreline instead. Cross-section and
        // seam-continuous spacing is covered by the deferred Step 0b
        // decoration work.
        let straight = (0..=21)
            .map(|index| Vec3::new(index as f32, 0.0, 0.0))
            .collect::<Vec<_>>();
        let straight_line = PropLine {
            // Skip the boundary stations: `track_frame` wraps the neighbouring
            // points on an open centerline, which would tilt the first and last
            // binormals. The interior stations are exactly collinear.
            start: 1.0,
            end: (straight.len() - 2) as f32,
            side: line.side,
            offset: line.offset,
            kind: PropKind::Trees,
            spacing: Some(6.0),
            phase: 2.0,
        };
        let placements = prop_placements(&straight, &straight_line);
        assert!(placements.len() > 2);
        for pair in placements.windows(2) {
            let distance = pair[0].0.distance(pair[1].0);
            assert!(
                (distance - 6.0).abs() < 0.05,
                "expected 6 m prop spacing on a straight centreline, got {distance}"
            );
        }
    }

    #[test]
    fn overlapping_decorations_are_rejected_only_on_the_same_lane() {
        let mut project = TrackProject::default_project();
        project.decorations.push(kerb_decoration(
            "kerb-a",
            "section-0",
            0.1,
            0.2,
            1,
            Vec::new(),
        ));
        project.decorations.push(kerb_decoration(
            "kerb-b",
            "section-0",
            0.15,
            0.25,
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
            "section-0",
            0.8,
            0.2,
            1,
            Vec::new(),
        ));
        assert!(project.normalize().is_err());
    }

    #[test]
    fn decoration_cross_section_chains_require_shared_points() {
        let mut project = TrackProject::default_project();
        project.points.push(Point {
            id: "point-extra".to_string(),
            position: [10.0, 10.0],
            kind: PointKind::Handle,
            junction: None,
            end: None,
        });
        project.sections.push(Section {
            id: "section-extra".to_string(),
            closed: false,
            points: vec!["point-0".to_string(), "point-extra".to_string()],
        });
        project.decorations.push(DecorationPath {
            id: "kerb-chain".to_string(),
            kind: DecorationKind::Kerb,
            side: 1,
            lane: None,
            chain: vec![
                DecorationLeg {
                    section_id: "section-extra".to_string(),
                    entry_fraction: 0.0,
                    exit_fraction: 1.0,
                    wrap: false,
                },
                DecorationLeg {
                    section_id: "section-0".to_string(),
                    entry_fraction: 0.0,
                    exit_fraction: 1.0,
                    wrap: false,
                },
            ],
            elements: Vec::new(),
            phase: 0.0,
            prop_kind: None,
            spacing: None,
        });
        assert!(project.clone().normalize().is_err());

        project.decorations[0].chain[0].section_id = "section-0".to_string();
        project.decorations[0].chain[1].section_id = "section-extra".to_string();
        assert!(project.normalize().is_ok());
    }

    #[test]
    fn cross_section_chain_keeps_kerb_and_prop_phase() {
        let mut project = two_section_loop_project();
        project.decorations.push(DecorationPath {
            id: "kerb-chain".to_string(),
            kind: DecorationKind::Kerb,
            side: 1,
            lane: None,
            chain: vec![
                chain_leg("section-a", 0.0, 1.0, false),
                chain_leg("section-b", 0.0, 1.0, false),
            ],
            elements: vec![1.5, 2.5],
            phase: 0.4,
            prop_kind: None,
            spacing: None,
        });
        project.decorations.push(DecorationPath {
            id: "prop-chain".to_string(),
            kind: DecorationKind::Prop,
            side: -1,
            lane: Some("far".to_string()),
            chain: vec![
                chain_leg("section-a", 0.0, 1.0, false),
                chain_leg("section-b", 0.0, 1.0, false),
            ],
            elements: Vec::new(),
            phase: 1.25,
            prop_kind: Some(PropKind::Trees),
            spacing: Some(6.0),
        });
        let project = project.normalize().expect("normalize project");
        let (points, _) = sample_layout_loop(&project, &project.layouts[0]).expect("points");
        let baked = bake_route_decorations(&project, &project.layouts[0], &points)
            .expect("bake decorations");

        assert!(baked.has_kerbs);
        assert_eq!(
            baked.kerb_runs.len(),
            1,
            "a cross-section chain must bake as one continuous run"
        );
        let run = &baked.kerb_runs[0];
        assert!(run.start.abs() < 1e-3);
        assert!((run.end - points.len() as f32).abs() < 1e-3);
        assert_eq!(run.elements, vec![1.5, 2.5]);
        assert!((run.phase - 0.4).abs() < 1e-6);
        let mut red = empty_buffers();
        let mut white = empty_buffers();
        append_decorated_kerbs(&points, &baked.kerb_runs, &mut red, &mut white);
        assert!(!red.positions.is_empty());
        assert!(!white.positions.is_empty());

        assert!(baked.has_props);
        assert_eq!(
            baked.prop_lines.len(),
            1,
            "a cross-section chain must bake as one continuous prop line"
        );
        let line = &baked.prop_lines[0];
        assert!(line.start.abs() < 1e-3);
        assert!((line.end - points.len() as f32).abs() < 1e-3);
        assert_eq!(line.spacing, Some(6.0));
        assert!((line.phase - 1.25).abs() < 1e-6);
        assert!(prop_placements(&points, line).len() > 4);
    }

    #[test]
    fn seam_wrapped_chain_keeps_one_continuous_phase() {
        let mut project = TrackProject::default_project();
        project.decorations.push(DecorationPath {
            id: "kerb-seam".to_string(),
            kind: DecorationKind::Kerb,
            side: 1,
            lane: None,
            chain: vec![chain_leg("section-0", 0.8, 0.3, true)],
            elements: vec![1.5, 2.5],
            phase: 0.4,
            prop_kind: None,
            spacing: None,
        });
        let project = project.normalize().expect("normalize project");
        let (points, _) = sample_layout_loop(&project, &project.layouts[0]).expect("points");
        let baked = bake_route_decorations(&project, &project.layouts[0], &points)
            .expect("bake decorations");

        assert!(baked.has_kerbs);
        assert_eq!(
            baked.kerb_runs.len(),
            2,
            "the seam run wraps at the route boundary"
        );
        let limit = points.len() as f32;
        let first = &baked.kerb_runs[0];
        let second = &baked.kerb_runs[1];
        assert!((first.start - 0.8 * limit).abs() < 1e-3);
        assert!((first.end - limit).abs() < 1e-3);
        assert!(second.start.abs() < 1e-3);
        assert!((second.end - 0.3 * limit).abs() < 1e-3);
        let arc = RouteArc::new(&points);
        let first_length = arc.distance_at_station(limit) - arc.distance_at_station(first.start);
        assert!(
            (second.phase - (first.phase + first_length)).abs() < 1e-4,
            "phase must continue across the wrapped seam"
        );
        assert_eq!(second.elements, first.elements);
    }

    #[test]
    fn partial_route_leg_samples_only_its_range() {
        let project = partial_route_project()
            .normalize()
            .expect("normalize project");
        let layout = &project.layouts[0];
        let spans = route_leg_spans(&project, layout).expect("route spans");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].section.id, "section-a");
        assert!((spans[0].end - spans[0].start - 0.5).abs() < 1e-5);
        let total = spans.iter().map(|span| span.end - span.start).sum::<f32>();
        assert!((total - 2.5).abs() < 1e-5);

        let (points, _) = sample_layout_loop(&project, layout).expect("loop");
        let mut accumulated = 0.0;
        let mut span_index = 0;
        let mut last_partial_parameter = 0.0;
        for (index, route_point) in points.iter().enumerate() {
            let route_parameter = index as f32 / SEGMENTS as f32 * total;
            while span_index + 1 < spans.len() {
                let length = spans[span_index].end - spans[span_index].start;
                if route_parameter < accumulated + length - 1e-6 {
                    break;
                }
                accumulated += length;
                span_index += 1;
            }
            let span = &spans[span_index];
            let length = span.end - span.start;
            let local = ((route_parameter - accumulated) / length).clamp(0.0, 0.999_999);
            let parameter = span.start + length * local;
            let mut expected =
                sample_section_at(&project, span.section, parameter).expect("sample");
            expected.y = terrain_height(expected.x, expected.z);
            assert!(
                expected.distance(*route_point) < 1e-4,
                "station {index} ({:?}) does not match section '{}' at {parameter}",
                route_point,
                span.section.id
            );
            if span.section.id == "section-a" {
                last_partial_parameter = parameter;
            }
        }
        assert!(
            last_partial_parameter < 0.5,
            "the partial route leg must not sample the rest of section A"
        );
        assert_eq!(spans[0].section.id, "section-a");
        assert_eq!(spans[1].section.id, "section-b");
        assert_eq!(spans[2].section.id, "section-c");
    }

    #[test]
    fn repeated_section_legs_map_to_the_correct_occurrence() {
        let mut project = two_section_loop_project();
        project.layouts[0].route = vec![
            Leg {
                section_id: "section-a".to_string(),
                entry: Anchor::Point("p0".to_string()),
                exit: Anchor::Point("p2".to_string()),
                wrap: false,
            },
            Leg {
                section_id: "section-b".to_string(),
                entry: Anchor::Point("p2".to_string()),
                exit: Anchor::Point("p0".to_string()),
                wrap: false,
            },
            Leg {
                section_id: "section-a".to_string(),
                entry: Anchor::Point("p0".to_string()),
                exit: Anchor::Point("p2".to_string()),
                wrap: false,
            },
            Leg {
                section_id: "section-b".to_string(),
                entry: Anchor::Point("p2".to_string()),
                exit: Anchor::Point("p0".to_string()),
                wrap: false,
            },
        ];
        project.decorations.push(DecorationPath {
            id: "kerb-repeat".to_string(),
            kind: DecorationKind::Kerb,
            side: 1,
            lane: None,
            chain: vec![
                chain_leg("section-a", 0.0, 1.0, false),
                chain_leg("section-b", 0.0, 1.0, false),
                chain_leg("section-a", 0.0, 1.0, false),
            ],
            elements: vec![1.5, 2.5],
            phase: 0.0,
            prop_kind: None,
            spacing: None,
        });
        let project = project.normalize().expect("normalize project");
        let (points, _) = sample_layout_loop(&project, &project.layouts[0]).expect("points");
        let baked = bake_route_decorations(&project, &project.layouts[0], &points)
            .expect("bake decorations");
        assert_eq!(baked.kerb_runs.len(), 1);
        let run = &baked.kerb_runs[0];
        let limit = points.len() as f32;
        assert!(run.start.abs() < 1e-3);
        assert!(
            (run.end - 0.75 * limit).abs() < 1e-3,
            "the chain must cover the first three route legs, not restart at the first occurrence"
        );
    }

    #[test]
    fn non_contiguous_decoration_chain_is_rejected() {
        let mut project = partial_route_project();
        project.decorations.push(DecorationPath {
            id: "kerb-skip".to_string(),
            kind: DecorationKind::Kerb,
            side: 1,
            lane: None,
            // A and C share a point, so validation accepts the chain, but the
            // active route visits B between them.
            chain: vec![
                chain_leg("section-a", 0.0, 1.0, false),
                chain_leg("section-c", 0.0, 1.0, false),
            ],
            elements: Vec::new(),
            phase: 0.0,
            prop_kind: None,
            spacing: None,
        });
        let project = project.normalize().expect("normalize project");
        let (points, _) = sample_layout_loop(&project, &project.layouts[0]).expect("points");
        let error = match bake_route_decorations(&project, &project.layouts[0], &points) {
            Ok(_) => panic!("a chain that skips route legs must fail"),
            Err(error) => error,
        };
        assert!(error.contains("kerb-skip"), "unexpected error: {error}");
        assert!(
            error.contains("not contiguous"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn decoration_chain_outside_the_active_route_is_rejected() {
        let mut project = two_section_loop_project();
        project.points.push(test_point("p5", [10.0, 30.0]));
        project.points.push(test_point("p6", [10.0, 40.0]));
        project.sections.push(Section {
            id: "section-off-route".to_string(),
            closed: false,
            points: vec!["p5".to_string(), "p6".to_string()],
        });
        project.decorations.push(kerb_decoration(
            "kerb-off-route",
            "section-off-route",
            0.0,
            1.0,
            1,
            Vec::new(),
        ));
        let project = project.normalize().expect("normalize project");
        let (points, _) = sample_layout_loop(&project, &project.layouts[0]).expect("points");
        let error = match bake_route_decorations(&project, &project.layouts[0], &points) {
            Ok(_) => panic!("a chain outside the route must fail"),
            Err(error) => error,
        };
        assert!(
            error.contains("kerb-off-route"),
            "unexpected error: {error}"
        );
        assert!(
            error.contains("outside the active route"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn cross_section_chain_bake_is_deterministic() {
        let mut project = two_section_loop_project();
        project.decorations.push(DecorationPath {
            id: "kerb-chain".to_string(),
            kind: DecorationKind::Kerb,
            side: 1,
            lane: None,
            chain: vec![
                chain_leg("section-a", 0.0, 1.0, false),
                chain_leg("section-b", 0.0, 1.0, false),
            ],
            elements: vec![1.5, 2.5],
            phase: 0.4,
            prop_kind: None,
            spacing: None,
        });
        project.decorations.push(DecorationPath {
            id: "prop-chain".to_string(),
            kind: DecorationKind::Prop,
            side: 1,
            lane: Some("near".to_string()),
            chain: vec![
                chain_leg("section-a", 0.0, 1.0, false),
                chain_leg("section-b", 0.0, 1.0, false),
            ],
            elements: Vec::new(),
            phase: 0.5,
            prop_kind: Some(PropKind::Tyres),
            spacing: Some(2.5),
        });
        let project = project.normalize().expect("normalize project");
        let first = track_from_project_with_default(&project, None).expect("first bake");
        let second = track_from_project_with_default(&project, None).expect("second bake");
        assert_eq!(first.visual, second.visual);
        assert_eq!(first.collision, second.collision);
        assert_eq!(first.metadata, second.metadata);
        assert_eq!(first.manifest, second.manifest);
    }

    #[test]
    fn point_insert_move_delete_round_trips() {
        let mut project = TrackProject::default_project();
        let before = project.section_centerline("section-0").expect("centerline");
        let id = project
            .insert_point("section-0", 1, 15.0, -30.0)
            .expect("insert");
        assert_eq!(
            project.section_by_id("section-0").expect("section").points[2],
            id
        );
        project.move_point(&id, 16.0, -31.0).expect("move");
        assert_eq!(
            project.point_by_id(&id).expect("point").position,
            [16.0, -31.0]
        );
        project.delete_point(&id).expect("delete");
        assert!(project.point_by_id(&id).is_none());
        let after = project.section_centerline("section-0").expect("centerline");
        assert_eq!(before, after);

        assert!(project.delete_point("point-0").is_err());
    }

    #[test]
    fn shared_junction_and_start_finish_points_cannot_be_deleted() {
        let mut project = TrackProject::default_project();
        project.points.push(Point {
            id: "point-branch".to_string(),
            position: [0.0, 0.0],
            kind: PointKind::Handle,
            junction: None,
            end: None,
        });
        project.sections.push(Section {
            id: "section-branch".to_string(),
            closed: false,
            points: vec!["point-0".to_string(), "point-branch".to_string()],
        });
        project.points[0].junction = Some(PointJunction {
            kind: JunctionKind::Normal,
        });
        let mut project = project.normalize().expect("shared project");
        assert!(project.delete_point("point-0").is_err());
        assert!(project.delete_point("point-branch").is_err());
        project.layouts[0].start_finish = "point-4".to_string();
        assert!(project.delete_point("point-4").is_err());
    }

    #[test]
    fn bake_is_deterministic_after_point_edits() {
        let mut project = TrackProject::default_project();
        project
            .insert_point("section-0", 1, 4.0, -6.0)
            .expect("insert");
        project
            .move_point("point-3", 42.0, 4.0)
            .expect("move point");
        project = project.normalize().expect("normalize");

        let first = track_from_project_with_default(&project, None).expect("first bake");
        let second = track_from_project_with_default(&project, None).expect("second bake");
        assert_eq!(first.visual, second.visual);
        assert_eq!(first.collision, second.collision);
        assert_eq!(first.metadata, second.metadata);
        assert_eq!(first.manifest, second.manifest);

        let centerline = project
            .section_centerline("section-0")
            .expect("section centerline");
        assert_eq!(centerline.len(), SEGMENTS + 1);
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

        // The default loop has no truly straight section under the `@6`
        // sampling, so verify centring on a synthetic straight centreline.
        let straight = (0..=20)
            .map(|index| Vec3::new(index as f32, 0.0, 0.0))
            .collect::<Vec<_>>();
        let offsets = racing_line_offsets(&straight);
        let middle = straight.len() / 2;
        assert!(
            offsets[middle].abs() < 1.0,
            "straight offset {} should stay near centre",
            offsets[middle]
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
        for (index, side) in [-1i8, 1i8].into_iter().enumerate() {
            project.decorations.push(kerb_decoration(
                &format!("kerb-{index}"),
                "section-0",
                0.0,
                1.0,
                side,
                vec![project.settings.kerb.element_length],
            ));
        }
        project.decorations.push(DecorationPath {
            id: "props-off".to_string(),
            kind: DecorationKind::Prop,
            side: 1,
            lane: Some("far".to_string()),
            chain: vec![DecorationLeg {
                section_id: "section-0".to_string(),
                entry_fraction: 0.0,
                exit_fraction: 1.0,
                wrap: false,
            }],
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
    fn prop_spacing_is_uniform_along_offset_paths() {
        // Use a smooth synthetic loop: the default loop has a tight hairpin
        // where an inside offset path legitimately bunches, so Euclidean
        // spacing is not a valid expectation there.
        let radius = 80.0;
        let points = (0..120)
            .map(|index| {
                let angle = TAU * index as f32 / 120.0;
                Vec3::new(radius * angle.cos(), 0.0, radius * angle.sin())
            })
            .collect::<Vec<_>>();
        let count = points.len() as f32;
        for kind in [PropKind::Blocks, PropKind::Tyres, PropKind::Trees] {
            let spacing = match kind {
                PropKind::Blocks => BLOCK_SPACING,
                PropKind::Tyres => TYRE_SPACING,
                PropKind::Trees => TREE_SPACING,
                PropKind::Off => unreachable!("off lines produce no placements"),
            };
            let tolerance = if kind == PropKind::Trees { 0.15 } else { 0.03 };
            for side in [-1i8, 1i8] {
                for offset in [PROP_OFFSET, PROP_FAR_OFFSET] {
                    let line = PropLine {
                        start: 0.0,
                        end: count,
                        side,
                        offset,
                        kind,
                        spacing: None,
                        phase: 0.0,
                    };
                    let placements = prop_placements(&points, &line);
                    assert!(
                        !placements.is_empty(),
                        "line {:?} produced no placements",
                        line
                    );
                    for pair in placements.windows(2) {
                        let distance = pair[0].0.distance(pair[1].0);
                        assert!(
                            (distance - spacing).abs() <= spacing * tolerance,
                            "prop spacing drifted: expected {spacing}, got {distance}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn tree_prop_lines_build_canopies_and_trunk_collision() {
        let mut project = TrackProject::default_project();
        project.decorations.push(prop_decoration(
            "trees-a",
            "section-0",
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
    fn guide_rails_cover_default_lanes_without_duplicates() {
        let track = track_from_project_with_default(&TrackProject::default_project(), None)
            .expect("default track");
        let rails = guide_rails(&track.prop_lines);
        assert_eq!(rails.len(), 6, "kerb rails plus one per unique prop lane");

        assert_eq!(rails[0].kind, "kerb");
        assert_eq!(rails[0].side, -1);
        assert!((rails[0].offset - 6.4).abs() < 1e-4);
        assert_eq!(rails[0].lane, None);
        assert_eq!(rails[1].kind, "kerb");
        assert_eq!(rails[1].side, 1);
        assert!((rails[1].offset - 6.4).abs() < 1e-4);
        assert_eq!(rails[1].lane, None);

        let expected = [
            (-1i8, PROP_OFFSET, "near"),
            (1, PROP_OFFSET, "near"),
            (-1, PROP_FAR_OFFSET, "far"),
            (1, PROP_FAR_OFFSET, "far"),
        ];
        for (rail, (side, offset, lane)) in rails[2..].iter().zip(expected) {
            assert_eq!(rail.kind, "prop");
            assert_eq!(rail.side, side);
            assert!((rail.offset - offset).abs() < 1e-4);
            assert_eq!(rail.lane.as_deref(), Some(lane));
        }
    }

    #[test]
    fn guide_rails_use_custom_lane_offsets() {
        let mut project = TrackProject::default_project();
        project.decorations.push(prop_decoration(
            "props-custom",
            "section-0",
            1,
            "11.5",
            PropKind::Blocks,
            8.0,
            0.0,
        ));
        let track = track_from_project_with_default(&project, None).expect("track");
        let rails = guide_rails(&track.prop_lines);
        let props = rails
            .iter()
            .filter(|rail| rail.kind == "prop")
            .collect::<Vec<_>>();
        assert_eq!(props.len(), 1);
        assert_eq!(props[0].side, 1);
        assert!((props[0].offset - 11.5).abs() < 1e-4);
        assert_eq!(props[0].lane.as_deref(), Some("11.5"));
    }

    #[test]
    fn guide_rails_skip_off_prop_lines() {
        let lines = vec![PropLine {
            start: 0.0,
            end: 1.0,
            side: 1,
            offset: PROP_OFFSET,
            kind: PropKind::Off,
            spacing: None,
            phase: 0.0,
        }];
        let rails = guide_rails(&lines);
        assert_eq!(rails.len(), 2);
        assert!(rails.iter().all(|rail| rail.kind == "kerb"));
    }

    #[test]
    fn guide_rails_order_deterministically_and_serialize_camel_case() {
        let line = |side: i8, offset: f32, kind: PropKind| PropLine {
            start: 0.0,
            end: 1.0,
            side,
            offset,
            kind,
            spacing: None,
            phase: 0.0,
        };
        let lines = vec![
            line(1, PROP_FAR_OFFSET, PropKind::Trees),
            line(-1, PROP_OFFSET, PropKind::Blocks),
            line(1, PROP_OFFSET, PropKind::Tyres),
            line(-1, PROP_FAR_OFFSET, PropKind::Trees),
            line(1, PROP_OFFSET, PropKind::Blocks),
        ];
        let rails = guide_rails(&lines);
        assert_eq!(rails, guide_rails(&lines));
        let summary = rails
            .iter()
            .map(|rail| {
                (
                    rail.kind.as_str(),
                    rail.side,
                    rail.offset,
                    rail.lane.as_deref(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            summary,
            vec![
                ("kerb", -1, 6.4, None),
                ("kerb", 1, 6.4, None),
                ("prop", -1, PROP_OFFSET, Some("near")),
                ("prop", 1, PROP_OFFSET, Some("near")),
                ("prop", -1, PROP_FAR_OFFSET, Some("far")),
                ("prop", 1, PROP_FAR_OFFSET, Some("far")),
            ]
        );

        let json = serde_json::to_string(&rails).expect("serialize guide rails");
        assert!(json.contains("\"kind\":\"kerb\""));
        assert!(json.contains("\"kind\":\"prop\""));
        assert!(json.contains("\"lane\":null"));
        assert!(json.contains("\"lane\":\"near\""));
        assert!(json.contains("\"lane\":\"far\""));
        let parsed = serde_json::from_str::<serde_json::Value>(&json).expect("parse guide rails");
        let first = &parsed[0];
        assert_eq!(first["kind"], "kerb");
        assert_eq!(first["side"], -1);
        assert_eq!(first["offset"], 6.4);
        assert!(first["lane"].is_null());
        assert_eq!(parsed[2]["lane"], "near");
        assert_eq!(parsed[4]["lane"], "far");
    }

    #[test]
    fn make_junction_validates_and_never_splits_the_host_section() {
        let mut project = TrackProject::default_project();
        let host_before = project
            .section_by_id("section-0")
            .expect("host section")
            .points
            .clone();

        assert!(project.make_junction("missing").is_err());
        assert!(
            project.make_junction("point-0").is_err(),
            "start/finish points cannot become junctions"
        );

        let branch_id = project.make_junction("point-1").expect("make junction");
        assert_eq!(
            project
                .section_by_id("section-0")
                .expect("host section")
                .points,
            host_before,
            "making a junction must not split the host section"
        );
        assert_eq!(project.sections.len(), 2);
        let branch = project.section_by_id(&branch_id).expect("branch section");
        assert!(!branch.closed);
        assert_eq!(branch.points.len(), 2);
        assert_eq!(branch.points[0], "point-1");
        let loose = project
            .point_by_id(branch.points.last().expect("loose end"))
            .expect("loose point");
        assert_eq!(loose.end, Some(PointEnd::Cap));
        assert_eq!(
            project
                .point_by_id("point-1")
                .expect("junction point")
                .junction,
            Some(PointJunction {
                kind: JunctionKind::Normal
            })
        );
        assert!(
            project.make_junction("point-1").is_err(),
            "a point may only have one junction role"
        );

        let second_branch = project.make_junction("point-2").expect("second junction");
        assert_ne!(branch_id, second_branch, "branch ids must be unique");

        let mut endpoint_host = TrackProject::default_project();
        endpoint_host.points.push(Point {
            id: "point-endpoint".to_string(),
            position: [0.0, 80.0],
            kind: PointKind::Handle,
            junction: None,
            end: Some(PointEnd::Cap),
        });
        endpoint_host.sections.push(Section {
            id: "section-endpoint-host".to_string(),
            closed: false,
            points: vec!["point-endpoint".to_string(), "point-2".to_string()],
        });
        assert!(
            endpoint_host.make_junction("point-endpoint").is_err(),
            "an endpoint host is not an interior junction host"
        );
    }

    #[test]
    fn junction_contact_is_derived_on_the_host_and_moves_with_the_handle() {
        let mut project = TrackProject::default_project();
        let host_before = project
            .section_centerline("section-0")
            .expect("host before");
        let branch_id = project.make_junction("point-1").expect("make junction");
        let host_after = project.section_centerline("section-0").expect("host after");
        assert_eq!(
            host_before, host_after,
            "the host section must stay smooth and unchanged at an interior junction"
        );
        assert!(
            section_anchor_indices(&project, project.section_by_id("section-0").expect("host"))
                .is_empty(),
            "an interior junction handle must not clamp the host section"
        );
        assert_eq!(
            project.point_by_id("point-1").expect("junction").kind,
            PointKind::Handle
        );

        let contact = junction_contact(&project, "point-1")
            .expect("contact")
            .expect("interior host");
        let host_points = project.section_centerline("section-0").expect("host");
        let nearest = host_points
            .iter()
            .map(|point| {
                ((point[0] - contact.position.x).powi(2) + (point[2] - contact.position.z).powi(2))
                    .sqrt()
            })
            .fold(f32::INFINITY, f32::min);
        assert!(
            nearest < 0.5,
            "host centerline should pass near the derived contact, nearest was {nearest} m"
        );

        let branch = project.section_by_id(&branch_id).expect("branch");
        let branch_start = sample_section_at(&project, branch, 0.0).expect("branch start");
        assert!(
            branch_start.distance(contact.position) < 1e-4,
            "branch start {:?} must coincide with contact {:?}",
            branch_start,
            contact.position
        );

        let old_contact = contact.position;
        let handle = project.point_by_id("point-1").expect("handle");
        let moved = [handle.position[0] + 6.0, handle.position[1] - 4.0];
        project
            .move_point("point-1", moved[0], moved[1])
            .expect("move handle");
        let moved_contact = junction_contact(&project, "point-1")
            .expect("contact")
            .expect("interior host");
        assert!(
            moved_contact.position.distance(old_contact) > 0.1,
            "moving the handle must move the derived contact"
        );
        let branch = project
            .section_by_id(&branch_id)
            .expect("branch after move");
        let branch_start = sample_section_at(&project, branch, 0.0).expect("branch start");
        assert!(
            branch_start.distance(moved_contact.position) < 1e-4,
            "the attached branch must follow the re-derived contact"
        );
        assert_eq!(
            project.point_by_id("point-1").expect("junction").kind,
            PointKind::Handle,
            "moving a junction handle must not turn the host point into an anchor"
        );
    }

    #[test]
    fn capped_branch_bakes_cap_visual_and_collision_deterministically() {
        let mut project = TrackProject::default_project();
        let base = track_from_project_with_default(&project, None).expect("base track");
        let base_triangles = base.collision.indices.len() / 3;
        project.make_junction("point-1").expect("make junction");

        let track = track_from_project_with_default(&project, None).expect("junction track");
        let mesh = |name: &str| {
            track
                .visual
                .meshes
                .iter()
                .position(|mesh| mesh.name == name)
                .and_then(|index| track.visual.mesh_buffers.get(index))
                .unwrap_or_else(|| panic!("missing {name} mesh"))
        };
        let branch = mesh("branch_asphalt");
        assert!(!branch.positions.is_empty());
        assert!(!branch.indices.is_empty());
        let caps = mesh("loose_end_caps");
        assert!(!caps.positions.is_empty(), "loose-end cap visual missing");
        assert!(!caps.indices.is_empty(), "loose-end cap visual missing");
        assert!(
            track.collision.indices.len() / 3 > base_triangles,
            "branch and cap collision should add geometry"
        );
        let cap_vertex = Vec3::from_array(caps.positions[0]);
        assert!(
            track
                .collision
                .vertices
                .iter()
                .any(|vertex| Vec3::from_array(*vertex).distance(cap_vertex) < 1e-5),
            "cap geometry must be present in collision"
        );

        let again = track_from_project_with_default(&project, None).expect("deterministic track");
        assert_eq!(track.visual, again.visual);
        assert_eq!(track.collision, again.collision);
        assert_eq!(track.metadata, again.metadata);
        assert_eq!(track.manifest, again.manifest);
    }

    #[test]
    fn unsupported_loose_end_treatments_are_rejected() {
        for treatment in [PointEnd::Open, PointEnd::Barrier] {
            let mut project = TrackProject::default_project();
            let branch_id = project.make_junction("point-1").expect("make junction");
            let loose_id = project
                .section_by_id(&branch_id)
                .expect("branch")
                .points
                .last()
                .expect("loose end")
                .clone();
            project
                .points
                .iter_mut()
                .find(|point| point.id == loose_id)
                .expect("loose point")
                .end = Some(treatment);
            let error = match track_from_project_with_default(&project, None) {
                Ok(_) => panic!("unsupported end treatment must fail"),
                Err(error) => error,
            };
            assert!(
                error.contains("unsupported treatment"),
                "unexpected error: {error}"
            );
        }
    }
}
