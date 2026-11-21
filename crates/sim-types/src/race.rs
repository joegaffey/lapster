use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize as SerdeDeserialize, Serialize as SerdeSerialize};

use crate::asset::{SectorSpec, SpawnPoint};

pub const RACE_DATA_VERSION: u16 = 1;

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
pub struct RaceData {
    pub version: u16,
    pub layout: Layout,
    pub start_finish: StartFinish,
    pub grid: Vec<SpawnPoint>,
    pub sectors: Vec<SectorSpec>,
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
pub struct Layout {
    pub layout_id: String,
    pub closed: bool,
    pub direction: LayoutDirection,
    pub points: Vec<[f32; 3]>,
    pub half_width: f32,
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
#[serde(rename_all = "lowercase")]
pub enum LayoutDirection {
    Forward,
    Reverse,
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
pub struct StartFinish {
    pub section_id: String,
    pub fraction: f32,
    pub position: [f32; 3],
    pub rotation_xyzw: [f32; 4],
}

impl RaceData {
    pub fn has_supported_version(&self) -> bool {
        self.version == RACE_DATA_VERSION
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> RaceData {
        RaceData {
            version: RACE_DATA_VERSION,
            layout: Layout {
                layout_id: String::from("main"),
                closed: true,
                direction: LayoutDirection::Forward,
                points: vec![[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [10.0, 0.0, 10.0]],
                half_width: 5.0,
            },
            start_finish: StartFinish {
                section_id: String::from("main"),
                fraction: 0.0,
                position: [0.0, 0.0, 0.0],
                rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
            },
            grid: vec![SpawnPoint {
                name: String::from("grid_0"),
                position: [0.0, 0.0, 2.0],
                rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
            }],
            sectors: vec![SectorSpec {
                name: String::from("sector_1"),
                start_fraction: 0.0,
                end_fraction: 1.0,
            }],
        }
    }

    #[test]
    fn race_data_round_trips_through_rkyv() {
        let data = sample();
        let bytes = rkyv::to_bytes::<_, 256>(&data).expect("serialize race data");
        rkyv::check_archived_root::<RaceData>(&bytes).expect("validate race data");
        let decoded: RaceData = rkyv::from_bytes(&bytes).expect("deserialize race data");
        assert_eq!(decoded, data);
    }

    #[test]
    fn version_is_explicit() {
        let data = sample();
        assert!(data.has_supported_version());
        let mut future = data;
        future.version = RACE_DATA_VERSION + 1;
        assert!(!future.has_supported_version());
    }
}
