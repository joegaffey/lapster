import fs from "node:fs";

import { NodeIO, PropertyType } from "@gltf-transform/core";
import { ALL_EXTENSIONS } from "@gltf-transform/extensions";
import { dedup, flatten, join, mergeDocuments, prune, unpartition } from "@gltf-transform/functions";

const io = new NodeIO().registerExtensions(ALL_EXTENSIONS);
const [configPath, bodyPath, wheelPath, outPath] = process.argv.slice(2);
if (!configPath || !bodyPath || !wheelPath || !outPath) {
  throw new Error("usage: node assemble.mjs <import.json> <body.glb> <wheel.glb> <out.glb>");
}

const config = JSON.parse(fs.readFileSync(configPath, "utf8"));
const bodyDoc = await io.read(bodyPath);
const wheelDoc = await io.read(wheelPath);
await wheelDoc.transform(
  dedup({ propertyTypes: [PropertyType.MATERIAL] }),
  flatten(),
  join({ keepNamed: false }),
  prune(),
);
const bodyScene = bodyDoc.getRoot().getDefaultScene() || bodyDoc.getRoot().listScenes()[0];
const bodyRoot = bodyScene.listChildren()[0];
if (!bodyRoot) throw new Error("body scene has no root node");

const map = mergeDocuments(bodyDoc, wheelDoc);
const wheelScene = wheelDoc.getRoot().getDefaultScene() || wheelDoc.getRoot().listScenes()[0];
const wheelRootSource = wheelScene.listChildren()[0];
if (!wheelRootSource || !wheelRootSource.getMesh()) {
  throw new Error("wheel scene root has no mesh");
}
const wheelRoot = map.get(wheelRootSource);
const wheelSceneTarget = map.get(wheelScene);
const wheelMesh = wheelRoot.getMesh();

const excludePatterns = (config.exclude_nodes ?? []).map(
  (pattern) => new RegExp(pattern, "i"),
);
for (const node of bodyRoot.listChildren()) {
  if (!excludePatterns.some((pattern) => pattern.test(node.getName()))) continue;
  const mesh = node.getMesh();
  node.dispose();
  if (mesh) mesh.dispose();
}

const glassMaterial = bodyDoc
  .createMaterial("Glass")
  .setBaseColorFactor([0.035, 0.045, 0.055, 0.32])
  .setRoughnessFactor(0.1)
  .setMetallicFactor(0.0);

const glassPattern = new RegExp(config.glass_names.join("|"), "i");
for (const node of bodyRoot.listChildren()) {
  if (!node.getMesh() || !glassPattern.test(node.getName())) continue;
  for (const primitive of node.getMesh().listPrimitives()) {
    primitive.setMaterial(glassMaterial);
  }
}

// Speed Dreams body: +X forward, +Y up, +Z right. Pre-rotate -90 degrees
// around Y so the existing GLB importer's 180-degree yaw lands on Lapster's
// -Z forward. The body lift restores the source suspension ride height.
bodyRoot.setRotation([0, -Math.SQRT1_2, 0, Math.SQRT1_2]);
bodyRoot.setTranslation([0, config.body_lift, 0]);

const wheels = [
  { name: "WHEEL_FL", axle: config.front, side: -1, right: false },
  { name: "WHEEL_FR", axle: config.front, side: 1, right: true },
  { name: "WHEEL_RL", axle: config.rear, side: -1, right: false },
  { name: "WHEEL_RR", axle: config.rear, side: 1, right: true },
];

for (const wheel of wheels) {
  const { axle_x, half_track, radius, width } = wheel.axle;
  const node = bodyDoc
    .createNode(wheel.name)
    .setMesh(wheelMesh)
    .setTranslation([
      axle_x,
      radius - config.body_lift,
      wheel.side * half_track,
    ])
    .setScale([radius * 2, radius * 2, width]);
  if (wheel.right) node.setRotation([0, 1, 0, 0]);
  bodyRoot.addChild(node);
}

wheelSceneTarget.dispose();

bodyDoc.getRoot().setDefaultScene(bodyScene);
await bodyDoc.transform(unpartition());
await io.write(outPath, bodyDoc);
console.log(`wrote ${outPath}`);
