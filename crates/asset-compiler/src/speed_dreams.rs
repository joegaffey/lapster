use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use anyhow::{bail, Context, Result};
use sim_types::{
    AeroSpec, BrakeSpec, CarPhysics, DriveLayout, EngineSpec, SteeringSpec, SuspensionSpec,
    TorquePoint,
};
use xml::reader::{EventReader, XmlEvent};

struct Node {
    name: String,
    attrs: HashMap<String, String>,
    children: Vec<Node>,
}

impl Node {
    fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.get(name).map(String::as_str)
    }

    fn child_section(&self, name: &str) -> Option<&Node> {
        self.children
            .iter()
            .find(|node| node.name == "section" && node.attr("name") == Some(name))
    }

    fn section(&self, path: &[&str]) -> Option<&Node> {
        let mut node = self;
        for name in path {
            node = node.child_section(name)?;
        }
        Some(node)
    }

    fn attnum(&self, name: &str) -> Option<f32> {
        self.children
            .iter()
            .find(|node| node.name == "attnum" && node.attr("name") == Some(name))
            .and_then(|node| node.attr("val"))
            .and_then(|value| value.parse().ok())
    }

    fn attstr(&self, name: &str) -> Option<&str> {
        self.children
            .iter()
            .find(|node| node.name == "attstr" && node.attr("name") == Some(name))
            .and_then(|node| node.attr("val"))
    }
}

fn parse_xml(path: &Path) -> Result<Node> {
    let file = File::open(path)
        .with_context(|| format!("failed to open Speed Dreams XML {}", path.display()))?;
    let parser = EventReader::new(BufReader::new(file));
    let mut stack = Vec::new();
    let mut root = None;
    for event in parser {
        match event.context("failed to parse Speed Dreams XML")? {
            XmlEvent::StartElement {
                name, attributes, ..
            } => {
                stack.push(Node {
                    name: name.local_name,
                    attrs: attributes
                        .into_iter()
                        .map(|attribute| (attribute.name.local_name, attribute.value))
                        .collect(),
                    children: Vec::new(),
                });
            }
            XmlEvent::EndElement { .. } => {
                let Some(node) = stack.pop() else {
                    bail!("unexpected closing tag in Speed Dreams XML");
                };
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                } else {
                    root = Some(node);
                }
            }
            _ => {}
        }
    }
    root.context("Speed Dreams XML has no root element")
}

fn require_attnum(node: &Node, name: &str) -> Result<f32> {
    node.attnum(name)
        .with_context(|| format!("missing numeric attribute '{name}'"))
}

fn wheel_section(root: &Node, front: bool, left: bool) -> Result<&Node> {
    let name = match (front, left) {
        (true, true) => "Front Left Wheel",
        (true, false) => "Front Right Wheel",
        (false, true) => "Rear Left Wheel",
        (false, false) => "Rear Right Wheel",
    };
    root.child_section(name)
        .with_context(|| format!("missing section '{name}'"))
}

fn suspension_section(root: &Node, front: bool, left: bool) -> Result<&Node> {
    let name = match (front, left) {
        (true, true) => "Front Left Suspension",
        (true, false) => "Front Right Suspension",
        (false, true) => "Rear Left Suspension",
        (false, false) => "Rear Right Suspension",
    };
    root.child_section(name)
        .with_context(|| format!("missing section '{name}'"))
}

fn brake_section(root: &Node, front: bool, left: bool) -> Result<&Node> {
    let name = match (front, left) {
        (true, true) => "Front Left Brake",
        (true, false) => "Front Right Brake",
        (false, true) => "Rear Left Brake",
        (false, false) => "Rear Right Brake",
    };
    root.child_section(name)
        .with_context(|| format!("missing section '{name}'"))
}

fn anti_roll_section(root: &Node, front: bool) -> Result<&Node> {
    let name = if front {
        "Front Anti-Roll Bar"
    } else {
        "Rear Anti-Roll Bar"
    };
    root.child_section(name)
        .with_context(|| format!("missing section '{name}'"))
}

fn engine_torque_curve(engine: &Node) -> Result<Vec<TorquePoint>> {
    let data = engine
        .child_section("data points")
        .context("missing engine data points")?;
    let mut points = data
        .children
        .iter()
        .filter(|node| node.name == "section")
        .filter_map(|node| {
            let rpm = node.attnum("rpm")?;
            let torque = node.attnum("Tq")?;
            Some(TorquePoint {
                rpm,
                torque_nm: torque,
            })
        })
        .collect::<Vec<_>>();
    points.sort_by(|a, b| a.rpm.total_cmp(&b.rpm));
    if points.is_empty() {
        bail!("engine data points contain no rpm/torque pairs");
    }
    Ok(points)
}

fn inertia_from_geometry(physics: &CarPhysics) -> [f32; 3] {
    let track = (physics.track_width[0] + physics.track_width[1]) * 0.5;
    let length = physics.wheelbase;
    [
        physics.mass * length * length / 12.0,
        physics.mass * (length * length + track * track) / 12.0,
        physics.mass * track * track / 12.0,
    ]
}

pub fn apply_physics(physics: &mut CarPhysics, path: &Path) -> Result<()> {
    let root = parse_xml(path)?;

    let car = root.child_section("Car").context("missing Car section")?;
    let mass = require_attnum(car, "mass")?;
    let cg_height = require_attnum(car, "GC height")?;
    let front_share = require_attnum(car, "front-rear weight repartition")?;
    let front_z = (physics.wheels[0].position[2] + physics.wheels[1].position[2]) * 0.5;
    physics.mass = mass;
    physics.center_of_gravity = [
        0.0,
        cg_height,
        front_z + (1.0 - front_share) * physics.wheelbase,
    ];
    physics.inertia = inertia_from_geometry(physics);

    let aero = root
        .section(&["Aerodynamics"])
        .context("missing Aerodynamics")?;
    let front_lift = require_attnum(aero, "front Clift")?;
    let rear_lift = require_attnum(aero, "rear Clift")?;
    physics.aero = AeroSpec {
        drag_coefficient: require_attnum(aero, "Cx")?,
        frontal_area: require_attnum(aero, "front area")?,
        downforce_coefficient: front_lift + rear_lift,
    };

    let engine = root.section(&["Engine"]).context("missing Engine")?;
    let torque_curve = engine_torque_curve(engine)?;
    let clutch_max_torque = torque_curve
        .iter()
        .map(|point| point.torque_nm)
        .fold(0.0f32, f32::max)
        * 1.2;
    physics.engine = EngineSpec {
        idle_rpm: require_attnum(engine, "tickover")?,
        redline_rpm: require_attnum(engine, "revs maxi")?,
        torque_curve,
    };

    let gearbox = root
        .section(&["Gearbox", "gears"])
        .context("missing Gearbox")?;
    let mut gear_ratios = gearbox
        .children
        .iter()
        .filter(|node| {
            node.name == "section"
                && node
                    .attr("name")
                    .is_some_and(|name| name != "r" && name.parse::<u32>().is_ok())
        })
        .map(|node| require_attnum(node, "ratio"))
        .collect::<Result<Vec<_>>>()?;
    gear_ratios.sort_by(|a, b| b.total_cmp(a));
    if gear_ratios.is_empty() {
        bail!("gearbox has no forward gears");
    }

    let drivetrain = root
        .section(&["Rear Differential"])
        .context("missing Rear Differential")?;
    let layout = root
        .section(&["Drivetrain"])
        .and_then(|section| section.attstr("type"))
        .unwrap_or("RWD");
    physics.drivetrain = sim_types::DrivetrainSpec {
        gear_ratios,
        final_drive: require_attnum(drivetrain, "ratio")?,
        clutch_max_torque,
        drive_layout: match layout {
            "FWD" => DriveLayout::Front,
            "4WD" => DriveLayout::All,
            _ => DriveLayout::Rear,
        },
    };

    let brakes = root
        .section(&["Brake System"])
        .context("missing Brake System")?;
    let brake_pressure = require_attnum(brakes, "max pressure")? * 1_000.0;
    let brake_bias = require_attnum(brakes, "front-rear brake repartition")?;

    let steer = root.section(&["Steer"]).context("missing Steer")?;
    let max_angle = require_attnum(steer, "steer lock")?.to_radians();
    let wheel_rotation = require_attnum(steer, "steering wheel rotation")?;
    physics.steering = SteeringSpec {
        max_angle,
        ratio: (wheel_rotation / max_angle.to_degrees()).max(1.0),
        ..physics.steering
    };

    let mut max_torque = [0.0; 4];
    for (index, wheel) in physics.wheels.iter_mut().enumerate() {
        let source = wheel_section(&root, wheel.is_front, wheel.is_left)?;
        let suspension = suspension_section(&root, wheel.is_front, wheel.is_left)?;
        let brake = brake_section(&root, wheel.is_front, wheel.is_left)?;
        let anti_roll = anti_roll_section(&root, wheel.is_front)?;

        let bellcrank = require_attnum(suspension, "bellcrank")?.max(0.1);
        let course = require_attnum(suspension, "suspension course")?;
        let spring = require_attnum(suspension, "spring")? * 1_000.0;
        wheel.suspension = SuspensionSpec {
            travel: course / bellcrank,
            spring_rate: spring * bellcrank * bellcrank,
            damper_bump: require_attnum(suspension, "slow bump")? * 1_000.0,
            damper_rebound: require_attnum(suspension, "slow rebound")? * 1_000.0,
            anti_roll: require_attnum(anti_roll, "spring")? * 1_000.0,
            ride_height: require_attnum(source, "ride height")? / 1_000.0,
        };
        wheel.tire.friction_scale = require_attnum(source, "mu")?;

        let piston_area = require_attnum(brake, "piston area")? / 10_000.0;
        let disk_radius = require_attnum(brake, "disk diameter")? / 2_000.0;
        let pad_mu = require_attnum(brake, "mu").unwrap_or(0.4);
        max_torque[index] = brake_pressure * piston_area * pad_mu * disk_radius;
    }
    physics.brakes = BrakeSpec {
        max_torque,
        bias: brake_bias,
    };

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source_physics() -> CarPhysics {
        sim_types::CarPhysics {
            mass: 1_270.0,
            inertia: [650.0, 1_300.0, 450.0],
            center_of_gravity: [0.0, 0.40, 0.0],
            wheelbase: 2.73,
            track_width: [1.62, 1.62],
            wheels: std::array::from_fn(|index| sim_types::WheelPhysics {
                name: ["FL", "FR", "RL", "RR"][index].to_string(),
                is_front: index < 2,
                is_left: index % 2 == 0,
                position: if index < 2 {
                    [-0.81, 0.347, -1.23]
                } else {
                    [-0.81, 0.350, 1.5]
                },
                radius: 0.347,
                width: 0.245,
                steerable: index < 2,
                driven: index >= 2,
                suspension: SuspensionSpec {
                    travel: 0.15,
                    spring_rate: 55_000.0,
                    damper_bump: 7_000.0,
                    damper_rebound: 9_000.0,
                    anti_roll: 18_000.0,
                    ride_height: 0.347,
                },
                tire: sim_types::TireSpec {
                    lateral: sim_types::PacejkaCoefficients {
                        b: 10.0,
                        c: 1.9,
                        d: 1.0,
                        e: 0.97,
                    },
                    longitudinal: sim_types::PacejkaCoefficients {
                        b: 12.0,
                        c: 1.65,
                        d: 1.0,
                        e: 0.9,
                    },
                    relaxation_length: 0.3,
                    peak_slip_ratio: 0.12,
                    peak_slip_angle: 0.12,
                    friction_scale: 1.0,
                },
            }),
            engine: EngineSpec {
                idle_rpm: 900.0,
                redline_rpm: 8_000.0,
                torque_curve: Vec::new(),
            },
            drivetrain: sim_types::DrivetrainSpec {
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
    fn maps_deckard_conejo_xml_physics() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/sources/cars/sc-deckard-conejo/body/sc-deckard-conejo.xml");
        let mut physics = source_physics();
        apply_physics(&mut physics, &path).unwrap();

        assert_eq!(physics.mass, 1_700.0);
        assert!((physics.center_of_gravity[1] - 0.30).abs() < 1e-6);
        assert!((physics.center_of_gravity[2] - 0.0804).abs() < 0.001);
        assert_eq!(physics.engine.idle_rpm, 1_000.0);
        assert_eq!(physics.engine.redline_rpm, 6_930.0);
        assert_eq!(physics.engine.torque_curve.len(), 21);
        assert_eq!(physics.engine.torque_curve[12].torque_nm, 562.65);
        assert_eq!(
            physics.drivetrain.gear_ratios,
            vec![3.01, 2.07, 1.43, 1.0, 0.84, 0.57]
        );
        assert_eq!(physics.drivetrain.final_drive, 3.8);
        assert!((physics.steering.max_angle - 45.0f32.to_radians()).abs() < 1e-6);
        assert_eq!(physics.wheels[0].tire.friction_scale, 1.3);
        assert!((physics.wheels[0].suspension.spring_rate - 67_787.2).abs() < 1.0);
        assert!((physics.wheels[2].suspension.spring_rate - 68_501.0).abs() < 1.0);
        assert!((physics.wheels[0].suspension.anti_roll - 27_000.0).abs() < 1.0);
        assert!((physics.wheels[2].suspension.anti_roll - 42_000.0).abs() < 1.0);
        assert!(physics.brakes.max_torque[0] > physics.brakes.max_torque[2]);
    }
}
