use anyhow::{bail, Result};
use serde::Deserialize;
use sim_types::{
    CarPhysics, InstanceRef, MaterialRef, MeshRef, NodeTransform, TextureRef, TrackMetadata,
    WheelVisual,
};

#[derive(Debug, Deserialize)]
pub struct UnitsSource {
    pub length: String,
    pub mass: String,
    pub time: String,
    pub angle: String,
}

#[derive(Debug, Deserialize)]
pub struct FrameSource {
    pub handedness: String,
    pub up: String,
    pub forward: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "shape", rename_all = "lowercase")]
pub enum Primitive {
    #[serde(rename = "box")]
    BoxShape {
        id: String,
        size: [f32; 3],
        position: [f32; 3],
        material: String,
    },
    Plane {
        id: String,
        size: [f32; 2],
        position: [f32; 3],
        material: String,
    },
    Cylinder {
        id: String,
        radius: f32,
        width: f32,
        axis: String,
        segments: u32,
        position: [f32; 3],
        material: String,
    },
}

impl Primitive {
    pub fn id(&self) -> &str {
        match self {
            Primitive::BoxShape { id, .. }
            | Primitive::Plane { id, .. }
            | Primitive::Cylinder { id, .. } => id,
        }
    }

    pub fn material(&self) -> &str {
        match self {
            Primitive::BoxShape { material, .. }
            | Primitive::Plane { material, .. }
            | Primitive::Cylinder { material, .. } => material,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "shape", rename_all = "lowercase")]
pub enum CollisionPrimitive {
    Plane {
        #[serde(rename = "id")]
        _id: String,
        size: [f32; 2],
        position: [f32; 3],
        surface: u16,
    },
    #[serde(rename = "box")]
    BoxShape {
        #[serde(rename = "id")]
        _id: String,
        size: [f32; 3],
        position: [f32; 3],
        surface: u16,
    },
}

impl CollisionPrimitive {
    pub fn surface(&self) -> u16 {
        match self {
            CollisionPrimitive::Plane { surface, .. }
            | CollisionPrimitive::BoxShape { surface, .. } => *surface,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CarVisualSource {
    pub body: NodeTransform,
    pub wheels: Vec<WheelVisual>,
    pub steering_wheel: Option<NodeTransform>,
    #[serde(default, rename = "meshes")]
    pub _meshes: Vec<MeshRef>,
    pub materials: Vec<MaterialRef>,
    #[serde(default)]
    pub textures: Vec<TextureRef>,
    pub primitives: Vec<Primitive>,
}

#[derive(Debug, Deserialize)]
pub struct CarSource {
    pub schema: String,
    pub kind: String,
    pub id: String,
    pub units: UnitsSource,
    pub frame: FrameSource,
    pub physics: CarPhysics,
    pub visual: CarVisualSource,
}

#[derive(Debug, Deserialize)]
pub struct TrackVisualSource {
    pub materials: Vec<MaterialRef>,
    #[serde(default)]
    pub textures: Vec<TextureRef>,
    pub primitives: Vec<Primitive>,
    #[serde(default, rename = "meshes")]
    pub _meshes: Vec<MeshRef>,
    #[serde(default, rename = "instances")]
    pub _instances: Vec<InstanceRef>,
}

#[derive(Debug, Deserialize)]
pub struct CollisionSource {
    pub primitives: Vec<CollisionPrimitive>,
}

#[derive(Debug, Deserialize)]
pub struct TrackSource {
    pub schema: String,
    pub kind: String,
    pub id: String,
    pub units: UnitsSource,
    pub frame: FrameSource,
    pub metadata: TrackMetadata,
    pub visual: TrackVisualSource,
    pub collision: CollisionSource,
}

pub fn validate_source_header(
    schema: &str,
    kind: &str,
    expected_kind: &str,
    units: &UnitsSource,
    frame: &FrameSource,
) -> Result<()> {
    if schema != "sim-source-v1" {
        bail!("unsupported source schema {schema}; expected sim-source-v1");
    }
    if kind != expected_kind {
        bail!("expected a {expected_kind} source, got {kind}");
    }
    if units.length != "m" || units.mass != "kg" || units.time != "s" || units.angle != "rad" {
        bail!("source units must be m, kg, s, rad");
    }
    if frame.handedness != "right" || frame.up != "+Y" || frame.forward != "-Z" {
        bail!("source frame must be right-handed with +Y up and -Z forward");
    }
    Ok(())
}
