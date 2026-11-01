import { NodeIO, PropertyType } from "@gltf-transform/core";
import { ALL_EXTENSIONS } from "@gltf-transform/extensions";
import { dedup, flatten, join, mergeDocuments, prune, unpartition } from "@gltf-transform/functions";

const io = new NodeIO().registerExtensions(ALL_EXTENSIONS);
const [bodyPath, wheelPath, outPath] = process.argv.slice(2);
if (!bodyPath || !wheelPath || !outPath) {
  throw new Error("usage: node assemble.mjs <body.glb> <wheel.glb> <out.glb>");
}

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
const glassMaterial = bodyDoc
  .createMaterial("Glass")
  .setBaseColorFactor([0.035, 0.045, 0.055, 0.32])
  .setRoughnessFactor(0.1)
  .setMetallicFactor(0.0);

const glassNames = /GLASHOUSE|^FLGT/i;
for (const node of bodyRoot.listChildren()) {
  if (!node.getMesh() || !glassNames.test(node.getName())) continue;
  for (const primitive of node.getMesh().listPrimitives()) {
    primitive.setMaterial(glassMaterial);
  }
}

// Speed Dreams body: +X forward, +Y up, +Z right. Pre-rotate -90 degrees
// around Y so the existing GLB importer's 180-degree yaw lands on Lapster's
// -Z forward. The 0.21 m lift restores the source suspension ride height.
bodyRoot.setRotation([0, -Math.SQRT1_2, 0, Math.SQRT1_2]);
bodyRoot.setTranslation([0, 0.21, 0]);

const wheels = [
  { name: "WHEEL_FL", x: 1.23, y: 0.81, radius: 0.3471, width: 0.245, right: false },
  { name: "WHEEL_FR", x: 1.23, y: -0.81, radius: 0.3471, width: 0.245, right: true },
  { name: "WHEEL_RL", x: -1.5, y: 0.81, radius: 0.35025, width: 0.275, right: false },
  { name: "WHEEL_RR", x: -1.5, y: -0.81, radius: 0.35025, width: 0.275, right: true },
];

for (const wheel of wheels) {
  const node = bodyDoc
    .createNode(wheel.name)
    .setMesh(wheelMesh)
    .setTranslation([wheel.x, wheel.radius - 0.21, -wheel.y])
    .setScale([wheel.radius * 2, wheel.radius * 2, wheel.width]);
  if (wheel.right) node.setRotation([0, 1, 0, 0]);
  bodyRoot.addChild(node);
}

wheelSceneTarget.dispose();
bodyDoc.getRoot().setDefaultScene(bodyScene);
await bodyDoc.transform(unpartition());
await io.write(outPath, bodyDoc);
console.log(`wrote ${outPath}`);
