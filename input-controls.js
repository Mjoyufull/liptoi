const motionButton = document.querySelector("#motion-start");
const basicButton = document.querySelector("#basic-start");
const recenterButton = document.querySelector("#recenter");
const inputStatus = document.querySelector("#input-status");
const bootScreen = document.querySelector("#boot-screen");
const terminal = document.querySelector("#terminal");

const touchCapable =
  navigator.maxTouchPoints > 0 || window.matchMedia("(any-pointer: coarse)").matches;
const phoneLike =
  touchCapable &&
  (window.matchMedia("(pointer: coarse)").matches ||
    Math.min(window.innerWidth, window.innerHeight) <= 900);
const motionCapable = phoneLike && window.isSecureContext && "DeviceOrientationEvent" in window;
let motionState = motionCapable ? "waiting" : "unavailable";
let motionAttempt = 0;

document.body.dataset.inputProfile = touchCapable ? "touch" : "desktop";

const sendMotionStatus = (status) => {
  window.dispatchEvent(new CustomEvent("liptoi-motion-status", { detail: status }));
};

const focusTerminal = () => {
  document.querySelector("#terminal [tabindex]")?.focus({ preventScroll: true });
};

const startGame = () => {
  window.dispatchEvent(new Event("liptoi-start"));
  focusTerminal();
};

const sourceLabel = () => {
  const source = terminal.dataset.controlSource;
  if (source && source !== "idle") return source.toUpperCase();
  return touchCapable ? "TOUCH / KEYS" : "MOUSE / KEYS";
};

const syncDock = () => {
  const phase = terminal.dataset.gamePhase ?? "ready";
  basicButton.hidden = phase !== "ready";
  motionButton.hidden = !motionCapable || motionState === "active" || motionState === "denied";
  recenterButton.hidden = motionState !== "active";

  if (motionState === "listening") {
    inputStatus.textContent = "MOVE PHONE · WAITING FOR A REAL SENSOR SAMPLE";
  } else if (motionState === "denied") {
    inputStatus.textContent = "TILT DENIED · TOUCH + KEYS STILL WORK";
  } else if (motionState === "timeout") {
    inputStatus.textContent = "NO GYRO DATA · TOUCH + KEYS STILL WORK";
  } else if (motionState === "active") {
    inputStatus.textContent = `AUTO SWITCH · ACTIVE: ${sourceLabel()}`;
  } else {
    inputStatus.textContent = `${sourceLabel()} · AUTO SWITCH`;
  }

  if (!motionButton.disabled) {
    motionButton.textContent = phase === "ready" ? "ENABLE TILT + START" : "ENABLE TILT";
  }
};

const waitForMotionSample = (attempt) =>
  new Promise((resolve, reject) => {
    const timeout = window.setTimeout(() => {
      window.removeEventListener("deviceorientation", onOrientation);
      reject(new Error("sensor timeout"));
    }, 3500);
    const onOrientation = (event) => {
      if (attempt !== motionAttempt) return;
      if (!Number.isFinite(event.beta) || !Number.isFinite(event.gamma)) return;
      window.clearTimeout(timeout);
      window.removeEventListener("deviceorientation", onOrientation);
      resolve();
    };
    window.addEventListener("deviceorientation", onOrientation);
  });

motionButton.addEventListener("click", async () => {
  motionAttempt += 1;
  const attempt = motionAttempt;
  motionButton.disabled = true;
  motionButton.textContent = "REQUESTING SENSOR…";

  try {
    const requestPermission = window.DeviceOrientationEvent.requestPermission;
    const permission =
      typeof requestPermission === "function"
        ? await requestPermission.call(window.DeviceOrientationEvent)
        : "granted";
    if (permission !== "granted") throw new Error("permission denied");

    motionState = "listening";
    sendMotionStatus("listening");
    motionButton.textContent = "MOVE PHONE TO CALIBRATE…";
    syncDock();
    await waitForMotionSample(attempt);

    motionState = "active";
    sendMotionStatus("active");
    if ((terminal.dataset.gamePhase ?? "ready") === "ready") startGame();
  } catch (error) {
    const denied = error.message === "permission denied";
    motionState = denied ? "denied" : "timeout";
    sendMotionStatus(denied ? "denied" : "unavailable");
  } finally {
    motionButton.disabled = false;
    syncDock();
    focusTerminal();
  }
});

basicButton.addEventListener("click", startGame);

recenterButton.addEventListener("click", () => {
  window.dispatchEvent(new Event("liptoi-recenter"));
  recenterButton.textContent = "TILT RECENTERED";
  window.setTimeout(() => {
    recenterButton.textContent = "RECENTER TILT";
  }, 900);
  focusTerminal();
});

new MutationObserver(syncDock).observe(terminal, {
  attributes: true,
  attributeFilter: ["data-game-phase", "data-control-source", "data-motion-status"],
});

window.addEventListener("TrunkApplicationStarted", () => {
  bootScreen.hidden = true;
  basicButton.disabled = false;
  basicButton.textContent = touchCapable ? "START · TOUCH / KEYS" : "START · MOUSE / KEYS";
  motionButton.disabled = false;
  window.dispatchEvent(
    new CustomEvent("liptoi-capabilities", {
      detail: `${touchCapable ? "touch" : "desktop"}${motionCapable ? "-motion" : ""}`,
    }),
  );
  syncDock();
});
