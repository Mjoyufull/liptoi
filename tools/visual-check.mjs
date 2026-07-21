#!/usr/bin/env node

import { mkdir, writeFile } from "node:fs/promises";
import process from "node:process";

const bidiUrl = process.argv[2] ?? "ws://127.0.0.1:9222/session";
const siteUrl = process.argv[3] ?? "http://127.0.0.1:4173/";
const outputDirectory = process.argv[4] ?? "/tmp/liptoi-visual-check";
const socket = new WebSocket(bidiUrl);
const pending = new Map();
const browserErrors = [];
const screenshots = [];
const layouts = {};
let commandId = 0;

socket.addEventListener("message", (event) => {
  const message = JSON.parse(event.data);
  if (message.id !== undefined) {
    const request = pending.get(message.id);
    if (!request) return;
    pending.delete(message.id);
    if (message.type === "error") {
      request.reject(new Error(`${message.error}: ${message.message}`));
    } else {
      request.resolve(message.result);
    }
    return;
  }

  if (message.method === "log.entryAdded" && message.params.level === "error") {
    browserErrors.push(message.params.text ?? JSON.stringify(message.params));
  }
});

await new Promise((resolve, reject) => {
  socket.addEventListener("open", resolve, { once: true });
  socket.addEventListener("error", reject, { once: true });
});

function command(method, params = {}) {
  commandId += 1;
  return new Promise((resolve, reject) => {
    pending.set(commandId, { resolve, reject });
    socket.send(JSON.stringify({ id: commandId, method, params }));
  });
}

async function evaluate(context, expression) {
  const response = await command("script.evaluate", {
    expression,
    target: { context },
    awaitPromise: true,
    resultOwnership: "none",
  });
  if (response.type === "exception") throw new Error(response.exceptionDetails.text);
  return response.result.value;
}

async function waitFor(context, expression, timeoutMilliseconds = 10_000) {
  const deadline = Date.now() + timeoutMilliseconds;
  while (Date.now() < deadline) {
    if (await evaluate(context, expression)) return;
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  const state = await evaluate(
    context,
    `JSON.stringify({
      phase: document.querySelector('#terminal')?.dataset.gamePhase,
      source: document.querySelector('#terminal')?.dataset.controlSource,
      motion: document.querySelector('#terminal')?.dataset.motionStatus,
      profile: document.body.dataset.inputProfile,
      motionButton: document.querySelector('#motion-start')?.textContent,
      motionDisabled: document.querySelector('#motion-start')?.disabled,
      status: document.querySelector('#input-status')?.textContent,
    })`,
  );
  throw new Error(
    `timed out waiting for: ${expression}\nstate: ${state}\nbrowser errors: ${browserErrors.join(" | ")}`,
  );
}

async function capture(context, filename) {
  const screenshot = await command("browsingContext.captureScreenshot", {
    context,
    origin: "viewport",
    format: { type: "image/png" },
  });
  await writeFile(`${outputDirectory}/${filename}`, Buffer.from(screenshot.data, "base64"));
  screenshots.push(filename);
}

async function setViewport(context, width, height) {
  await command("browsingContext.setViewport", {
    context,
    viewport: { width, height },
    devicePixelRatio: 1,
  });
}

async function navigate(context) {
  await command("browsingContext.navigate", { context, url: siteUrl, wait: "complete" });
  await waitFor(
    context,
    "document.querySelector('#terminal canvas') !== null && document.querySelector('#boot-screen')?.hidden === true && document.querySelector('#terminal')?.dataset.gamePhase === 'ready'",
  );
}

async function inspectLayout(context, name) {
  const snapshot = JSON.parse(
    await evaluate(
      context,
      `JSON.stringify((() => {
        const terminal = document.querySelector('#terminal');
        const canvas = terminal.querySelector('canvas');
        const dock = document.querySelector('#control-dock');
        const rect = (element) => {
          const value = element.getBoundingClientRect();
          return { x: value.x, y: value.y, width: value.width, height: value.height };
        };
        return {
          viewport: { width: innerWidth, height: innerHeight, dpr: devicePixelRatio },
          terminal: rect(terminal),
          canvas: { ...rect(canvas), bufferWidth: canvas.width, bufferHeight: canvas.height },
          dock: rect(dock),
          profile: document.body.dataset.inputProfile,
          phase: terminal.dataset.gamePhase,
          source: terminal.dataset.controlSource,
          motion: terminal.dataset.motionStatus,
          basicLabel: document.querySelector('#basic-start').textContent,
          motionLabel: document.querySelector('#motion-start').textContent,
          motionHidden: document.querySelector('#motion-start').hidden,
          status: document.querySelector('#input-status').textContent,
          overflowX: document.documentElement.scrollWidth - innerWidth,
          overflowY: document.documentElement.scrollHeight - innerHeight,
        };
      })())`,
    ),
  );
  const near = (left, right) => Math.abs(left - right) <= 1;
  if (!near(snapshot.canvas.width, snapshot.terminal.width)) {
    throw new Error(`${name}: canvas width does not fill terminal: ${JSON.stringify(snapshot)}`);
  }
  if (!near(snapshot.canvas.height, snapshot.terminal.height)) {
    throw new Error(`${name}: canvas height does not fill terminal: ${JSON.stringify(snapshot)}`);
  }
  if (snapshot.canvas.bufferWidth < snapshot.canvas.width * snapshot.viewport.dpr) {
    throw new Error(`${name}: canvas backing buffer is undersized: ${JSON.stringify(snapshot)}`);
  }
  if (snapshot.canvas.bufferHeight < snapshot.canvas.height * snapshot.viewport.dpr) {
    throw new Error(`${name}: canvas backing buffer is undersized: ${JSON.stringify(snapshot)}`);
  }
  if (snapshot.overflowX > 0 || snapshot.overflowY > 0) {
    throw new Error(`${name}: page overflows its viewport: ${JSON.stringify(snapshot)}`);
  }
  layouts[name] = snapshot;
}

async function dispatchOrientation(context, beta, gamma) {
  await evaluate(
    context,
    `window.dispatchEvent(new DeviceOrientationEvent('deviceorientation', { beta: ${beta}, gamma: ${gamma} })); true`,
  );
}

async function pointer(context, type, xRatio, yRatio, pointerType) {
  await evaluate(
    context,
    `(() => {
      const terminal = document.querySelector('#terminal');
      const bounds = terminal.getBoundingClientRect();
      terminal.dispatchEvent(new PointerEvent('${type}', {
        bubbles: true,
        buttons: ${type === "pointerup" ? 0 : 1},
        clientX: bounds.left + bounds.width * ${xRatio},
        clientY: bounds.top + bounds.height * ${yRatio},
        pointerId: 7,
        pointerType: '${pointerType}',
      }));
      return true;
    })()`,
  );
}

await mkdir(outputDirectory, { recursive: true });
await command("session.new", { capabilities: {} });
await command("session.subscribe", { events: ["log.entryAdded"] });
const tree = await command("browsingContext.getTree");
const context = tree.contexts[0].context;

await setViewport(context, 1440, 900);
await navigate(context);
await waitFor(
  context,
  "document.body.dataset.inputProfile === 'desktop' && document.querySelector('#basic-start').textContent.includes('MOUSE') && document.querySelector('#motion-start').hidden",
);
await inspectLayout(context, "desktop-ready");
await capture(context, "ready-desktop.png");

await evaluate(context, "document.querySelector('#basic-start').click(); true");
await waitFor(context, "document.querySelector('#terminal')?.dataset.gamePhase === 'running'");
await pointer(context, "pointerdown", 0.72, 0.4, "mouse");
await waitFor(context, "document.querySelector('#terminal')?.dataset.controlSource === 'mouse'");
await pointer(context, "pointerup", 0.72, 0.4, "mouse");
await inspectLayout(context, "desktop-running");
await capture(context, "running-desktop.png");

await command("script.addPreloadScript", {
  contexts: [context],
  functionDeclaration: `() => {
    Object.defineProperty(Navigator.prototype, 'maxTouchPoints', {
      configurable: true,
      get: () => 5,
    });
    class MockDeviceOrientationEvent extends Event {
      constructor(type, init = {}) {
        super(type, init);
        this.beta = init.beta ?? null;
        this.gamma = init.gamma ?? null;
      }
      static requestPermission() {
        return Promise.resolve('granted');
      }
    }
    Object.defineProperty(window, 'DeviceOrientationEvent', {
      configurable: true,
      value: MockDeviceOrientationEvent,
    });
  }`,
});

await setViewport(context, 390, 844);
await navigate(context);
await waitFor(
  context,
  "document.body.dataset.inputProfile === 'touch' && !document.querySelector('#motion-start').hidden && document.querySelector('#basic-start').textContent.includes('TOUCH')",
);
await inspectLayout(context, "phone-portrait-ready");
await capture(context, "ready-phone-portrait.png");

await evaluate(
  context,
  `Object.defineProperty(window.DeviceOrientationEvent, 'requestPermission', {
    configurable: true,
    value: () => Promise.resolve('granted'),
  }); true`,
);
await evaluate(context, "document.querySelector('#motion-start').click(); true");
await waitFor(
  context,
  "document.querySelector('#terminal')?.dataset.motionStatus === 'listening' && document.querySelector('#terminal')?.dataset.gamePhase === 'ready'",
);
await dispatchOrientation(context, 42, 3);
await waitFor(
  context,
  "document.querySelector('#terminal')?.dataset.motionStatus === 'active' && document.querySelector('#terminal')?.dataset.gamePhase === 'running'",
);
await dispatchOrientation(context, 58, 17);
await waitFor(context, "document.querySelector('#terminal')?.dataset.controlSource === 'tilt'");
await new Promise((resolve) => setTimeout(resolve, 300));
await inspectLayout(context, "phone-portrait-tilt");
await capture(context, "running-phone-tilt.png");

await pointer(context, "pointerdown", 0.98, 0.98, "touch");
await waitFor(context, "document.querySelector('#terminal')?.dataset.controlSource === 'touch'");
await waitFor(
  context,
  "document.querySelector('#terminal')?.dataset.gamePhase === 'game-over'",
  10_000,
);
await pointer(context, "pointerup", 0.98, 0.98, "touch");
await dispatchOrientation(context, 64, 22);
await waitFor(context, "document.querySelector('#terminal')?.dataset.controlSource === 'tilt'");
await new Promise((resolve) => setTimeout(resolve, 750));
await capture(context, "game-over-phone-tilt-sand.png");

await setViewport(context, 844, 390);
await navigate(context);
await inspectLayout(context, "phone-landscape-ready");
await evaluate(context, "document.querySelector('#basic-start').click(); true");
await waitFor(context, "document.querySelector('#terminal')?.dataset.gamePhase === 'running'");
await pointer(context, "pointerdown", 0.6, 0.4, "touch");
await waitFor(context, "document.querySelector('#terminal')?.dataset.controlSource === 'touch'");
await pointer(context, "pointerup", 0.6, 0.4, "touch");
await inspectLayout(context, "phone-landscape-touch");
await capture(context, "running-phone-landscape.png");

const report = { outputDirectory, screenshots, layouts, browserErrors };
process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
await command("session.end");
socket.close();

if (browserErrors.length > 0) process.exitCode = 1;
