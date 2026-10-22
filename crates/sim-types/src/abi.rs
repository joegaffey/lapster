pub const SAB_MAGIC: [u8; 4] = *b"LSIM";
pub const SAB_LAYOUT_VERSION: u16 = 1;
pub const SAB_TOTAL_SIZE: usize = 4096;

pub const HEADER_OFFSET: usize = 0;
pub const HEADER_SIZE: usize = 64;

pub const SNAPSHOT_OFFSET: usize = 64;
pub const SNAPSHOT_COUNT: usize = 4;
pub const SNAPSHOT_STRIDE: usize = 256;
pub const SNAPSHOT_REGION_SIZE: usize = SNAPSHOT_COUNT * SNAPSHOT_STRIDE;

pub const INPUT_MAILBOX_OFFSET: usize = 1088;
pub const INPUT_MAILBOX_SIZE: usize = 64;

pub const FFB_MAILBOX_OFFSET: usize = 1152;
pub const FFB_MAILBOX_SIZE: usize = 64;

pub const RESERVED_OFFSET: usize = 1216;
pub const RESERVED_SIZE: usize = 2880;

pub const HEADER_FLAG_PAUSED: u32 = 1 << 0;
pub const HEADER_FLAG_RESET: u32 = 1 << 1;
pub const FFB_FLAG_ACTIVE: u32 = 1 << 0;
pub const FFB_FLAG_STOP: u32 = 1 << 1;

#[repr(C, align(8))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SharedHeader {
    pub magic: [u8; 4],
    pub layout_version: u16,
    pub header_size: u16,
    pub slot_count: u32,
    pub slot_stride: u32,
    pub latest_slot: u32,
    pub flags: u32,
    pub writer_tick: u32,
    pub reserved0: u32,
    pub time_base_seconds: f64,
    pub reserved1: [u8; 24],
}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnapshotSlot {
    pub seq: u32,
    pub reserved0: u32,
    pub tick: u64,
    pub timestamp_seconds: f64,
    pub reserved1: [u8; 8],
    pub position: [f32; 4],
    pub rotation_xyzw: [f32; 4],
    pub velocity: [f32; 4],
    pub angular_velocity: [f32; 4],
    pub wheel_angular_speeds: [f32; 4],
    pub suspension_travel: [f32; 4],
    pub engine_rpm_rack_torque_throttle_brake: [f32; 4],
    pub slip_ratio: [f32; 4],
    pub slip_angle: [f32; 4],
    pub tire_load: [f32; 4],
    pub contact_flags: [f32; 4],
    pub surface_friction: [f32; 4],
    pub reserved2: [u8; 32],
}

#[repr(C, align(8))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InputMailbox {
    pub seq: u32,
    pub buttons: u32,
    pub timestamp_seconds: f64,
    pub steering: f32,
    pub throttle: f32,
    pub brake: f32,
    pub clutch: f32,
    pub gear: i32,
    pub reserved: [u8; 28],
}

#[repr(C, align(8))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FfbMailbox {
    pub seq: u32,
    pub flags: u32,
    pub timestamp_seconds: f64,
    pub torque: f32,
    pub reserved: [u8; 44],
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, offset_of, size_of};

    #[test]
    fn header_layout_is_stable() {
        assert_eq!(size_of::<SharedHeader>(), HEADER_SIZE);
        assert_eq!(align_of::<SharedHeader>(), 8);
        assert_eq!(offset_of!(SharedHeader, magic), 0);
        assert_eq!(offset_of!(SharedHeader, layout_version), 4);
        assert_eq!(offset_of!(SharedHeader, header_size), 6);
        assert_eq!(offset_of!(SharedHeader, slot_count), 8);
        assert_eq!(offset_of!(SharedHeader, slot_stride), 12);
        assert_eq!(offset_of!(SharedHeader, latest_slot), 16);
        assert_eq!(offset_of!(SharedHeader, flags), 20);
        assert_eq!(offset_of!(SharedHeader, writer_tick), 24);
        assert_eq!(offset_of!(SharedHeader, reserved0), 28);
        assert_eq!(offset_of!(SharedHeader, time_base_seconds), 32);
        assert_eq!(offset_of!(SharedHeader, reserved1), 40);
    }

    #[test]
    fn snapshot_layout_is_stable() {
        assert_eq!(size_of::<SnapshotSlot>(), SNAPSHOT_STRIDE);
        assert_eq!(align_of::<SnapshotSlot>(), 16);
        assert_eq!(offset_of!(SnapshotSlot, seq), 0);
        assert_eq!(offset_of!(SnapshotSlot, reserved0), 4);
        assert_eq!(offset_of!(SnapshotSlot, tick), 8);
        assert_eq!(offset_of!(SnapshotSlot, timestamp_seconds), 16);
        assert_eq!(offset_of!(SnapshotSlot, reserved1), 24);
        assert_eq!(offset_of!(SnapshotSlot, position), 32);
        assert_eq!(offset_of!(SnapshotSlot, rotation_xyzw), 48);
        assert_eq!(offset_of!(SnapshotSlot, velocity), 64);
        assert_eq!(offset_of!(SnapshotSlot, angular_velocity), 80);
        assert_eq!(offset_of!(SnapshotSlot, wheel_angular_speeds), 96);
        assert_eq!(offset_of!(SnapshotSlot, suspension_travel), 112);
        assert_eq!(
            offset_of!(SnapshotSlot, engine_rpm_rack_torque_throttle_brake),
            128
        );
        assert_eq!(offset_of!(SnapshotSlot, slip_ratio), 144);
        assert_eq!(offset_of!(SnapshotSlot, slip_angle), 160);
        assert_eq!(offset_of!(SnapshotSlot, tire_load), 176);
        assert_eq!(offset_of!(SnapshotSlot, contact_flags), 192);
        assert_eq!(offset_of!(SnapshotSlot, surface_friction), 208);
        assert_eq!(offset_of!(SnapshotSlot, reserved2), 224);
    }

    #[test]
    fn mailbox_layout_is_stable() {
        assert_eq!(size_of::<InputMailbox>(), INPUT_MAILBOX_SIZE);
        assert_eq!(align_of::<InputMailbox>(), 8);
        assert_eq!(offset_of!(InputMailbox, seq), 0);
        assert_eq!(offset_of!(InputMailbox, buttons), 4);
        assert_eq!(offset_of!(InputMailbox, timestamp_seconds), 8);
        assert_eq!(offset_of!(InputMailbox, steering), 16);
        assert_eq!(offset_of!(InputMailbox, throttle), 20);
        assert_eq!(offset_of!(InputMailbox, brake), 24);
        assert_eq!(offset_of!(InputMailbox, clutch), 28);
        assert_eq!(offset_of!(InputMailbox, gear), 32);
        assert_eq!(offset_of!(InputMailbox, reserved), 36);

        assert_eq!(size_of::<FfbMailbox>(), FFB_MAILBOX_SIZE);
        assert_eq!(align_of::<FfbMailbox>(), 8);
        assert_eq!(offset_of!(FfbMailbox, seq), 0);
        assert_eq!(offset_of!(FfbMailbox, flags), 4);
        assert_eq!(offset_of!(FfbMailbox, timestamp_seconds), 8);
        assert_eq!(offset_of!(FfbMailbox, torque), 16);
        assert_eq!(offset_of!(FfbMailbox, reserved), 20);
    }

    #[test]
    fn regions_fit_the_sab() {
        assert_eq!(HEADER_OFFSET + HEADER_SIZE, SNAPSHOT_OFFSET);
        assert_eq!(SNAPSHOT_OFFSET + SNAPSHOT_REGION_SIZE, INPUT_MAILBOX_OFFSET);
        assert_eq!(
            INPUT_MAILBOX_OFFSET + INPUT_MAILBOX_SIZE,
            FFB_MAILBOX_OFFSET
        );
        assert_eq!(FFB_MAILBOX_OFFSET + FFB_MAILBOX_SIZE, RESERVED_OFFSET);
        assert_eq!(RESERVED_OFFSET + RESERVED_SIZE, SAB_TOTAL_SIZE);
    }

    #[test]
    fn regions_are_aligned() {
        assert_eq!(HEADER_OFFSET % 8, 0);
        assert_eq!(SNAPSHOT_OFFSET % 16, 0);
        assert_eq!(SNAPSHOT_STRIDE % 16, 0);
        assert_eq!(INPUT_MAILBOX_OFFSET % 16, 0);
        assert_eq!(FFB_MAILBOX_OFFSET % 16, 0);
    }
}
