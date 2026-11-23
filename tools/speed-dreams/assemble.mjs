import fs from "node:fs";
import path from "node:path";

import { NodeIO, PropertyType } from "@gltf-transform/core";
import { ALL_EXTENSIONS } from "@gltf-transform/extensions";
import { dedup, flatten, join, mergeDocuments, prune, unpartition } from "@gltf-transform/functions";

const io = new NodeIO().registerExtensions(ALL_EXTENSIONS);

function positionKey(position) {
  return position
    .map((value) => (value === 0 ? "0.0000" : value.toFixed(4)))
    .join(",");
}

// AC3D poly objects store positions followed by per-vertex normals. Assimp
// drops those normals when exporting the wheel to GLB and duplicates every
// surface with reversed winding, which leaves zero normals and back-facing
// triangles. Keep the source normals for the outward copy and remove the
// redundant reverse/degenerate triangles.
function readAccNormals(accPath) {
  if (!accPath || !fs.existsSync(accPath)) return null;
  const lines = fs.readFileSync(accPath, "utf8").split(/\r?\n/);
  const normals = new Map();
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index].trim();
    if (!line.startsWith("numvert")) continue;
    const count = Number.parseInt(line.split(/\s+/)[1], 10);
    if (!Number.isFinite(count) || count <= 0) continue;
    for (let vertex = 1; vertex <= count; vertex += 1) {
      const values = lines[index + vertex]?.trim().split(/\s+/).map(Number);
      if (!values || values.length < 6 || !values.every(Number.isFinite)) continue;
      normals.set(positionKey(values.slice(0, 3)), values.slice(3, 6));
    }
    index += count;
  }
  return normals;
}

function faceNormal(positions, a, b, c) {
  const pa = positions[a];
  const pb = positions[b];
  const pc = positions[c];
  const ab = [pb[0] - pa[0], pb[1] - pa[1], pb[2] - pa[2]];
  const ac = [pc[0] - pa[0], pc[1] - pa[1], pc[2] - pa[2]];
  return [
    ab[1] * ac[2] - ab[2] * ac[1],
    ab[2] * ac[0] - ab[0] * ac[2],
    ab[0] * ac[1] - ab[1] * ac[0],
  ];
}

function repairWheelNormals(document, mesh, sourceNormals) {
  if (!sourceNormals) return;
  for (const primitive of mesh.listPrimitives()) {
    const position = primitive.getAttribute("POSITION");
    const indicesAttribute = primitive.getIndices();
    if (!position) continue;
    const vertexCount = position.getCount();
    const indices = indicesAttribute
      ? Array.from({ length: indicesAttribute.getCount() }, (_, index) =>
          indicesAttribute.getScalar(index),
        )
      : Array.from({ length: vertexCount }, (_, index) => index);
    if (indices.length === 0) continue;

    const positions = Array.from({ length: vertexCount }, (_, index) =>
      position.getElement(index, []),
    );
    const source = positions.map((value) => {
      const normal = sourceNormals.get(positionKey(value));
      return normal && Number.isFinite(normal[0]) && Number.isFinite(normal[1]) &&
        Number.isFinite(normal[2]) && Math.hypot(...normal) > 1e-6
        ? normal
        : null;
    });

    const triangles = [];
    const groups = new Map();
    for (let index = 0; index + 2 < indices.length; index += 3) {
      const ids = [indices[index], indices[index + 1], indices[index + 2]];
      const normal = faceNormal(positions, ids[0], ids[1], ids[2]);
      const length = Math.hypot(...normal);
      if (length <= 1e-12) continue; // Drop zero-area triangles with no surface.
      let dot = null;
      const vertexNormals = ids.map((id) => source[id]);
      if (vertexNormals.every(Boolean)) {
        const average = vertexNormals.reduce(
          (sum, value) => [sum[0] + value[0], sum[1] + value[1], sum[2] + value[2]],
          [0, 0, 0],
        );
        const averageLength = Math.hypot(...average);
        if (averageLength > 1e-9) {
          dot =
            (normal[0] * average[0] + normal[1] * average[1] + normal[2] * average[2]) /
            (length * averageLength);
        }
      }
      const triangle = { ids, dot };
      triangles.push(triangle);
      const key = [...ids].sort((a, b) => a - b).join(",");
      const group = groups.get(key);
      if (group) group.push(triangle);
      else groups.set(key, [triangle]);
    }

    // Only repair meshes that actually contain reverse-winding duplicates.
    // This keeps the healthy wheel assets (single-sided, valid normals) on the
    // existing bake path.
    if (![...groups.values()].some((group) => group.length > 1)) continue;

    const kept = [];
    for (const group of groups.values()) {
      if (group.length === 1) {
        kept.push(group[0]);
        continue;
      }
      // Reverse-winding duplicates share the same vertex set. Keep the copy
      // that agrees with the source normals, or the first copy when the
      // source normals are unavailable.
      group.sort((a, b) => (b.dot ?? -1) - (a.dot ?? -1));
      kept.push(group[0]);
    }
    if (kept.length === 0) continue;

    const keptIndices = new Uint32Array(kept.length * 3);
    const sums = new Float64Array(vertexCount * 3);
    kept.forEach((triangle, triangleIndex) => {
      const normal = faceNormal(positions, triangle.ids[0], triangle.ids[1], triangle.ids[2]);
      triangle.ids.forEach((id, corner) => {
        keptIndices[triangleIndex * 3 + corner] = id;
        sums[id * 3] += normal[0];
        sums[id * 3 + 1] += normal[1];
        sums[id * 3 + 2] += normal[2];
      });
    });

    const normals = new Float32Array(vertexCount * 3);
    for (let vertex = 0; vertex < vertexCount; vertex += 1) {
      let normal = source[vertex];
      if (!normal) {
        const length = Math.hypot(
          sums[vertex * 3],
          sums[vertex * 3 + 1],
          sums[vertex * 3 + 2],
        );
        normal =
          length > 1e-9
            ? [
                sums[vertex * 3] / length,
                sums[vertex * 3 + 1] / length,
                sums[vertex * 3 + 2] / length,
              ]
            : [0, 1, 0];
      }
      normals.set(normal, vertex * 3);
    }

    primitive.setAttribute(
      "NORMAL",
      document.createAccessor().setArray(normals).setType("VEC3"),
    );
    primitive.setIndices(
      document.createAccessor().setArray(keptIndices).setType("SCALAR"),
    );
  }
}
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
const sourceWheelPath = config.wheel
  ? path.resolve(path.dirname(configPath), config.wheel)
  : null;
repairWheelNormals(bodyDoc, wheelMesh, readAccNormals(sourceWheelPath));

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
