use core::fmt;

use rkyv::bytecheck::CheckBytes;
use rkyv::ser::serializers::AllocSerializer;
use rkyv::validation::validators::DefaultValidator;
use sha2::{Digest, Sha256};
use sim_types::{
    AssetKind, AssetManifest, CarPhysics, CarVisual, ChunkKind, ChunkRef, Compression, PackHeader,
    TrackCollision, TrackMetadata, TrackVisual, MAGIC, PACK_FORMAT_VERSION, PACK_HEADER_SIZE,
    SCHEMA_VERSION,
};

const CHUNK_ALIGN: usize = 64;

pub struct ChunkSpec {
    pub kind: ChunkKind,
    pub bytes: Vec<u8>,
}

pub fn archive<T>(value: &T) -> Result<Vec<u8>, PackError>
where
    T: rkyv::Serialize<AllocSerializer<256>>,
{
    let bytes = rkyv::to_bytes::<_, 256>(value)
        .map_err(|error| PackError::Build(format!("failed to serialize chunk: {error:?}")))?;
    Ok(bytes.to_vec())
}

pub fn write_pack(
    mut manifest: AssetManifest,
    chunks: Vec<ChunkSpec>,
) -> Result<Vec<u8>, PackError> {
    manifest.chunks = chunks
        .iter()
        .map(|chunk| ChunkRef {
            kind: chunk.kind,
            offset: 0,
            len: chunk.bytes.len() as u64,
            align: CHUNK_ALIGN as u32,
            hash: sha256(&chunk.bytes),
            compression: Compression::None,
            schema_version: SCHEMA_VERSION,
        })
        .collect();
    manifest
        .validate()
        .map_err(|error| PackError::ManifestInvalid(error.to_string()))?;

    let mut manifest_bytes =
        serde_json::to_vec(&manifest).map_err(|error| PackError::Manifest(error.to_string()))?;
    for _ in 0..8 {
        let base = align_up(PACK_HEADER_SIZE + manifest_bytes.len(), CHUNK_ALIGN);
        let mut offset = base;
        for chunk in &mut manifest.chunks {
            chunk.offset = offset as u64;
            offset = align_up(offset + chunk.len as usize, CHUNK_ALIGN);
        }
        let next = serde_json::to_vec(&manifest)
            .map_err(|error| PackError::Manifest(error.to_string()))?;
        let stable = next.len() == manifest_bytes.len()
            && align_up(PACK_HEADER_SIZE + next.len(), CHUNK_ALIGN) == base;
        manifest_bytes = next;
        if stable {
            break;
        }
    }

    let base = align_up(PACK_HEADER_SIZE + manifest_bytes.len(), CHUNK_ALIGN);
    if manifest.chunks.first().map(|chunk| chunk.offset as usize) != Some(base)
        && !manifest.chunks.is_empty()
    {
        return Err(PackError::Build(
            "failed to stabilize pack chunk offsets".to_string(),
        ));
    }

    let mut out = vec![0u8; PACK_HEADER_SIZE];
    out.extend_from_slice(&manifest_bytes);
    out.resize(base, 0);

    for (chunk, spec) in manifest.chunks.iter().zip(chunks.iter()) {
        let offset = chunk.offset as usize;
        if out.len() > offset {
            return Err(PackError::Build(
                "chunk offset overlaps previous data".to_string(),
            ));
        }
        out.resize(offset, 0);
        out.extend_from_slice(&spec.bytes);
    }

    let pack_len = out.len() as u64;
    let header = PackHeader {
        magic: MAGIC,
        format_version: PACK_FORMAT_VERSION,
        header_size: PACK_HEADER_SIZE as u16,
        manifest_offset: PACK_HEADER_SIZE as u64,
        manifest_len: manifest_bytes.len() as u64,
        pack_len,
        reserved: [0; 32],
    };
    write_header(&mut out[..PACK_HEADER_SIZE], &header);

    Pack::from_bytes(&out)?;
    Ok(out)
}

fn write_header(out: &mut [u8], header: &PackHeader) {
    out[0..4].copy_from_slice(&header.magic);
    out[4..6].copy_from_slice(&header.format_version.to_le_bytes());
    out[6..8].copy_from_slice(&header.header_size.to_le_bytes());
    out[8..16].copy_from_slice(&header.manifest_offset.to_le_bytes());
    out[16..24].copy_from_slice(&header.manifest_len.to_le_bytes());
    out[24..32].copy_from_slice(&header.pack_len.to_le_bytes());
    out[32..64].copy_from_slice(&header.reserved);
}

pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    let digest = Sha256::digest(bytes);
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&digest);
    hash
}

fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}

#[derive(Debug)]
pub enum PackError {
    TooShort,
    BadMagic,
    UnsupportedVersion(u16),
    BadHeaderSize(u16),
    LengthMismatch { expected: u64, actual: u64 },
    ManifestOutOfBounds,
    MisalignedManifest { offset: u64 },
    Manifest(String),
    ManifestInvalid(String),
    ChunkOutOfBounds { kind: ChunkKind },
    HashMismatch { kind: ChunkKind },
    MissingEntryPoint(ChunkKind),
    MissingChunk(u32),
    Build(String),
    Archive(String),
}

impl fmt::Display for PackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackError::TooShort => f.write_str("pack is shorter than the header"),
            PackError::BadMagic => f.write_str("pack magic does not match LSIM"),
            PackError::UnsupportedVersion(version) => {
                write!(f, "unsupported pack format version {version}")
            }
            PackError::BadHeaderSize(size) => write!(f, "unsupported pack header size {size}"),
            PackError::LengthMismatch { expected, actual } => {
                write!(f, "pack length mismatch: expected {expected}, got {actual}")
            }
            PackError::ManifestOutOfBounds => f.write_str("manifest is outside the pack"),
            PackError::MisalignedManifest { offset } => {
                write!(f, "manifest offset {offset} is not 16-byte aligned")
            }
            PackError::Manifest(message) => write!(f, "manifest is invalid: {message}"),
            PackError::ManifestInvalid(message) => {
                write!(f, "manifest validation failed: {message}")
            }
            PackError::ChunkOutOfBounds { kind } => {
                write!(f, "chunk {kind} is outside the pack")
            }
            PackError::HashMismatch { kind } => write!(f, "chunk {kind} hash does not match"),
            PackError::MissingEntryPoint(kind) => write!(f, "missing entrypoint {kind}"),
            PackError::MissingChunk(index) => write!(f, "missing chunk {index}"),
            PackError::Build(message) => write!(f, "failed to build pack: {message}"),
            PackError::Archive(message) => write!(f, "archive validation failed: {message}"),
        }
    }
}

impl std::error::Error for PackError {}

pub struct Pack<'a> {
    bytes: &'a [u8],
    header: PackHeader,
    manifest: AssetManifest,
}

impl<'a> Pack<'a> {
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, PackError> {
        if bytes.len() < PACK_HEADER_SIZE {
            return Err(PackError::TooShort);
        }

        let header = read_header(bytes);
        if header.magic != MAGIC {
            return Err(PackError::BadMagic);
        }
        if header.format_version != PACK_FORMAT_VERSION {
            return Err(PackError::UnsupportedVersion(header.format_version));
        }
        if header.header_size as usize != PACK_HEADER_SIZE {
            return Err(PackError::BadHeaderSize(header.header_size));
        }
        if header.pack_len != bytes.len() as u64 {
            return Err(PackError::LengthMismatch {
                expected: header.pack_len,
                actual: bytes.len() as u64,
            });
        }
        if header.manifest_offset & 15 != 0 {
            return Err(PackError::MisalignedManifest {
                offset: header.manifest_offset,
            });
        }

        let manifest_start = header.manifest_offset as usize;
        let manifest_end = manifest_start
            .checked_add(header.manifest_len as usize)
            .ok_or(PackError::ManifestOutOfBounds)?;
        if manifest_end > bytes.len() {
            return Err(PackError::ManifestOutOfBounds);
        }

        let manifest_bytes = &bytes[manifest_start..manifest_end];
        let manifest: AssetManifest = serde_json::from_slice(manifest_bytes)
            .map_err(|error| PackError::Manifest(error.to_string()))?;
        manifest
            .validate()
            .map_err(|error| PackError::ManifestInvalid(error.to_string()))?;

        let pack = Self {
            bytes,
            header,
            manifest,
        };
        pack.verify_chunks()?;
        Ok(pack)
    }

    pub fn header(&self) -> &PackHeader {
        &self.header
    }

    pub fn manifest(&self) -> &AssetManifest {
        &self.manifest
    }

    pub fn kind(&self) -> AssetKind {
        self.manifest.kind
    }

    pub fn chunk_bytes(&self, kind: ChunkKind) -> Result<&'a [u8], PackError> {
        let index = self
            .manifest
            .entrypoints
            .get(kind)
            .ok_or(PackError::MissingEntryPoint(kind))?;
        let chunk = self
            .manifest
            .chunks
            .get(index as usize)
            .ok_or(PackError::MissingChunk(index))?;
        self.chunk_slice(chunk)
    }

    pub fn physics(&self) -> Result<&'a <CarPhysics as rkyv::Archive>::Archived, PackError> {
        self.archived::<CarPhysics>(ChunkKind::Physics)
    }

    pub fn car_visual(&self) -> Result<&'a <CarVisual as rkyv::Archive>::Archived, PackError> {
        self.archived::<CarVisual>(ChunkKind::Visual)
    }

    pub fn track_metadata(
        &self,
    ) -> Result<&'a <TrackMetadata as rkyv::Archive>::Archived, PackError> {
        self.archived::<TrackMetadata>(ChunkKind::Metadata)
    }

    pub fn track_collision(
        &self,
    ) -> Result<&'a <TrackCollision as rkyv::Archive>::Archived, PackError> {
        self.archived::<TrackCollision>(ChunkKind::Collision)
    }

    pub fn track_visual(&self) -> Result<&'a <TrackVisual as rkyv::Archive>::Archived, PackError> {
        self.archived::<TrackVisual>(ChunkKind::Visual)
    }

    pub fn car_physics_owned(&self) -> Result<CarPhysics, PackError> {
        self.deserialize::<CarPhysics>(ChunkKind::Physics)
    }

    pub fn car_visual_owned(&self) -> Result<CarVisual, PackError> {
        self.deserialize::<CarVisual>(ChunkKind::Visual)
    }

    pub fn track_metadata_owned(&self) -> Result<TrackMetadata, PackError> {
        self.deserialize::<TrackMetadata>(ChunkKind::Metadata)
    }

    pub fn track_collision_owned(&self) -> Result<TrackCollision, PackError> {
        self.deserialize::<TrackCollision>(ChunkKind::Collision)
    }

    pub fn track_visual_owned(&self) -> Result<TrackVisual, PackError> {
        self.deserialize::<TrackVisual>(ChunkKind::Visual)
    }

    fn chunk_slice(&self, chunk: &ChunkRef) -> Result<&'a [u8], PackError> {
        let start = chunk.offset as usize;
        let end = start
            .checked_add(chunk.len as usize)
            .ok_or(PackError::ChunkOutOfBounds { kind: chunk.kind })?;
        if start > self.bytes.len() || end > self.bytes.len() {
            return Err(PackError::ChunkOutOfBounds { kind: chunk.kind });
        }
        Ok(&self.bytes[start..end])
    }

    fn verify_chunks(&self) -> Result<(), PackError> {
        for chunk in &self.manifest.chunks {
            let bytes = self.chunk_slice(chunk)?;
            let hash = Sha256::digest(bytes);
            if hash.as_slice() != chunk.hash.as_slice() {
                return Err(PackError::HashMismatch { kind: chunk.kind });
            }
        }
        Ok(())
    }

    fn archived<T>(&self, kind: ChunkKind) -> Result<&'a T::Archived, PackError>
    where
        T: rkyv::Archive,
        T::Archived: CheckBytes<DefaultValidator<'a>>,
    {
        let bytes = self.chunk_bytes(kind)?;
        rkyv::check_archived_root::<T>(bytes)
            .map_err(|error| PackError::Archive(format!("{error:?}")))
    }

    fn deserialize<T>(&self, kind: ChunkKind) -> Result<T, PackError>
    where
        T: rkyv::Archive,
        T::Archived: 'a
            + CheckBytes<DefaultValidator<'a>>
            + rkyv::Deserialize<T, rkyv::de::deserializers::SharedDeserializeMap>,
    {
        let bytes = self.chunk_bytes(kind)?;
        rkyv::from_bytes::<T>(bytes).map_err(|error| PackError::Archive(format!("{error:?}")))
    }
}

fn read_header(bytes: &[u8]) -> PackHeader {
    let mut magic = [0u8; 4];
    magic.copy_from_slice(&bytes[0..4]);
    let mut reserved = [0u8; 32];
    reserved.copy_from_slice(&bytes[32..64]);
    PackHeader {
        magic,
        format_version: read_u16(bytes, 4),
        header_size: read_u16(bytes, 6),
        manifest_offset: read_u64(bytes, 8),
        manifest_len: read_u64(bytes, 16),
        pack_len: read_u64(bytes, 24),
        reserved,
    }
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    let mut value = [0u8; 8];
    value.copy_from_slice(&bytes[offset..offset + 8]);
    u64::from_le_bytes(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_types::{
        Aabb, AssetStats, EntryPoints, MeshBuffers, TrackVisual, ENGINE_FRAME, ENGINE_UNITS,
    };

    fn empty_track() -> (TrackVisual, TrackCollision, TrackMetadata) {
        (
            TrackVisual {
                meshes: Vec::new(),
                materials: Vec::new(),
                textures: Vec::new(),
                mesh_buffers: vec![MeshBuffers {
                    positions: vec![[0.0, 0.0, 0.0]],
                    normals: vec![[0.0, 1.0, 0.0]],
                    uvs: vec![[0.0, 0.0]],
                    colors: Vec::new(),
                    indices: Vec::new(),
                }],
                instances: Vec::new(),
            },
            TrackCollision {
                vertices: Vec::new(),
                indices: Vec::new(),
                surface_ids: Vec::new(),
                bounds: Aabb {
                    min: [-1.0; 3],
                    max: [1.0; 3],
                },
            },
            TrackMetadata {
                name: "tiny".to_string(),
                length: 0.0,
                spawns: Vec::new(),
                pit_boxes: Vec::new(),
                cameras: Vec::new(),
                sectors: Vec::new(),
                surfaces: Vec::new(),
            },
        )
    }

    #[test]
    fn writes_and_parses_tiny_track_pack() {
        let (visual, collision, metadata) = empty_track();
        let chunks = vec![
            ChunkSpec {
                kind: ChunkKind::Visual,
                bytes: archive(&visual).expect("visual archive"),
            },
            ChunkSpec {
                kind: ChunkKind::Collision,
                bytes: archive(&collision).expect("collision archive"),
            },
            ChunkSpec {
                kind: ChunkKind::Metadata,
                bytes: archive(&metadata).expect("metadata archive"),
            },
        ];
        let manifest = AssetManifest {
            magic: MAGIC,
            schema_version: SCHEMA_VERSION,
            kind: AssetKind::Track,
            id: "tiny".to_string(),
            source_hash: [0; 32],
            units: ENGINE_UNITS,
            frame: ENGINE_FRAME,
            bounds: Aabb {
                min: [-1.0; 3],
                max: [1.0; 3],
            },
            chunks: Vec::new(),
            entrypoints: EntryPoints {
                physics: None,
                visual: Some(0),
                collision: Some(1),
                metadata: Some(2),
                audio: None,
                texture: None,
            },
            stats: AssetStats {
                vertices: 0,
                triangles: 0,
                materials: 0,
                textures: 0,
                draw_calls: 0,
            },
            warnings: Vec::new(),
        };

        let bytes = write_pack(manifest, chunks).expect("pack writes");
        let pack = Pack::from_bytes(&bytes).expect("pack parses");
        assert_eq!(pack.kind(), AssetKind::Track);
        assert_eq!(pack.manifest().id, "tiny");
        assert_eq!(pack.track_visual_owned().expect("visual"), visual);
        assert_eq!(pack.track_collision_owned().expect("collision"), collision);
        assert_eq!(pack.track_metadata_owned().expect("metadata"), metadata);
        assert_eq!(
            pack.chunk_bytes(ChunkKind::Visual)
                .expect("visual chunk")
                .len(),
            archive(&visual).expect("archive").len()
        );
    }
}
