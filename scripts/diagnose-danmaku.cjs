const fs = require("node:fs");
const path = require("node:path");
const { spawnSync } = require("node:child_process");

const projectRoot = path.resolve(__dirname, "..");

if (!process.versions.electron) {
  const electronPath = require("electron");
  const env = { ...process.env };
  delete env.ELECTRON_RUN_AS_NODE;
  const child = spawnSync(electronPath, ["--no-sandbox", __filename, ...process.argv.slice(2)], {
    cwd: projectRoot,
    env,
    stdio: "inherit",
  });
  process.exit(child.status ?? 1);
}

const { app, BrowserWindow } = require("electron");

const options = parseArgs(process.argv.slice(2));
app.commandLine.appendSwitch("no-sandbox");
app.commandLine.appendSwitch("disable-background-timer-throttling");
app.commandLine.appendSwitch("disable-renderer-backgrounding");

app.whenReady()
  .then(async () => {
    const workerSource = readBuiltDanmakuWorker();
    const window = new BrowserWindow({
      width: options.width,
      height: options.height,
      show: false,
      webPreferences: {
        backgroundThrottling: false,
        contextIsolation: true,
        nodeIntegration: false,
        sandbox: false,
      },
    });

    await window.loadURL("data:text/html;charset=utf-8,<html><body></body></html>");
    const result = await window.webContents.executeJavaScript(
      `(${runBenchmark.toString()})(${JSON.stringify(workerSource)}, ${JSON.stringify(options)})`,
      true,
    );
    console.log(JSON.stringify(result, null, 2));
    window.destroy();
    app.quit();
  })
  .catch((error) => {
    console.error(JSON.stringify({ ok: false, error: error instanceof Error ? error.message : String(error) }, null, 2));
    app.quit();
    process.exitCode = 1;
  });

function parseArgs(args) {
  const numberOption = (name, fallback) => {
    const index = args.indexOf(name);
    const value = index >= 0 ? Number(args[index + 1]) : NaN;
    return Number.isFinite(value) && value > 0 ? Math.round(value) : fallback;
  };
  return {
    width: numberOption("--width", 1920),
    height: numberOption("--height", 1080),
    dpr: numberOption("--dpr", 1),
    count: numberOption("--count", 5000),
    sampleMs: numberOption("--sample-ms", 5000),
    targetFps: numberOption("--target-fps", 60),
    timelineSeconds: numberOption("--timeline-seconds", 5),
    area: Math.min(1, Math.max(0.25, numberOption("--area-percent", 100) / 100)),
  };
}

function readBuiltDanmakuWorker() {
  const rendererDir = path.join(projectRoot, "dist/renderer");
  const workerFile = fs.readdirSync(rendererDir)
    .filter((file) => /^danmaku\.worker-.*\.js$/.test(file))
    .sort()
    .pop();
  if (!workerFile) {
    throw new Error("built danmaku worker not found; run `npm run build` first");
  }
  return fs.readFileSync(path.join(rendererDir, workerFile), "utf8");
}

async function runBenchmark(workerSource, options) {
  const hasOffscreen = typeof OffscreenCanvas !== "undefined"
    && "transferControlToOffscreen" in HTMLCanvasElement.prototype;
  if (!hasOffscreen) {
    return {
      ok: false,
      error: "OffscreenCanvas transfer is not available",
    };
  }

  document.body.style.margin = "0";
  document.body.style.background = "#000";
  const canvas = document.createElement("canvas");
  canvas.style.width = `${options.width}px`;
  canvas.style.height = `${options.height}px`;
  document.body.appendChild(canvas);

  const workerUrl = URL.createObjectURL(new Blob([workerSource], { type: "text/javascript" }));
  const worker = new Worker(workerUrl);
  const offscreen = canvas.transferControlToOffscreen();
  const startedAt = performance.now();
  let mainTicks = 0;
  let mainLongTicks = 0;
  let lastTickAt = startedAt;
  let rafId = 0;
  let clockTimer = 0;

  const mainTick = (now) => {
    mainTicks += 1;
    if (now - lastTickAt > 34) {
      mainLongTicks += 1;
    }
    lastTickAt = now;
    rafId = requestAnimationFrame(mainTick);
  };
  rafId = requestAnimationFrame(mainTick);

  try {
    return await new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        reject(new Error("danmaku profile timed out"));
      }, options.sampleMs + 6000);
      let nextSnapshotId = 1;
      const snapshotResolvers = new Map();

      worker.onerror = (event) => {
        clearTimeout(timeout);
        reject(new Error(event.message || `danmaku worker failed at ${event.filename || "unknown"}:${event.lineno || 0}`));
      };
      worker.onmessage = async (event) => {
        if (event.data?.type === "snapshot") {
          const resolver = snapshotResolvers.get(event.data.id);
          if (resolver) {
            snapshotResolvers.delete(event.data.id);
            resolver(event.data);
          }
          return;
        }
        if (event.data?.type !== "profileResult") return;
        clearTimeout(timeout);
        const elapsedMs = performance.now() - startedAt;
        try {
          const pause = await runPauseRetentionCheck();
          resolve({
            ok: true,
            options,
            worker: event.data,
            pause,
            mainThread: {
              durationMs: Number(elapsedMs.toFixed(2)),
              ticks: mainTicks,
              achievedFps: Number((mainTicks / (elapsedMs / 1000)).toFixed(2)),
              longTicks: mainLongTicks,
            },
          });
        } catch (error) {
          reject(error);
        }
      };

      worker.postMessage({
        type: "init",
        canvas: offscreen,
        width: options.width,
        height: options.height,
        dpr: options.dpr,
        area: options.area,
      }, [offscreen]);
      worker.postMessage({
        type: "rawItems",
        items: createStressItems(options.count, options.timelineSeconds),
        position: 0,
      });
      worker.postMessage({ type: "visible", visible: true, position: 0 });
      worker.postMessage({
        type: "clock",
        position: 0,
        paused: false,
        seeking: false,
        timestamp: performance.now(),
      });
      worker.postMessage({
        type: "profile",
        sampleMs: options.sampleMs,
        targetFps: options.targetFps,
      });

      clockTimer = setInterval(() => {
        worker.postMessage({
          type: "clock",
          position: (performance.now() - startedAt) / 1000,
          paused: false,
          seeking: false,
          timestamp: performance.now(),
        });
      }, 250);

      function snapshot() {
        const id = nextSnapshotId++;
        worker.postMessage({ type: "snapshot", id });
        return new Promise((resolveSnapshot, rejectSnapshot) => {
          const snapshotTimeout = setTimeout(() => {
            snapshotResolvers.delete(id);
            rejectSnapshot(new Error("timeout waiting for danmaku snapshot"));
          }, 2000);
          snapshotResolvers.set(id, (payload) => {
            clearTimeout(snapshotTimeout);
            resolveSnapshot(payload);
          });
        });
      }

      async function runPauseRetentionCheck() {
        if (clockTimer) {
          clearInterval(clockTimer);
          clockTimer = 0;
        }
        const pauseStartedAt = performance.now();
        worker.postMessage({
          type: "rawItems",
          items: createPauseItems(),
          position: 0,
        });
        worker.postMessage({ type: "visible", visible: true, position: 0 });
        worker.postMessage({
          type: "clock",
          position: 0,
          paused: true,
          seeking: true,
          timestamp: performance.now(),
        });
        worker.postMessage({
          type: "clock",
          position: 0,
          paused: false,
          seeking: false,
          timestamp: performance.now(),
        });
        await sleep(1450);
        worker.postMessage({ type: "renderNow" });
        await sleep(30);
        const before = await snapshot();
        worker.postMessage({
          type: "clock",
          position: 0,
          paused: true,
          seeking: false,
          timestamp: performance.now(),
        });
        worker.postMessage({ type: "renderNow" });
        await sleep(420);
        worker.postMessage({ type: "renderNow" });
        await sleep(30);
        const after = await snapshot();
        return {
          ok: Boolean(before.canvasDirty && after.canvasDirty && before.activeCount > 0 && after.activeCount > 0),
          durationMs: Number((performance.now() - pauseStartedAt).toFixed(2)),
          before,
          after,
        };
      }
    });
  } finally {
    if (rafId) cancelAnimationFrame(rafId);
    if (clockTimer) clearInterval(clockTimer);
    worker.postMessage({ type: "dispose" });
    worker.terminate();
    URL.revokeObjectURL(workerUrl);
  }

  function createStressItems(count, seconds) {
    const items = [];
    const colors = [0xffffff, 0x66ccff, 0xffcc33, 0xff6699, 0xaaff66, 0xd7b5ff];
    for (let index = 0; index < count; index += 1) {
      const burstOffset = (index % 120) / 120 * 0.22;
      const burstBase = Math.floor(index / 120) / Math.max(1, Math.ceil(count / 120)) * seconds;
      const modeRoll = index % 20;
      items.push({
        id: `stress-${index}`,
        time: Math.min(seconds, burstBase + burstOffset),
        mode: modeRoll === 0 ? "top" : modeRoll === 1 ? "bottom" : "scroll",
        color: colors[index % colors.length],
        text: `弹幕压力测试 ${index} 这是一条较长的滚动弹幕`,
      });
    }
    return items;
  }

  function createPauseItems() {
    return Array.from({ length: 160 }, (_, index) => ({
      id: `pause-${index}`,
      time: 1.15 + (index % 40) * 0.006,
      mode: index % 18 === 0 ? "top" : "scroll",
      color: index % 2 === 0 ? 0xffffff : 0xffcc33,
      text: `暂停保留检查 ${index} 弹幕不应该消失`,
    }));
  }

  function sleep(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }
}
