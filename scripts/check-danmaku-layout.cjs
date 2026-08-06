const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

const projectRoot = path.resolve(__dirname, "..");
const workerSource = readLatestWorker();
const postedMessages = [];

class FakeCanvasContext {
  constructor(canvas) {
    this.canvas = canvas;
  }

  measureText(text) {
    return { width: Array.from(text).length * 14 };
  }

  save() {}
  restore() {}
  setTransform() {}
  clearRect() {}
  drawImage() {}
  strokeText() {}
  fillText() {}
}

class FakeOffscreenCanvas {
  constructor(width, height) {
    this.width = width;
    this.height = height;
    this.context = new FakeCanvasContext(this);
  }

  getContext() {
    return this.context;
  }
}

const workerSelf = {
  postMessage(message) {
    postedMessages.push(message);
  },
  requestAnimationFrame() {
    return 1;
  },
  cancelAnimationFrame() {},
  setTimeout() {
    return 1;
  },
  clearTimeout() {},
};

vm.runInNewContext(workerSource, {
  self: workerSelf,
  OffscreenCanvas: FakeOffscreenCanvas,
  performance,
  console,
  Map,
  Set,
  Math,
  Number,
  Array,
  Object,
  String,
});

let snapshotId = 0;
const send = (data) => workerSelf.onmessage({ data });
const snapshot = () => {
  const id = ++snapshotId;
  send({ type: "snapshot", id });
  const result = postedMessages.findLast((message) => message.type === "snapshot" && message.id === id);
  if (!result) throw new Error(`missing snapshot ${id}`);
  return result;
};

send({
  type: "init",
  canvas: new FakeOffscreenCanvas(1920, 1080),
  width: 1920,
  height: 1080,
  dpr: 1,
  area: 1,
});
send({ type: "rawItems", items: createSeekItems(), position: 16 });
send({ type: "visible", visible: true, position: 16 });
send({
  type: "clock",
  position: 16,
  paused: true,
  seeking: false,
  timestamp: performance.now(),
});
send({ type: "renderNow" });
const seek = snapshot();

send({ type: "resize", width: 2560, height: 1440, dpr: 1, area: 1 });
send({ type: "renderNow" });
const fullscreen = snapshot();
send({ type: "resize", width: 1920, height: 1080, dpr: 1, area: 1 });
send({ type: "renderNow" });
const restored = snapshot();

// Repeated ready/clock publications are common during startup. They must not
// reconstruct a frame that has already been laid out.
send({ type: "visible", visible: true, position: 16 });
send({ type: "clock", position: 16, paused: true, seeking: false, timestamp: performance.now() });
send({ type: "clock", position: 16, paused: true, seeking: false, timestamp: performance.now() });
send({ type: "renderNow" });
const repeatedStartupSignals = snapshot();

const result = {
  ok: seek.activeCount >= 4
    && seek.activeLaneCount >= 4
    && seek.maxActiveLane >= 3
    && fullscreen.laneSignature === seek.laneSignature
    && restored.laneSignature === seek.laneSignature
    && fullscreen.scrollXSignature === seek.scrollXSignature
    && restored.scrollXSignature === seek.scrollXSignature
    && repeatedStartupSignals.laneSignature === seek.laneSignature,
  seek,
  fullscreen,
  restored,
  repeatedStartupSignals,
};

console.log(JSON.stringify(result.ok ? {
  ok: true,
  seekActiveCount: seek.activeCount,
  seekActiveLaneCount: seek.activeLaneCount,
  resizeLaneSignatureStable: fullscreen.laneSignature === seek.laneSignature
    && restored.laneSignature === seek.laneSignature,
  resizeHorizontalPositionStable: fullscreen.scrollXSignature === seek.scrollXSignature
    && restored.scrollXSignature === seek.scrollXSignature,
  repeatedStartupSignalsStable: repeatedStartupSignals.laneSignature === seek.laneSignature,
} : result, null, 2));
if (!result.ok) process.exitCode = 1;

function createSeekItems() {
  return Array.from({ length: 34 }, (_, index) => ({
    id: `seek-rebuild-${index}`,
    time: 10 + index * 0.18,
    mode: "scroll",
    color: index % 2 === 0 ? 0xffffff : 0x66ccff,
    text: `跳转轨道重建 ${index} 应保持自然分布`,
  }));
}

function readLatestWorker() {
  const rendererDir = path.join(projectRoot, "dist/renderer");
  const candidates = fs.readdirSync(rendererDir)
    .filter((file) => /^danmaku\.worker-.*\.js$/.test(file))
    .map((file) => ({
      file,
      modifiedAt: fs.statSync(path.join(rendererDir, file)).mtimeMs,
    }))
    .sort((left, right) => right.modifiedAt - left.modifiedAt);
  if (!candidates[0]) {
    throw new Error("built danmaku worker not found; run `npm run build` first");
  }
  return fs.readFileSync(path.join(rendererDir, candidates[0].file), "utf8");
}
