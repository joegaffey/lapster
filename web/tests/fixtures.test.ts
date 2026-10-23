import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const carUrl = new URL("../../fixtures/cars/minimal/car.json", import.meta.url);
const trackUrl = new URL("../../fixtures/tracks/minimal/track.json", import.meta.url);

async function loadFixture(url: URL) {
  const data = await readFile(url, "utf8");
  return JSON.parse(data);
}

test("car fixture follows the source contract", async () => {
  const car = await loadFixture(carUrl);

  assert.equal(car.schema, "sim-source-v1");
  assert.equal(car.kind, "car");
  assert.equal(car.units.length, "m");
  assert.equal(car.units.mass, "kg");
  assert.equal(car.frame.handedness, "right");
  assert.equal(car.frame.up, "+Y");
  assert.equal(car.frame.forward, "-Z");

  assert.equal(car.physics.wheels.length, 4);
  assert.deepEqual(
    car.physics.wheels.map((wheel: { name: string }) => wheel.name),
    ["FL", "FR", "RL", "RR"],
  );

  for (const wheel of car.physics.wheels) {
    const [x, , z] = wheel.position;
    if (wheel.name === "FL" || wheel.name === "FR") {
      assert.equal(wheel.is_front, true);
      assert.equal(z < 0, true);
    } else {
      assert.equal(wheel.is_front, false);
      assert.equal(z > 0, true);
    }
    if (wheel.name === "FL" || wheel.name === "RL") {
      assert.equal(wheel.is_left, true);
      assert.equal(x < 0, true);
    } else {
      assert.equal(wheel.is_left, false);
      assert.equal(x > 0, true);
    }
  }

  assert.equal(car.physics.wheels[0].steerable, true);
  assert.equal(car.physics.wheels[1].steerable, true);
  assert.equal(car.physics.wheels[2].steerable, false);
  assert.equal(car.physics.wheels[3].steerable, false);
  assert.equal(car.physics.wheels[0].driven, false);
  assert.equal(car.physics.wheels[1].driven, false);
  assert.equal(car.physics.wheels[2].driven, true);
  assert.equal(car.physics.wheels[3].driven, true);

  assert.equal(car.physics.drivetrain.drive_layout, "Rear");
  assert.equal(car.physics.engine.torque_curve.length >= 2, true);
  assert.equal(car.physics.drivetrain.gear_ratios.length >= 2, true);

  assert.equal(car.visual.wheels.length, 4);
  assert.equal(car.visual.meshes.length, 3);
  assert.equal(car.visual.materials.length, 3);

  for (let index = 0; index < car.visual.wheels.length; index += 1) {
    const visual = car.visual.wheels[index];
    const physics = car.physics.wheels[index];
    assert.equal(visual.name, physics.name);
    assert.deepEqual(visual.transform.translation, physics.position);
    assert.equal(visual.mesh >= 0 && visual.mesh < car.visual.meshes.length, true);
  }

  const materialNames = new Set(
    car.visual.materials.map((material: { name: string }) => material.name),
  );
  for (const primitive of car.visual.primitives) {
    assert.equal(materialNames.has(primitive.material), true);
  }
});

test("track fixture follows the source contract", async () => {
  const track = await loadFixture(trackUrl);

  assert.equal(track.schema, "sim-source-v1");
  assert.equal(track.kind, "track");
  assert.equal(track.units.length, "m");
  assert.equal(track.frame.up, "+Y");
  assert.equal(track.frame.forward, "-Z");

  assert.equal(track.metadata.spawns.length >= 1, true);
  assert.equal(track.metadata.surfaces.some((s: { name: string }) => s.name === "asphalt"), true);
  assert.equal(track.metadata.surfaces.some((s: { name: string }) => s.name === "wall"), true);
  assert.equal(track.metadata.surfaces.some((s: { name: string }) => s.name === "line"), true);

  const surfaceIds = new Set(
    track.metadata.surfaces.map((surface: { id: number }) => surface.id),
  );
  for (const primitive of track.collision.primitives) {
    assert.equal(surfaceIds.has(primitive.surface), true);
  }

  const visualIds = new Set(
    track.visual.primitives.map((primitive: { id: string }) => primitive.id),
  );
  for (const id of ["ground", "wall_left", "wall_right", "start_line", "gauge_sign", "axis_x", "axis_y", "axis_neg_z"]) {
    assert.equal(visualIds.has(id), true);
  }

  const materialNames = new Set(
    track.visual.materials.map((material: { name: string }) => material.name),
  );
  for (const primitive of track.visual.primitives) {
    assert.equal(materialNames.has(primitive.material), true);
  }

  assert.equal(track.visual.textures.length, 1);
  assert.equal(track.visual.textures[0].generator, "gauge");
  assert.equal(
    track.visual.materials.find((material: { name: string }) => material.name === "sign")
      .base_color_texture,
    0,
  );

  const spawn = track.metadata.spawns[0];
  assert.deepEqual(spawn.position, [0.0, 0.0, 0.0]);
  assert.deepEqual(spawn.rotation_xyzw, [0.0, 0.0, 0.0, 1.0]);
});
