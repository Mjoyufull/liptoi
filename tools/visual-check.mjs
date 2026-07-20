#!/usr/bin/env node

import { mkdir, writeFile } from "node:fs/promises";
import process from "node:process";

const bidiUrl = process.argv[2] ?? "ws://127.0.0.1:9222/session";
const siteUrl = process.argv[3] ?? "http://127.0.0.1:4173/";
const outputDirectory = process.argv[4] ?? "/tmp/liptoi-visual-check";
const socket = new WebSocket(bidiUrl);
const pending = new Map();
const browserErrors = [];
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
  if (response.type === "exception") {
    throw new Error(response.exceptionDetails.text);
  }
  return response.result.value;
}

async function waitFor(context, expression, timeoutMilliseconds = 10_000) {
  const deadline = Date.now() + timeoutMilliseconds;
  while (Date.now() < deadline) {
    if (await evaluate(context, expression)) return;
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error(
    `timed out waiting for: ${expression}\nbrowser errors: ${browserErrors.join(" | ")}`,
  );
}

async function capture(context, filename) {
  const screenshot = await command("browsingContext.captureScreenshot", {
    context,
    origin: "viewport",
    format: { type: "image/png" },
  });
  await writeFile(`${outputDirectory}/${filename}`, Buffer.from(screenshot.data, "base64"));
}

async function setViewport(context, width, height) {
  await command("browsingContext.setViewport", {
    context,
    viewport: { width, height },
    devicePixelRatio: 1,
  });
}

await mkdir(outputDirectory, { recursive: true });
await command("session.new", { capabilities: {} });
await command("session.subscribe", { events: ["log.entryAdded"] });
const tree = await command("browsingContext.getTree");
const context = tree.contexts[0].context;

await setViewport(context, 1440, 900);
await command("browsingContext.navigate", {
  context,
  url: siteUrl,
  wait: "complete",
});
await waitFor(
  context,
  "document.querySelector('#terminal canvas') !== null && document.querySelector('#boot-screen')?.hidden === true && document.querySelector('#terminal')?.dataset.gamePhase === 'ready'",
);
await capture(context, "ready-desktop.png");

await evaluate(context, "document.querySelector('#motion-start').click(); true");
await waitFor(context, "document.querySelector('#terminal')?.dataset.gamePhase === 'running'");
await evaluate(
  context,
  `(() => {
    const orientation = (beta, gamma) => {
      const event = new Event('deviceorientation');
      Object.defineProperties(event, {
        beta: { value: beta },
        gamma: { value: gamma },
      });
      window.dispatchEvent(event);
    };
    orientation(38, 4);
    orientation(52, 20);
    return true;
  })()`,
);
await waitFor(context, "document.querySelector('#terminal')?.dataset.controlSource === 'tilt'");
await new Promise((resolve) => setTimeout(resolve, 1_100));
await capture(context, "running-desktop.png");
await evaluate(
  context,
  `(() => {
    const terminal = document.querySelector('#terminal');
    const bounds = terminal.getBoundingClientRect();
    terminal.dispatchEvent(new PointerEvent('pointerdown', {
      bubbles: true,
      buttons: 1,
      clientX: bounds.right - 2,
      clientY: bounds.bottom - 2,
      pointerId: 1,
      pointerType: 'mouse',
    }));
    return true;
  })()`,
);
await waitFor(
  context,
  "document.querySelector('#terminal')?.dataset.gamePhase === 'game-over'",
  9_000,
);
await new Promise((resolve) => setTimeout(resolve, 750));
await capture(context, "game-over-desktop.png");

await setViewport(context, 390, 844);
await command("browsingContext.navigate", {
  context,
  url: siteUrl,
  wait: "complete",
});
await waitFor(
  context,
  "document.querySelector('#terminal canvas') !== null && document.querySelector('#boot-screen')?.hidden === true && document.querySelector('#terminal')?.dataset.gamePhase === 'ready'",
);
await new Promise((resolve) => setTimeout(resolve, 300));
await capture(context, "ready-mobile.png");

const report = {
  outputDirectory,
  screenshots: [
    "ready-desktop.png",
    "running-desktop.png",
    "game-over-desktop.png",
    "ready-mobile.png",
  ],
  browserErrors,
};
process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
await command("session.end");
socket.close();

if (browserErrors.length > 0) process.exitCode = 1;
