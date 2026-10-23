use serde::{Deserialize, Serialize};

use crate::asset::{CarPhysics, PacejkaCoefficients, TorquePoint};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CarTuning {
    pub chassis: ChassisTuning,
    pub front: AxleTuning,
    pub rear: AxleTuning,
    pub engine: EngineTuning,
    pub drivetrain: DrivetrainTuning,
    pub brakes: BrakesTuning,
    pub steering: SteeringTuning,
    pub aero: AeroTuning,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChassisTuning {
    pub inertia: [f32; 3],
    pub center_of_gravity: [f32; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AxleTuning {
    pub suspension: SuspensionTuning,
    pub tire: TireTuning,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuspensionTuning {
    pub travel: f32,
    pub spring_rate: f32,
    pub damper_bump: f32,
    pub damper_rebound: f32,
    pub anti_roll: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TireTuning {
    pub lateral: PacejkaTuning,
    pub longitudinal: PacejkaTuning,
    pub relaxation_length: f32,
    pub peak_slip_ratio: f32,
    pub friction_scale: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PacejkaTuning {
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineTuning {
    pub idle_rpm: f32,
    pub redline_rpm: f32,
    pub torque_curve: Vec<[f32; 2]>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrivetrainTuning {
    pub gear_ratios: Vec<f32>,
    pub final_drive: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrakesTuning {
    pub front: f32,
    pub rear: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SteeringTuning {
    pub max_angle: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AeroTuning {
    pub drag_coefficient: f32,
    pub frontal_area: f32,
}

impl CarTuning {
    pub fn from_physics(physics: &CarPhysics) -> Self {
        let front = physics
            .wheels
            .iter()
            .find(|wheel| wheel.is_front)
            .unwrap_or(&physics.wheels[0]);
        let rear = physics
            .wheels
            .iter()
            .find(|wheel| !wheel.is_front)
            .unwrap_or(&physics.wheels[2]);

        Self {
            chassis: ChassisTuning {
                inertia: physics.inertia,
                center_of_gravity: physics.center_of_gravity,
            },
            front: AxleTuning::from_wheel(front),
            rear: AxleTuning::from_wheel(rear),
            engine: EngineTuning {
                idle_rpm: physics.engine.idle_rpm,
                redline_rpm: physics.engine.redline_rpm,
                torque_curve: physics
                    .engine
                    .torque_curve
                    .iter()
                    .map(|point| [point.rpm, point.torque_nm])
                    .collect(),
            },
            drivetrain: DrivetrainTuning {
                gear_ratios: physics.drivetrain.gear_ratios.clone(),
                final_drive: physics.drivetrain.final_drive,
            },
            brakes: BrakesTuning {
                front: physics.brakes.max_torque[0],
                rear: physics.brakes.max_torque[2],
            },
            steering: SteeringTuning {
                max_angle: physics.steering.max_angle,
            },
            aero: AeroTuning {
                drag_coefficient: physics.aero.drag_coefficient,
                frontal_area: physics.aero.frontal_area,
            },
        }
    }

    pub fn sanitize(&mut self) {
        for value in &mut self.chassis.inertia {
            *value = finite_clamp(*value, 50.0, 10_000.0, 500.0);
        }
        for value in &mut self.chassis.center_of_gravity {
            *value = finite_clamp(*value, -2.0, 2.0, 0.0);
        }
        self.front.sanitize();
        self.rear.sanitize();

        self.engine.idle_rpm = finite_clamp(self.engine.idle_rpm, 400.0, 2_500.0, 900.0);
        self.engine.redline_rpm = finite_clamp(self.engine.redline_rpm, 1_500.0, 15_000.0, 7_500.0);
        if self.engine.redline_rpm <= self.engine.idle_rpm {
            self.engine.redline_rpm = (self.engine.idle_rpm + 1_000.0).min(15_000.0);
        }
        self.engine.torque_curve.retain(|point| {
            point[0].is_finite()
                && point[1].is_finite()
                && (500.0..=15_000.0).contains(&point[0])
                && (0.0..=2_000.0).contains(&point[1])
        });
        self.engine.torque_curve.truncate(32);
        self.engine
            .torque_curve
            .sort_by(|a, b| a[0].total_cmp(&b[0]));
        if self.engine.torque_curve.is_empty() {
            self.engine.torque_curve = vec![[1_000.0, 180.0], [3_000.0, 300.0], [6_000.0, 250.0]];
        }

        self.drivetrain
            .gear_ratios
            .retain(|ratio| ratio.is_finite());
        self.drivetrain.gear_ratios.truncate(12);
        for ratio in &mut self.drivetrain.gear_ratios {
            *ratio = finite_clamp(*ratio, 0.1, 8.0, 1.0);
        }
        if self.drivetrain.gear_ratios.is_empty() {
            self.drivetrain.gear_ratios.push(1.0);
        }
        self.drivetrain.final_drive = finite_clamp(self.drivetrain.final_drive, 1.0, 8.0, 3.7);
        self.brakes.front = finite_clamp(self.brakes.front, 100.0, 10_000.0, 2_000.0);
        self.brakes.rear = finite_clamp(self.brakes.rear, 100.0, 10_000.0, 1_400.0);
        self.steering.max_angle = finite_clamp(self.steering.max_angle, 0.05, 1.2, 0.6);
        self.aero.drag_coefficient = finite_clamp(self.aero.drag_coefficient, 0.05, 2.0, 0.3);
        self.aero.frontal_area = finite_clamp(self.aero.frontal_area, 0.5, 5.0, 1.85);
    }

    pub fn apply_to(&self, physics: &mut CarPhysics) {
        physics.inertia = self.chassis.inertia;
        physics.center_of_gravity = self.chassis.center_of_gravity;
        for wheel in &mut physics.wheels {
            let axle = if wheel.is_front {
                &self.front
            } else {
                &self.rear
            };
            wheel.suspension.travel = axle.suspension.travel;
            wheel.suspension.spring_rate = axle.suspension.spring_rate;
            wheel.suspension.damper_bump = axle.suspension.damper_bump;
            wheel.suspension.damper_rebound = axle.suspension.damper_rebound;
            wheel.suspension.anti_roll = axle.suspension.anti_roll;
            wheel.tire.lateral = axle.tire.lateral.to_coefficients();
            wheel.tire.longitudinal = axle.tire.longitudinal.to_coefficients();
            wheel.tire.relaxation_length = axle.tire.relaxation_length;
            wheel.tire.peak_slip_ratio = axle.tire.peak_slip_ratio;
            wheel.tire.friction_scale = axle.tire.friction_scale;
        }
        physics.engine.idle_rpm = self.engine.idle_rpm;
        physics.engine.redline_rpm = self.engine.redline_rpm;
        physics.engine.torque_curve = self
            .engine
            .torque_curve
            .iter()
            .map(|point| TorquePoint {
                rpm: point[0],
                torque_nm: point[1],
            })
            .collect();
        physics.drivetrain.gear_ratios = self.drivetrain.gear_ratios.clone();
        physics.drivetrain.final_drive = self.drivetrain.final_drive;
        physics.brakes.max_torque = [
            self.brakes.front,
            self.brakes.front,
            self.brakes.rear,
            self.brakes.rear,
        ];
        physics.steering.max_angle = self.steering.max_angle;
        physics.aero.drag_coefficient = self.aero.drag_coefficient;
        physics.aero.frontal_area = self.aero.frontal_area;
    }
}

impl AxleTuning {
    fn from_wheel(wheel: &crate::asset::WheelPhysics) -> Self {
        Self {
            suspension: SuspensionTuning {
                travel: wheel.suspension.travel,
                spring_rate: wheel.suspension.spring_rate,
                damper_bump: wheel.suspension.damper_bump,
                damper_rebound: wheel.suspension.damper_rebound,
                anti_roll: wheel.suspension.anti_roll,
            },
            tire: TireTuning {
                lateral: PacejkaTuning::from_coefficients(&wheel.tire.lateral),
                longitudinal: PacejkaTuning::from_coefficients(&wheel.tire.longitudinal),
                relaxation_length: wheel.tire.relaxation_length,
                peak_slip_ratio: wheel.tire.peak_slip_ratio,
                friction_scale: wheel.tire.friction_scale,
            },
        }
    }

    fn sanitize(&mut self) {
        self.suspension.travel = finite_clamp(self.suspension.travel, 0.02, 0.5, 0.15);
        self.suspension.spring_rate =
            finite_clamp(self.suspension.spring_rate, 5_000.0, 300_000.0, 55_000.0);
        self.suspension.damper_bump =
            finite_clamp(self.suspension.damper_bump, 500.0, 50_000.0, 7_000.0);
        self.suspension.damper_rebound =
            finite_clamp(self.suspension.damper_rebound, 500.0, 50_000.0, 8_000.0);
        self.suspension.anti_roll =
            finite_clamp(self.suspension.anti_roll, 0.0, 100_000.0, 10_000.0);
        self.tire.sanitize();
    }
}

impl TireTuning {
    fn sanitize(&mut self) {
        self.lateral.sanitize();
        self.longitudinal.sanitize();
        self.relaxation_length = finite_clamp(self.relaxation_length, 0.01, 3.0, 0.3);
        self.peak_slip_ratio = finite_clamp(self.peak_slip_ratio, 0.01, 1.0, 0.12);
        self.friction_scale = finite_clamp(self.friction_scale, 0.1, 2.0, 1.0);
    }
}

impl PacejkaTuning {
    fn from_coefficients(coefficients: &PacejkaCoefficients) -> Self {
        Self {
            b: coefficients.b,
            c: coefficients.c,
            d: coefficients.d,
            e: coefficients.e,
        }
    }

    fn to_coefficients(&self) -> PacejkaCoefficients {
        PacejkaCoefficients {
            b: self.b,
            c: self.c,
            d: self.d,
            e: self.e,
        }
    }

    fn sanitize(&mut self) {
        self.b = finite_clamp(self.b, 0.5, 30.0, 10.0);
        self.c = finite_clamp(self.c, 0.5, 3.0, 1.8);
        self.d = finite_clamp(self.d, 0.1, 2.5, 1.0);
        self.e = finite_clamp(self.e, 0.0, 2.0, 0.9);
    }
}

fn finite_clamp(value: f32, min: f32, max: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset::{
        AeroSpec, BrakeSpec, DriveLayout, DrivetrainSpec, EngineSpec, SteeringSpec, SuspensionSpec,
        TireSpec, WheelPhysics,
    };

    fn physics() -> CarPhysics {
        let suspension = SuspensionSpec {
            travel: 0.15,
            spring_rate: 55_000.0,
            damper_bump: 7_000.0,
            damper_rebound: 9_000.0,
            anti_roll: 18_000.0,
            ride_height: 0.34,
        };
        let tire = TireSpec {
            lateral: PacejkaCoefficients {
                b: 10.0,
                c: 1.9,
                d: 1.0,
                e: 0.97,
            },
            longitudinal: PacejkaCoefficients {
                b: 12.0,
                c: 1.65,
                d: 1.0,
                e: 0.9,
            },
            relaxation_length: 0.3,
            peak_slip_ratio: 0.12,
            peak_slip_angle: 0.12,
            friction_scale: 1.0,
        };
        let wheels = [
            ("FL", true, true, false),
            ("FR", true, false, false),
            ("RL", false, true, true),
            ("RR", false, false, true),
        ]
        .map(|(name, is_front, is_left, driven)| WheelPhysics {
            name: String::from(name),
            is_front,
            is_left,
            position: [
                if is_left { -0.7 } else { 0.7 },
                0.34,
                if is_front { -1.2 } else { 1.2 },
            ],
            radius: 0.33,
            width: 0.225,
            steerable: is_front,
            driven,
            suspension,
            tire,
        });
        CarPhysics {
            mass: 1_270.0,
            inertia: [650.0, 1_300.0, 450.0],
            center_of_gravity: [0.0, 0.4, 0.0],
            wheelbase: 2.425,
            track_width: [1.46, 1.46],
            wheels,
            engine: EngineSpec {
                idle_rpm: 900.0,
                redline_rpm: 8_000.0,
                torque_curve: vec![TorquePoint {
                    rpm: 4_500.0,
                    torque_nm: 314.0,
                }],
            },
            drivetrain: DrivetrainSpec {
                gear_ratios: vec![3.483, 2.015, 1.391, 1.0, 0.719, 0.6],
                final_drive: 4.1,
                clutch_max_torque: 400.0,
                drive_layout: DriveLayout::Rear,
            },
            brakes: BrakeSpec {
                max_torque: [2_200.0, 2_200.0, 1_400.0, 1_400.0],
                bias: 0.6,
            },
            steering: SteeringSpec {
                max_angle: 0.6,
                ratio: 14.0,
                caster: 0.09,
                kingpin_inclination: 0.12,
            },
            aero: AeroSpec {
                drag_coefficient: 0.29,
                frontal_area: 1.85,
                downforce_coefficient: 0.0,
            },
        }
    }

    #[test]
    fn setup_round_trip_and_apply() {
        let mut original = physics();
        let setup = CarTuning::from_physics(&original);
        let json = serde_json::to_string(&setup).unwrap();
        let parsed: CarTuning = serde_json::from_str(&json).unwrap();
        assert_eq!(setup, parsed);
        parsed.apply_to(&mut original);
        assert_eq!(CarTuning::from_physics(&original), setup);
        assert_eq!(original.wheels[0].tire.friction_scale, 1.0);
        assert_eq!(original.wheels[2].tire.friction_scale, 1.0);
    }

    #[test]
    fn sanitize_bounds_bad_values() {
        let mut setup = CarTuning::from_physics(&physics());
        setup.front.tire.friction_scale = f32::NAN;
        setup.front.suspension.spring_rate = 1.0;
        setup.engine.torque_curve.clear();
        setup.drivetrain.gear_ratios.clear();
        setup.sanitize();
        assert_eq!(setup.front.tire.friction_scale, 1.0);
        assert_eq!(setup.front.suspension.spring_rate, 5_000.0);
        assert!(!setup.engine.torque_curve.is_empty());
        assert_eq!(setup.drivetrain.gear_ratios, vec![1.0]);
    }
}
