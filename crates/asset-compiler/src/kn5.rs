use std::collections::HashMap;
use std::path::Path;

use anyhow::{bail, Context, Result};
use glam::Mat4;

const MAGIC: &[u8; 6] = b"sc6969";
const DUMMY: i32 = 1;
const MESH: i32 = 2;
const SKINNED: i32 = 3;

#[derive(Debug, Clone)]
pub struct Kn5Material {
    pub name: String,
    pub shader: String,
    pub props: HashMap<String, f32>,
    pub textures: HashMap<String, String>,
    pub alpha_blend: bool,
    pub alpha_tested: bool,
}

#[derive(Debug, Clone)]
pub struct Kn5Texture {
    pub name: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Kn5Mesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
    pub material: u32,
}

#[derive(Debug, Clone)]
pub struct Kn5Node {
    pub kind: i32,
    pub name: String,
    pub active: bool,
    pub matrix: Option<Mat4>,
    pub children: Vec<Kn5Node>,
    pub mesh: Option<Kn5Mesh>,
}

#[derive(Debug)]
pub struct Kn5 {
    pub version: u32,
    pub textures: Vec<Kn5Texture>,
    pub materials: Vec<Kn5Material>,
    pub root: Kn5Node,
}

impl Kn5 {
    pub fn load(path: &Path) -> Result<Self> {
        let bytes =
            std::fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
        Self::from_bytes(&bytes).with_context(|| format!("failed to parse {}", path.display()))
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let mut reader = Reader { bytes, offset: 0 };
        if reader.raw(6)? != MAGIC {
            bail!("not a KN5 file");
        }
        let version = reader.u32()?;
        if version > 5 {
            reader.u32()?;
        }

        let texture_count = reader.u32()?;
        let mut textures = Vec::with_capacity(texture_count as usize);
        for _ in 0..texture_count {
            reader.u32()?;
            let name = reader.string()?;
            let len = reader.u32()? as usize;
            textures.push(Kn5Texture {
                name,
                bytes: reader.raw(len)?.to_vec(),
            });
        }

        let material_count = reader.u32()?;
        let mut materials = Vec::with_capacity(material_count as usize);
        for _ in 0..material_count {
            let name = reader.string()?;
            let shader = reader.string()?;
            let alpha_blend = reader.u8()? != 0;
            let alpha_tested = reader.u8()? != 0;
            reader.i32()?;
            let prop_count = reader.u32()?;
            let mut props = HashMap::with_capacity(prop_count as usize);
            for _ in 0..prop_count {
                let key = reader.string()?;
                let value = reader.f32()?;
                reader.raw(4 * 9)?;
                props.insert(key, value);
            }
            let texture_count = reader.u32()?;
            let mut textures = HashMap::with_capacity(texture_count as usize);
            for _ in 0..texture_count {
                let slot = reader.string()?;
                reader.u32()?;
                let texture = reader.string()?;
                textures.insert(slot, texture);
            }
            materials.push(Kn5Material {
                name,
                shader,
                props,
                textures,
                alpha_blend,
                alpha_tested,
            });
        }

        let root = read_node(&mut reader)?;
        Ok(Self {
            version,
            textures,
            materials,
            root,
        })
    }

    pub fn walk(&self) -> Vec<(&Kn5Node, Mat4)> {
        let mut out = Vec::new();
        walk_node(&self.root, Mat4::IDENTITY, &mut out);
        out
    }

    pub fn find_node(&self, name: &str) -> Option<(&Kn5Node, Mat4)> {
        self.walk().into_iter().find(|(node, _)| node.name == name)
    }
}

fn walk_node<'a>(node: &'a Kn5Node, parent: Mat4, out: &mut Vec<(&'a Kn5Node, Mat4)>) {
    let world = parent * node.matrix.unwrap_or(Mat4::IDENTITY);
    out.push((node, world));
    for child in &node.children {
        walk_node(child, world, out);
    }
}

fn read_node(reader: &mut Reader<'_>) -> Result<Kn5Node> {
    let kind = reader.i32()?;
    let name = reader.string()?;
    let child_count = reader.i32()?;
    if !(0..=100_000).contains(&child_count) {
        bail!("invalid child count {child_count} for node {name}");
    }
    let active = reader.u8()? != 0;
    let mut matrix = None;
    let mut mesh = None;

    match kind {
        DUMMY => {
            let values = reader.f32_array::<16>()?;
            matrix = Some(Mat4::from_cols_array(&values));
        }
        MESH => {
            reader.u8()?;
            reader.u8()?;
            reader.u8()?;
            let vertex_count = reader.u32()? as usize;
            let mut positions = Vec::with_capacity(vertex_count);
            let mut normals = Vec::with_capacity(vertex_count);
            let mut uvs = Vec::with_capacity(vertex_count);
            for _ in 0..vertex_count {
                let values = reader.f32_array::<11>()?;
                positions.push([values[0], values[1], values[2]]);
                normals.push([values[3], values[4], values[5]]);
                uvs.push([values[6], values[7]]);
            }
            let index_count = reader.u32()? as usize;
            let indices = reader.u16_array(index_count)?;
            let material = reader.u32()?;
            reader.u32()?;
            reader.f32()?;
            reader.f32()?;
            reader.f32_array::<3>()?;
            reader.f32()?;
            reader.u8()?;
            mesh = Some(Kn5Mesh {
                positions,
                normals,
                uvs,
                indices: indices.into_iter().map(u32::from).collect(),
                material,
            });
        }
        SKINNED => bail!("skinned KN5 nodes are not supported yet"),
        other => bail!("unknown KN5 node type {other}"),
    }

    let mut children = Vec::with_capacity(child_count as usize);
    for _ in 0..child_count {
        children.push(read_node(reader)?);
    }

    Ok(Kn5Node {
        kind,
        name,
        active,
        matrix,
        children,
        mesh,
    })
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn raw(&mut self, len: usize) -> Result<&'a [u8]> {
        let end = self
            .offset
            .checked_add(len)
            .context("KN5 offset overflow")?;
        if end > self.bytes.len() {
            bail!("KN5 file ended unexpectedly at offset {}", self.offset);
        }
        let value = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8> {
        Ok(self.raw(1)?[0])
    }

    fn u32(&mut self) -> Result<u32> {
        let bytes: [u8; 4] = self.raw(4)?.try_into().unwrap();
        Ok(u32::from_le_bytes(bytes))
    }

    fn i32(&mut self) -> Result<i32> {
        let bytes: [u8; 4] = self.raw(4)?.try_into().unwrap();
        Ok(i32::from_le_bytes(bytes))
    }

    fn f32(&mut self) -> Result<f32> {
        let bytes: [u8; 4] = self.raw(4)?.try_into().unwrap();
        Ok(f32::from_le_bytes(bytes))
    }

    fn f32_array<const N: usize>(&mut self) -> Result<[f32; N]> {
        let bytes = self.raw(4 * N)?;
        let mut values = [0.0; N];
        for (index, value) in values.iter_mut().enumerate() {
            let start = index * 4;
            *value = f32::from_le_bytes(bytes[start..start + 4].try_into().unwrap());
        }
        Ok(values)
    }

    fn u16_array(&mut self, len: usize) -> Result<Vec<u16>> {
        let bytes = self.raw(len * 2)?;
        let mut values = Vec::with_capacity(len);
        for chunk in bytes.chunks(2) {
            values.push(u16::from_le_bytes(chunk.try_into().unwrap()));
        }
        Ok(values)
    }

    fn string(&mut self) -> Result<String> {
        let len = self.u32()? as usize;
        if len > 1_000_000 {
            bail!("KN5 string is too long: {len}");
        }
        let bytes = self.raw(len)?;
        Ok(String::from_utf8_lossy(bytes).into_owned())
    }
}
