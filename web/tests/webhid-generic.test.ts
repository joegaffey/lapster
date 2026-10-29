import assert from "node:assert/strict";
import { test } from "node:test";

Object.defineProperty(globalThis, "navigator", {
  value: { hid: {} },
  configurable: true,
});

const { WebHIDDevice } = await import(
  "../src/vendor/webhid-wheel/webhid-wheel.js"
);

function fakeDevice(items: unknown[]) {
  return {
    collections: [
      {
        usagePage: 0x01,
        usage: 0x04,
        inputReports: [{ reportId: 0, items }],
      },
    ],
    opened: false,
    vendorId: 0x2341,
    productId: 0x8036,
    productName: "Arduino Handbrake",
    async open() {
      this.opened = true;
    },
    async close() {
      this.opened = false;
    },
    addEventListener() {},
    removeEventListener() {},
    oninputreport: null,
  };
}

test("generic HID splits multi-axis report items", async () => {
  const device = new WebHIDDevice();
  await device.connect(
    fakeDevice([
      {
        reportSize: 8,
        reportCount: 4,
        logicalMinimum: 0,
        logicalMaximum: 255,
        usages: [0x00010030, 0x00010031, 0x00010032, 0x00010033],
      },
    ]),
  );
  assert.equal(device.inputLayout.axes.length, 4);
  device.processReport({
    reportId: 0,
    timeStamp: 1,
    data: new DataView(Uint8Array.from([0, 64, 128, 255]).buffer),
  });
  assert.deepEqual(
    device.state.axes.map((value) => Math.round(value * 255)),
    [0, 64, 128, 255],
  );
});

test("generic HID reads Simulation Controls axes", async () => {
  const device = new WebHIDDevice();
  await device.connect(
    fakeDevice([
      {
        reportSize: 8,
        reportCount: 2,
        logicalMinimum: 0,
        logicalMaximum: 255,
        usages: [0x000200c5, 0x000200c6],
      },
    ]),
  );
  assert.equal(device.inputLayout.axes.length, 2);
  device.processReport({
    reportId: 0,
    timeStamp: 1,
    data: new DataView(Uint8Array.from([32, 224]).buffer),
  });
  assert.deepEqual(
    device.state.axes.map((value) => Math.round(value * 255)),
    [32, 224],
  );
});
