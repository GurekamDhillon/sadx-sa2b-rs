//! Character physics (`PhysicsData`, 0x84 bytes per character).
//!
//! SA2 PC: `PhysicsArray` at 0x17391E8 in `sonic2app.exe`, 19 entries,
//! indexed by [`Character`](crate::enums::Character). Read from the exe file
//! on disk, never from a running process. Field names from `SA2Structs.h`.

use sa_formats::bytes::{AddressMap, Bytes, Endian};
use sa_formats::pe::Pe;

use crate::error::{Error, Result};

pub const SA2_PHYSICS_ARRAY: u32 = 0x17391E8;
pub const SA2_PHYSICS_COUNT: usize = 19;
pub const PHYSICS_SIZE: usize = 0x84;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PhysicsData {
    pub hang_time: i32,
    pub floor_grip: f32,
    pub speed_cap_h: f32,
    pub speed_cap_v: f32,
    pub speed_max_h: f32,
    pub push_speed_max: f32,
    pub jump_speed: f32,
    pub no_control_speed: f32,
    pub slide_speed: f32,
    pub jog_speed: f32,
    pub run_speed: f32,
    pub rush_speed: f32,
    pub knockback_speed: f32,
    pub dash_speed: f32,
    pub jump_add: f32,
    pub run_accel: f32,
    pub air_accel: f32,
    pub run_decel: f32,
    pub run_brake: f32,
    pub air_brake: f32,
    pub air_decel: f32,
    pub air_resist: f32,
    pub air_resist_v: f32,
    pub air_resist_h: f32,
    pub ground_friction: f32,
    pub ground_friction_h: f32,
    pub friction_cap: f32,
    pub rat_bound: f32,
    pub radius: f32,
    pub height: f32,
    pub weight: f32,
    pub camera_y: f32,
    pub center_height: f32,
}

impl PhysicsData {
    /// Field names in declaration order (32 floats after `hang_time`).
    pub const FLOAT_FIELDS: [&'static str; 32] = [
        "floor_grip", "speed_cap_h", "speed_cap_v", "speed_max_h", "push_speed_max", "jump_speed", "no_control_speed", "slide_speed",
        "jog_speed", "run_speed", "rush_speed", "knockback_speed", "dash_speed", "jump_add", "run_accel", "air_accel",
        "run_decel", "run_brake", "air_brake", "air_decel", "air_resist", "air_resist_v", "air_resist_h", "ground_friction",
        "ground_friction_h", "friction_cap", "rat_bound", "radius", "height", "weight", "camera_y", "center_height",
    ];

    /// Decodes one 0x84-byte little-endian row.
    pub fn decode(row: &[u8]) -> Result<PhysicsData> {
        let b = Bytes::new(row, Endian::Little);
        let f: Vec<f32> = (0..32).map(|i| b.f32(4 + i * 4)).collect::<std::result::Result<_, _>>()?;
        Ok(PhysicsData {
            hang_time: b.i32(0)?,
            floor_grip: f[0],
            speed_cap_h: f[1],
            speed_cap_v: f[2],
            speed_max_h: f[3],
            push_speed_max: f[4],
            jump_speed: f[5],
            no_control_speed: f[6],
            slide_speed: f[7],
            jog_speed: f[8],
            run_speed: f[9],
            rush_speed: f[10],
            knockback_speed: f[11],
            dash_speed: f[12],
            jump_add: f[13],
            run_accel: f[14],
            air_accel: f[15],
            run_decel: f[16],
            run_brake: f[17],
            air_brake: f[18],
            air_decel: f[19],
            air_resist: f[20],
            air_resist_v: f[21],
            air_resist_h: f[22],
            ground_friction: f[23],
            ground_friction_h: f[24],
            friction_cap: f[25],
            rat_bound: f[26],
            radius: f[27],
            height: f[28],
            weight: f[29],
            camera_y: f[30],
            center_height: f[31],
        })
    }

    /// Encodes back to the 0x84-byte little-endian row.
    pub fn encode(&self) -> [u8; PHYSICS_SIZE] {
        let mut out = [0u8; PHYSICS_SIZE];
        out[0..4].copy_from_slice(&self.hang_time.to_le_bytes());
        for (i, v) in self.floats().iter().enumerate() {
            out[4 + i * 4..8 + i * 4].copy_from_slice(&v.to_le_bytes());
        }
        out
    }

    pub fn floats(&self) -> [f32; 32] {
        [
            self.floor_grip, self.speed_cap_h, self.speed_cap_v, self.speed_max_h, self.push_speed_max, self.jump_speed,
            self.no_control_speed, self.slide_speed, self.jog_speed, self.run_speed, self.rush_speed, self.knockback_speed,
            self.dash_speed, self.jump_add, self.run_accel, self.air_accel, self.run_decel, self.run_brake, self.air_brake,
            self.air_decel, self.air_resist, self.air_resist_v, self.air_resist_h, self.ground_friction,
            self.ground_friction_h, self.friction_cap, self.rat_bound, self.radius, self.height, self.weight, self.camera_y,
            self.center_height,
        ]
    }
}

/// Reads the whole physics table from an exe image.
pub fn read_table(exe: &[u8], address: u32, count: usize) -> Result<Vec<PhysicsData>> {
    let pe = Pe::parse(exe)?;
    let off = pe.offset(address).ok_or_else(|| Error::NotFound(format!("physics table address {address:#x} not mapped in exe")))?;
    let b = Bytes::new(exe, Endian::Little);
    (0..count).map(|i| PhysicsData::decode(b.slice(off + i * PHYSICS_SIZE, PHYSICS_SIZE)?)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> PhysicsData {
        let mut row = [0u8; PHYSICS_SIZE];
        row[0..4].copy_from_slice(&60i32.to_le_bytes());
        for i in 0..32 {
            row[4 + i * 4..8 + i * 4].copy_from_slice(&(i as f32 + 0.5).to_le_bytes());
        }
        PhysicsData::decode(&row).unwrap()
    }

    #[test]
    fn decode_field_order() {
        let p = sample();
        assert_eq!(p.hang_time, 60);
        assert_eq!(p.floor_grip, 0.5);
        assert_eq!(p.jump_speed, 5.5);
        assert_eq!(p.center_height, 31.5);
    }

    #[test]
    fn encode_round_trips() {
        let p = sample();
        assert_eq!(PhysicsData::decode(&p.encode()).unwrap(), p);
    }

    #[test]
    fn short_row_errors() {
        assert!(PhysicsData::decode(&[0u8; 0x80]).is_err());
    }

    #[test]
    fn struct_matches_game_size() {
        assert_eq!(std::mem::size_of::<PhysicsData>(), PHYSICS_SIZE);
    }
}
