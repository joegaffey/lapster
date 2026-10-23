use core::fmt;

use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize as SerdeDeserialize, Serialize as SerdeSerialize};

pub const MAGIC: [u8; 4] = *b"LSIM";
pub const SCHEMA_VERSION: u16 = 3;
pub const PACK_FORMAT_VERSION: u16 = 1;
pub const PACK_HEADER_SIZE: usize = 64;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackHeader {
    pub magic: [u8; 4],
    pub format_version: u16,
    pub header_size: u16,
    pub manifest_offset: u64,
    pub manifest_len: u64,
    pub pack_len: u64,
    pub reserved: [u8; 32],
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub enum AssetKind {
    Car,
    Track,
}

impl AssetKind {
    pub fn required_entrypoints(self) -> &'static [ChunkKind] {
        static CAR_REQUIRED: [ChunkKind; 2] = [ChunkKind::Physics, ChunkKind::Visual];
        static TRACK_REQUIRED: [ChunkKind; 3] =
            [ChunkKind::Visual, ChunkKind::Collision, ChunkKind::Metadata];
        match self {
            AssetKind::Car => &CAR_REQUIRED,
            AssetKind::Track => &TRACK_REQUIRED,
        }
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub enum ChunkKind {
    Physics,
    Visual,
    Collision,
    Metadata,
    Audio,
    Texture,
}

impl ChunkKind {
    pub const ALL: [ChunkKind; 6] = [
        ChunkKind::Physics,
        ChunkKind::Visual,
        ChunkKind::Collision,
        ChunkKind::Metadata,
        ChunkKind::Audio,
        ChunkKind::Texture,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ChunkKind::Physics => "physics",
            ChunkKind::Visual => "visual",
            ChunkKind::Collision => "collision",
            ChunkKind::Metadata => "metadata",
            ChunkKind::Audio => "audio",
            ChunkKind::Texture => "texture",
        }
    }
}

impl fmt::Display for ChunkKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub enum Compression {
    None,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub enum Handedness {
    Right,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub enum Axis {
    X,
    Y,
    Z,
    NegX,
    NegY,
    NegZ,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub enum LengthUnit {
    Meter,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub enum MassUnit {
    Kilogram,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub enum TimeUnit {
    Second,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub enum AngleUnit {
    Radian,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub struct UnitSystem {
    pub length: LengthUnit,
    pub mass: MassUnit,
    pub time: TimeUnit,
    pub angle: AngleUnit,
}

pub const ENGINE_UNITS: UnitSystem = UnitSystem {
    length: LengthUnit::Meter,
    mass: MassUnit::Kilogram,
    time: TimeUnit::Second,
    angle: AngleUnit::Radian,
};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub struct Frame {
    pub handedness: Handedness,
    pub up: Axis,
    pub forward: Axis,
}

pub const ENGINE_FRAME: Frame = Frame {
    handedness: Handedness::Right,
    up: Axis::Y,
    forward: Axis::NegZ,
};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub struct Aabb {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl Aabb {
    pub fn is_valid(&self) -> bool {
        self.min
            .iter()
            .zip(self.max.iter())
            .all(|(min, max)| min.is_finite() && max.is_finite() && min <= max)
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub struct ChunkRef {
    pub kind: ChunkKind,
    pub offset: u64,
    pub len: u64,
    pub align: u32,
    pub hash: [u8; 32],
    pub compression: Compression,
    pub schema_version: u16,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub struct EntryPoints {
    pub physics: Option<u32>,
    pub visual: Option<u32>,
    pub collision: Option<u32>,
    pub metadata: Option<u32>,
    pub audio: Option<u32>,
    pub texture: Option<u32>,
}

impl EntryPoints {
    pub fn get(&self, kind: ChunkKind) -> Option<u32> {
        match kind {
            ChunkKind::Physics => self.physics,
            ChunkKind::Visual => self.visual,
            ChunkKind::Collision => self.collision,
            ChunkKind::Metadata => self.metadata,
            ChunkKind::Audio => self.audio,
            ChunkKind::Texture => self.texture,
        }
    }

    pub fn entries(&self) -> [(ChunkKind, Option<u32>); 6] {
        [
            (ChunkKind::Physics, self.physics),
            (ChunkKind::Visual, self.visual),
            (ChunkKind::Collision, self.collision),
            (ChunkKind::Metadata, self.metadata),
            (ChunkKind::Audio, self.audio),
            (ChunkKind::Texture, self.texture),
        ]
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub struct AssetStats {
    pub vertices: u32,
    pub triangles: u32,
    pub materials: u16,
    pub textures: u16,
    pub draw_calls: u16,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub struct AssetManifest {
    pub magic: [u8; 4],
    pub schema_version: u16,
    pub kind: AssetKind,
    pub id: String,
    pub source_hash: [u8; 32],
    pub units: UnitSystem,
    pub frame: Frame,
    pub bounds: Aabb,
    pub chunks: Vec<ChunkRef>,
    pub entrypoints: EntryPoints,
    pub stats: AssetStats,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ManifestError {
    BadMagic,
    UnsupportedVersion(u16),
    EmptyId,
    EmptyChunks,
    InvalidBounds,
    InvalidAlignment {
        index: u32,
        align: u32,
    },
    MisalignedChunk {
        index: u32,
        offset: u64,
        align: u32,
    },
    EmptyChunk(u32),
    UnsupportedChunkVersion {
        index: u32,
        version: u16,
    },
    InvalidEntryPoint {
        kind: ChunkKind,
        index: u32,
    },
    WrongEntryPointKind {
        expected: ChunkKind,
        index: u32,
        actual: ChunkKind,
    },
    MissingEntryPoint(ChunkKind),
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ManifestError::BadMagic => f.write_str("asset magic does not match LSIM"),
            ManifestError::UnsupportedVersion(version) => {
                write!(f, "unsupported manifest version {version}")
            }
            ManifestError::EmptyId => f.write_str("asset id is empty"),
            ManifestError::EmptyChunks => f.write_str("manifest has no chunks"),
            ManifestError::InvalidBounds => f.write_str("asset bounds are invalid"),
            ManifestError::InvalidAlignment { index, align } => {
                write!(f, "chunk {index} has invalid alignment {align}")
            }
            ManifestError::MisalignedChunk {
                index,
                offset,
                align,
            } => write!(f, "chunk {index} offset {offset} is not aligned to {align}"),
            ManifestError::EmptyChunk(index) => write!(f, "chunk {index} is empty"),
            ManifestError::UnsupportedChunkVersion { index, version } => {
                write!(f, "chunk {index} has unsupported version {version}")
            }
            ManifestError::InvalidEntryPoint { kind, index } => {
                write!(f, "entrypoint {kind} references missing chunk {index}")
            }
            ManifestError::WrongEntryPointKind {
                expected,
                index,
                actual,
            } => write!(
                f,
                "entrypoint {expected} references chunk {index} of kind {actual}"
            ),
            ManifestError::MissingEntryPoint(kind) => {
                write!(f, "required entrypoint {kind} is missing")
            }
        }
    }
}

impl std::error::Error for ManifestError {}

impl AssetManifest {
    pub fn validate(&self) -> Result<(), ManifestError> {
        if self.magic != MAGIC {
            return Err(ManifestError::BadMagic);
        }
        if self.schema_version != SCHEMA_VERSION {
            return Err(ManifestError::UnsupportedVersion(self.schema_version));
        }
        if self.id.is_empty() {
            return Err(ManifestError::EmptyId);
        }
        if !self.bounds.is_valid() {
            return Err(ManifestError::InvalidBounds);
        }
        if self.chunks.is_empty() {
            return Err(ManifestError::EmptyChunks);
        }

        for (index, chunk) in self.chunks.iter().enumerate() {
            let index = index as u32;
            if !chunk.align.is_power_of_two() {
                return Err(ManifestError::InvalidAlignment {
                    index,
                    align: chunk.align,
                });
            }
            if chunk.offset % u64::from(chunk.align) != 0 {
                return Err(ManifestError::MisalignedChunk {
                    index,
                    offset: chunk.offset,
                    align: chunk.align,
                });
            }
            if chunk.len == 0 {
                return Err(ManifestError::EmptyChunk(index));
            }
            if chunk.schema_version != SCHEMA_VERSION {
                return Err(ManifestError::UnsupportedChunkVersion {
                    index,
                    version: chunk.schema_version,
                });
            }
        }

        for (kind, entry) in self.entrypoints.entries() {
            let Some(index) = entry else {
                continue;
            };
            let Some(chunk) = self.chunks.get(index as usize) else {
                return Err(ManifestError::InvalidEntryPoint { kind, index });
            };
            if chunk.kind != kind {
                return Err(ManifestError::WrongEntryPointKind {
                    expected: kind,
                    index,
                    actual: chunk.kind,
                });
            }
        }

        for kind in self.kind.required_entrypoints() {
            if self.entrypoints.get(*kind).is_none() {
                return Err(ManifestError::MissingEntryPoint(*kind));
            }
        }

        Ok(())
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub struct PacejkaCoefficients {
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub struct TireSpec {
    pub lateral: PacejkaCoefficients,
    pub longitudinal: PacejkaCoefficients,
    pub relaxation_length: f32,
    pub peak_slip_ratio: f32,
    pub peak_slip_angle: f32,
    pub friction_scale: f32,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub struct SuspensionSpec {
    pub travel: f32,
    pub spring_rate: f32,
    pub damper_bump: f32,
    pub damper_rebound: f32,
    pub anti_roll: f32,
    pub ride_height: f32,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub struct WheelPhysics {
    pub name: String,
    pub is_front: bool,
    pub is_left: bool,
    pub position: [f32; 3],
    pub radius: f32,
    pub width: f32,
    pub steerable: bool,
    pub driven: bool,
    pub suspension: SuspensionSpec,
    pub tire: TireSpec,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub struct TorquePoint {
    pub rpm: f32,
    pub torque_nm: f32,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub struct EngineSpec {
    pub idle_rpm: f32,
    pub redline_rpm: f32,
    pub torque_curve: Vec<TorquePoint>,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub enum DriveLayout {
    Front,
    Rear,
    All,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub struct DrivetrainSpec {
    pub gear_ratios: Vec<f32>,
    pub final_drive: f32,
    pub clutch_max_torque: f32,
    pub drive_layout: DriveLayout,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub struct BrakeSpec {
    pub max_torque: [f32; 4],
    pub bias: f32,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub struct SteeringSpec {
    pub max_angle: f32,
    pub ratio: f32,
    pub caster: f32,
    pub kingpin_inclination: f32,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub struct AeroSpec {
    pub drag_coefficient: f32,
    pub frontal_area: f32,
    pub downforce_coefficient: f32,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub struct CarPhysics {
    pub mass: f32,
    pub inertia: [f32; 3],
    pub center_of_gravity: [f32; 3],
    pub wheelbase: f32,
    pub track_width: [f32; 2],
    pub wheels: [WheelPhysics; 4],
    pub engine: EngineSpec,
    pub drivetrain: DrivetrainSpec,
    pub brakes: BrakeSpec,
    pub steering: SteeringSpec,
    pub aero: AeroSpec,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub struct NodeTransform {
    pub translation: [f32; 3],
    pub rotation_xyzw: [f32; 4],
    pub scale: [f32; 3],
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub struct MeshRef {
    pub name: String,
    pub vertex_offset: u32,
    pub vertex_count: u32,
    pub index_offset: u32,
    pub index_count: u32,
    pub material: u16,
    pub bounds: Aabb,
    pub lod: u8,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub struct MaterialRef {
    pub name: String,
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub base_color_texture: Option<u32>,
    #[serde(default)]
    pub outline: bool,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub struct TextureRef {
    pub name: String,
    pub mime: String,
    pub hash: [u8; 32],
    pub width: u32,
    pub height: u32,
    pub rgba8: Vec<u8>,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub struct MeshBuffers {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub colors: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub struct WheelVisual {
    pub name: String,
    pub mesh: u32,
    pub transform: NodeTransform,
    pub radius: f32,
    pub width: f32,
    pub is_front: bool,
    pub is_left: bool,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub struct CarVisual {
    pub meshes: Vec<MeshRef>,
    pub materials: Vec<MaterialRef>,
    pub textures: Vec<TextureRef>,
    pub mesh_buffers: Vec<MeshBuffers>,
    pub body: NodeTransform,
    pub wheels: [WheelVisual; 4],
    pub steering_wheel: Option<NodeTransform>,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub struct SpawnPoint {
    pub name: String,
    pub position: [f32; 3],
    pub rotation_xyzw: [f32; 4],
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub struct CameraSpec {
    pub name: String,
    pub position: [f32; 3],
    pub target: [f32; 3],
    pub fov_y: f32,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub struct SectorSpec {
    pub name: String,
    pub start_fraction: f32,
    pub end_fraction: f32,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub struct SurfaceSpec {
    pub id: u16,
    pub name: String,
    pub friction: f32,
    pub roughness: f32,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub struct TrackMetadata {
    pub name: String,
    pub length: f32,
    pub spawns: Vec<SpawnPoint>,
    pub pit_boxes: Vec<SpawnPoint>,
    pub cameras: Vec<CameraSpec>,
    pub sectors: Vec<SectorSpec>,
    pub surfaces: Vec<SurfaceSpec>,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub struct TrackCollision {
    pub vertices: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub surface_ids: Vec<u16>,
    pub bounds: Aabb,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub struct InstanceRef {
    pub mesh: u32,
    pub transform: NodeTransform,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[archive(check_bytes)]
pub struct TrackVisual {
    pub meshes: Vec<MeshRef>,
    pub materials: Vec<MaterialRef>,
    pub textures: Vec<TextureRef>,
    pub mesh_buffers: Vec<MeshBuffers>,
    pub instances: Vec<InstanceRef>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, offset_of, size_of};

    #[test]
    fn pack_header_layout_is_stable() {
        assert_eq!(size_of::<PackHeader>(), PACK_HEADER_SIZE);
        assert_eq!(align_of::<PackHeader>(), 8);
        assert_eq!(offset_of!(PackHeader, magic), 0);
        assert_eq!(offset_of!(PackHeader, format_version), 4);
        assert_eq!(offset_of!(PackHeader, header_size), 6);
        assert_eq!(offset_of!(PackHeader, manifest_offset), 8);
        assert_eq!(offset_of!(PackHeader, manifest_len), 16);
        assert_eq!(offset_of!(PackHeader, pack_len), 24);
        assert_eq!(offset_of!(PackHeader, reserved), 32);
    }

    fn manifest(kind: AssetKind, entrypoints: EntryPoints) -> AssetManifest {
        AssetManifest {
            magic: MAGIC,
            schema_version: SCHEMA_VERSION,
            kind,
            id: String::from("test"),
            source_hash: [0; 32],
            units: ENGINE_UNITS,
            frame: ENGINE_FRAME,
            bounds: Aabb {
                min: [-1.0, 0.0, -1.0],
                max: [1.0, 1.0, 1.0],
            },
            chunks: vec![
                ChunkRef {
                    kind: ChunkKind::Physics,
                    offset: 0,
                    len: 32,
                    align: 16,
                    hash: [0; 32],
                    compression: Compression::None,
                    schema_version: SCHEMA_VERSION,
                },
                ChunkRef {
                    kind: ChunkKind::Visual,
                    offset: 32,
                    len: 32,
                    align: 16,
                    hash: [0; 32],
                    compression: Compression::None,
                    schema_version: SCHEMA_VERSION,
                },
                ChunkRef {
                    kind: ChunkKind::Collision,
                    offset: 64,
                    len: 32,
                    align: 16,
                    hash: [0; 32],
                    compression: Compression::None,
                    schema_version: SCHEMA_VERSION,
                },
                ChunkRef {
                    kind: ChunkKind::Metadata,
                    offset: 96,
                    len: 32,
                    align: 16,
                    hash: [0; 32],
                    compression: Compression::None,
                    schema_version: SCHEMA_VERSION,
                },
            ],
            entrypoints,
            stats: AssetStats {
                vertices: 8,
                triangles: 12,
                materials: 1,
                textures: 0,
                draw_calls: 1,
            },
            warnings: Vec::new(),
        }
    }

    #[test]
    fn accepts_valid_car_manifest() {
        let mut manifest = manifest(
            AssetKind::Car,
            EntryPoints {
                physics: Some(0),
                visual: Some(1),
                collision: None,
                metadata: None,
                audio: None,
                texture: None,
            },
        );
        assert_eq!(manifest.validate(), Ok(()));
        manifest.magic = *b"NOPE";
        assert_eq!(manifest.validate(), Err(ManifestError::BadMagic));
    }

    #[test]
    fn requires_track_entrypoints() {
        let track = manifest(
            AssetKind::Track,
            EntryPoints {
                physics: None,
                visual: Some(1),
                collision: Some(2),
                metadata: None,
                audio: None,
                texture: None,
            },
        );
        assert_eq!(
            track.validate(),
            Err(ManifestError::MissingEntryPoint(ChunkKind::Metadata))
        );
    }

    #[test]
    fn rejects_wrong_entrypoint_kind() {
        let car = manifest(
            AssetKind::Car,
            EntryPoints {
                physics: Some(1),
                visual: Some(1),
                collision: None,
                metadata: None,
                audio: None,
                texture: None,
            },
        );
        assert_eq!(
            car.validate(),
            Err(ManifestError::WrongEntryPointKind {
                expected: ChunkKind::Physics,
                index: 1,
                actual: ChunkKind::Visual,
            })
        );
    }

    #[test]
    fn rejects_misaligned_chunk() {
        let mut car = manifest(
            AssetKind::Car,
            EntryPoints {
                physics: Some(0),
                visual: Some(1),
                collision: None,
                metadata: None,
                audio: None,
                texture: None,
            },
        );
        car.chunks[0].offset = 3;
        assert_eq!(
            car.validate(),
            Err(ManifestError::MisalignedChunk {
                index: 0,
                offset: 3,
                align: 16,
            })
        );
    }
}
