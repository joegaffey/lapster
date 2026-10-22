use core::fmt;

use rkyv::bytecheck::CheckBytes;
use rkyv::validation::validators::DefaultValidator;
use sha2::{Digest, Sha256};
use sim_types::{
    AssetKind, AssetManifest, CarPhysics, CarVisual, ChunkKind, ChunkRef, PackHeader,
    TrackCollision, TrackMetadata, TrackVisual, MAGIC, PACK_FORMAT_VERSION, PACK_HEADER_SIZE,
};

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
