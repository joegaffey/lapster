pub const SAB_MAGIC: [u8; 4] = *b"LSIM";
pub const SAB_LAYOUT_VERSION: u16 = 2;
pub const SAB_TOTAL_SIZE: usize = 4096;

pub const HEADER_OFFSET: usize = 0;
pub const HEADER_SIZE: usize = 64;

pub const SNAPSHOT_OFFSET: usize = 64;
pub const SNAPSHOT_COUNT: usize = 4;
pub const SNAPSHOT_STRIDE: usize = 288;
pub const SNAPSHOT_REGION_SIZE: usize = SNAPSHOT_COUNT * SNAPSHOT_STRIDE;

pub const INPUT_MAILBOX_OFFSET: usize = 1216;
pub const INPUT_MAILBOX_SIZE: usize = 64;

pub const FFB_MAILBOX_OFFSET: usize = 1280;
pub const FFB_MAILBOX_SIZE: usize = 64;

pub const RESERVED_OFFSET: usize = 1344;
pub const RESERVED_SIZE: usize = 2752;

pub const HEADER_FLAG_PAUSED: u32 = 1 << 0;
pub const HEADER_FLAG_RESET: u32 = 1 << 1;
pub const FFB_FLAG_ACTIVE: u32 = 1 << 0;
pub const FFB_FLAG_STOP: u32 = 1 << 1;

pub const SNAPSHOT_STATE_FLOATS: usize = 64;
pub mod snapshot_state {
    pub const POSITION: usize = 0;
    pub const ROTATION: usize = 3;
    pub const VELOCITY: usize = 7;
    pub const ANGULAR_VELOCITY: usize = 10;
    pub const WHEEL_ANGULAR_SPEEDS: usize = 13;
    pub const SUSPENSION_TRAVEL: usize = 17;
    pub const ENGINE_RPM: usize = 21;
    pub const RACK_TORQUE: usize = 22;
    pub const THROTTLE: usize = 23;
    pub const BRAKE: usize = 24;
    pub const SLIP_RATIO: usize = 25;
    pub const SLIP_ANGLE: usize = 29;
    pub const TIRE_LOAD: usize = 33;
    pub const CONTACT_FLAGS: usize = 37;
    pub const SURFACE_FRICTION: usize = 41;
    pub const WHEEL_STEER_ANGLE: usize = 45;
    pub const WHEEL_SPIN_ANGLE: usize = 49;
    pub const CONTACT_CENTER: usize = 53;
    pub const CONTACT_NORMAL: usize = 56;
    pub const GEAR: usize = 59;
    pub const ABS_ACTIVE: usize = 60;
    pub const ESC_ACTIVITY: usize = 61;
    pub const TICK: usize = 62;
}

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
    pub state: [f32; SNAPSHOT_STATE_FLOATS],
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
        assert_eq!(offset_of!(SnapshotSlot, state), 32);
        assert_eq!(snapshot_state::TICK, 62);
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
