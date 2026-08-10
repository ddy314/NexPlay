const { execFileSync } = require("node:child_process");
const { fork } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");
const { performance } = require("node:perf_hooks");

const projectRoot = path.resolve(__dirname, "..");
const defaultMedia =
  "/mnt/media/entertainment/bangumi/[SweetSub&LoliHouse] Takopii no Genzai/[SweetSub&LoliHouse] Takopii no Genzai - 01 [WebRip 1080p HEVC-10bit AAC ASSx2].mkv";
const rawArgs = process.argv.slice(2);
const discover = rawArgs.includes("--discover");
const includeSubtitles = rawArgs.includes("--subtitles");
const nativeSize = rawArgs.includes("--native-size");
const discoverDirCandidate = rawArgs[rawArgs.indexOf("--discover") + 1];
const discoverDirArg = discoverDirCandidate && !discoverDirCandidate.startsWith("--") ? discoverDirCandidate : null;
const limitArg = Number(rawArgs[rawArgs.indexOf("--limit") + 1]);
const discoverProbeLimit = Number.isFinite(limitArg) && limitArg > 0 ? Math.round(limitArg) : 400;
const playbackMsArg = Number(rawArgs[rawArgs.indexOf("--playback-ms") + 1]);
const playbackMs = Number.isFinite(playbackMsArg) && playbackMsArg > 0 ? Math.round(playbackMsArg) : 2000;
const seekSettleSampleMs = 650;
const exactSeekFirstFrameBudgetMs = 400;
const optionValueIndexes = new Set(
  ["--discover", "--limit", "--playback-ms"]
    .map((option) => rawArgs.indexOf(option) + 1)
    .filter((index) => index > 0),
);
const mediaArgs = rawArgs.filter((arg, index) => !arg.startsWith("--") && !optionValueIndexes.has(index));
const mediaPath = mediaArgs.join(" ") || process.env.NEXPLAY_DIAG_MEDIA || defaultMedia;
const daemonPath = path.join(projectRoot, "native/mpv-render-bridge/renderer-daemon.cjs");

if (!discover && !fs.existsSync(mediaPath)) {
  console.error(JSON.stringify({ ok: false, error: `media file not found: ${mediaPath}` }, null, 2));
  process.exit(1);
}

const daemon = fork(daemonPath, [], {
  cwd: projectRoot,
  serialization: "advanced",
  stdio: ["ignore", "ignore", "inherit", "ipc"],
});
daemon.on("exit", (code, signal) => {
  if (code && code !== 0) {
    console.error(`[diagnose-player] render daemon exited: code=${code} signal=${signal || "none"}`);
  }
});

let nextRequestId = 1;

function request(command, timeoutMs = 20000) {
  const id = nextRequestId++;
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      daemon.off("message", onMessage);
      reject(new Error(`timeout waiting for ${command.type}`));
    }, timeoutMs);
    const onMessage = (message) => {
      if (message.id !== id) return;
      clearTimeout(timeout);
      daemon.off("message", onMessage);
      message.ok ? resolve(message.payload) : reject(new Error(message.error));
    };
    daemon.on("message", onMessage);
    daemon.send({ id, command });
  });
}

function frameStats(frame) {
  let nonBlack = 0;
  let alphaNonOpaque = 0;
  for (let index = 0; index < frame.pixels.length; index += 4) {
    if (frame.pixels[index] || frame.pixels[index + 1] || frame.pixels[index + 2]) {
      nonBlack += 1;
    }
    if (frame.pixels[index + 3] !== 255) {
      alphaNonOpaque += 1;
    }
  }
  return { nonBlack, alphaNonOpaque };
}

function diffFrames(left, right) {
  const length = Math.min(left.pixels.length, right.pixels.length);
  let changedPixels = 0;
  let totalDelta = 0;
  for (let index = 0; index < length; index += 4) {
    const delta =
      Math.abs(left.pixels[index] - right.pixels[index]) +
      Math.abs(left.pixels[index + 1] - right.pixels[index + 1]) +
      Math.abs(left.pixels[index + 2] - right.pixels[index + 2]);
    if (delta > 18) {
      changedPixels += 1;
      totalDelta += delta;
    }
  }
  return { changedPixels, totalDelta };
}

function writePpm(filePath, frame) {
  const header = Buffer.from(`P6\n${frame.width} ${frame.height}\n255\n`);
  const rgb = Buffer.alloc(frame.width * frame.height * 3);
  for (let src = 0, dst = 0; src < frame.pixels.length; src += 4, dst += 3) {
    rgb[dst] = frame.pixels[src];
    rgb[dst + 1] = frame.pixels[src + 1];
    rgb[dst + 2] = frame.pixels[src + 2];
  }
  fs.writeFileSync(filePath, Buffer.concat([header, rgb]));
}

async function sleep(ms) {
  await new Promise((resolve) => setTimeout(resolve, ms));
}

async function waitForNewFrame(width, height, label = "frame", targetPosition = null, timeoutMs = 3000) {
  const deadline = performance.now() + timeoutMs;
  let lastFrame = null;
  while (performance.now() < deadline) {
    lastFrame = await request({ type: "renderFrame", width, height }, timeoutMs);
    const pts = lastFrame?.pts ?? lastFrame?.position;
    const targetMatches = targetPosition === null
      || (Number.isFinite(pts) && pts >= targetPosition - 1.25 && pts <= targetPosition + 2.5);
    if (lastFrame?.hasFrame !== false && lastFrame?.pixels && targetMatches) return lastFrame;
    await sleep(5);
  }
  throw new Error(`timeout waiting for ${label} (${lastFrame?.generation ?? "unknown"})`);
}

async function waitForPosition(position) {
  let state = null;
  for (let attempt = 0; attempt < 12; attempt += 1) {
    await sleep(60);
    state = await request({ type: "state" });
    if (typeof state.position !== "number" || Math.abs(state.position - position) < 1.25) {
      return state;
    }
  }
  return state || request({ type: "state" });
}

async function renderAt(position, subtitleId, frameSize) {
  await request({ type: "setPause", paused: true });
  await request({ type: "setTrack", kind: "subtitle", id: subtitleId });
  await request({ type: "seek", position });
  const state = await waitForPosition(position);
  const frame = await waitForNewFrame(frameSize.width, frameSize.height, "subtitle comparison frame", position);
  return { frame, state };
}

async function renderSameFrameWithSubtitle(position, subtitleId, frameSize) {
  const withSubtitle = await renderAt(position, subtitleId, frameSize);
  const withSubtitleAgain = await renderAt(position, subtitleId, frameSize);
  const withoutSubtitle = await renderAt(position, null, frameSize);
  return { withSubtitle, withSubtitleAgain, withoutSubtitle };
}

function frameSizeFromState(state, { nativeSize = false } = {}) {
  if (nativeSize) {
    const sourceWidth = Math.max(2, Math.round(state.videoWidth || 960));
    const sourceHeight = Math.max(2, Math.round(state.videoHeight || 540));
    const scale = Math.min(3840 / sourceWidth, 2160 / sourceHeight, 1);
    return {
      width: Math.max(2, Math.round(sourceWidth * scale)),
      height: Math.max(2, Math.round(sourceHeight * scale)),
    };
  }
  return {
    width: Math.max(2, Math.min(960, Math.round(state.videoWidth || 960))),
    height: Math.max(2, Math.min(540, Math.round(state.videoHeight || 540))),
  };
}

async function renderMeasurementFrame(load, frameSize) {
  const duration = load.duration || 0;
  const candidates = [0, 8, 20, 45, 90, 180, 300, 420]
    .filter((position) => position === 0 || position < duration || !duration);
  let fallback = null;
  for (const position of candidates) {
    if (position > 0) {
      await request({ type: "setPause", paused: true });
      await request({ type: "seek", position }, 30000);
      await sleep(120);
    }
    const start = performance.now();
    const frame = await waitForNewFrame(frameSize.width, frameSize.height, `measurement frame at ${position}s`, position);
    const ms = performance.now() - start;
    const stats = frameStats(frame);
    const measurement = { position, frame, ms, stats };
    fallback = fallback || measurement;
    if (stats.nonBlack > 0) {
      return measurement;
    }
  }
  return fallback;
}

async function measureContinuousPlayback(frameSize, fps, durationMs) {
  const frameBudgetMs = 1000 / Math.max(1, fps || 24);
  const renderSamples = [];
  const startedAt = performance.now();
  let nextDueAt = startedAt;
  let lateFrames = 0;
  let renderedFrames = 0;
  let requestedFrames = 0;
  const uniquePts = new Set();
  const ptsValues = [];

  await request({ type: "setPause", paused: false });
  while (performance.now() - startedAt < durationMs) {
    const now = performance.now();
    if (now < nextDueAt) {
      await sleep(Math.max(0, nextDueAt - now));
    }
    const frameStart = performance.now();
    const frame = await request({ type: "renderFrame", width: frameSize.width, height: frameSize.height }, 30000);
    const renderMs = performance.now() - frameStart;
    renderSamples.push(renderMs);
    requestedFrames += 1;
    if (frame.hasFrame !== false) {
      renderedFrames += 1;
      if (Number.isFinite(frame.pts ?? frame.position)) {
        const pts = Number((frame.pts ?? frame.position).toFixed(6));
        uniquePts.add(pts);
        ptsValues.push(pts);
      }
    }
    if (renderMs > frameBudgetMs) {
      lateFrames += 1;
    }
    nextDueAt += frameBudgetMs;
    if (performance.now() > nextDueAt + frameBudgetMs) {
      nextDueAt = performance.now();
      lateFrames += 1;
    }
  }
  await request({ type: "setPause", paused: true });

  const elapsedMs = performance.now() - startedAt;
  const expectedFrames = Math.max(1, Math.floor(elapsedMs / frameBudgetMs));
  const avgMs = renderSamples.reduce((sum, value) => sum + value, 0) / Math.max(1, renderSamples.length);
  const ptsIntervalsMs = ptsValues.slice(1).map((value, index) => Math.max(0, (value - ptsValues[index]) * 1000));
  const uniqueFrames = uniquePts.size;
  const droppedFramesEstimate = ptsIntervalsMs.reduce((total, intervalMs) => {
    if (intervalMs <= frameBudgetMs * 1.5) return total;
    return total + Math.max(0, Math.round(intervalMs / frameBudgetMs) - 1);
  }, 0);
  const uniqueFrameMissRate = droppedFramesEstimate / Math.max(1, uniqueFrames + droppedFramesEstimate);
  const requestP95Ms = percentile(renderSamples, 0.95);
  return {
    durationMs: Number(elapsedMs.toFixed(2)),
    targetFps: Number(Math.max(1, fps || 24).toFixed(3)),
    achievedFps: Number((renderedFrames / (elapsedMs / 1000)).toFixed(2)),
    expectedFrames,
    renderedFrames,
    requestedFrames,
    uniqueFrames,
    uniqueFrameMissRate: Number(uniqueFrameMissRate.toFixed(4)),
    droppedFramesEstimate,
    lateFrames,
    avgRenderMs: Number(avgMs.toFixed(2)),
    maxRenderMs: Number(Math.max(...renderSamples).toFixed(2)),
    requestP95Ms: Number(requestP95Ms.toFixed(2)),
    ptsIntervalP95Ms: Number(percentile(ptsIntervalsMs, 0.95).toFixed(2)),
    frameBudgetMs: Number(frameBudgetMs.toFixed(2)),
    transportRecommendation: requestP95Ms > frameBudgetMs || uniqueFrameMissRate > 0.01
      ? "sharedMemoryTripleBuffer"
      : "softwareFrameUpload",
  };
}

function percentile(values, ratio) {
  if (!values.length) return 0;
  const sorted = [...values].sort((left, right) => left - right);
  return sorted[Math.min(sorted.length - 1, Math.max(0, Math.ceil(sorted.length * ratio) - 1))];
}

function discoverMediaFiles(root) {
  const output = execFileSync("find", [
    root,
    "-type",
    "f",
    "(",
    "-iname",
    "*.mkv",
    "-o",
    "-iname",
    "*.mp4",
    "-o",
    "-iname",
    "*.webm",
    ")",
  ], { encoding: "utf8" });
  return output.split("\n").filter(Boolean);
}

function probeMedia(filePath) {
  try {
    const output = execFileSync("ffprobe", [
      "-v",
      "error",
      "-select_streams",
      "v:0",
      "-show_entries",
      "stream=width,height,avg_frame_rate,r_frame_rate",
      "-of",
      "json",
      filePath,
    ], { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] });
    const stream = JSON.parse(output).streams?.[0] || {};
    const rate = stream.avg_frame_rate || stream.r_frame_rate || "0/1";
    const [num, den] = rate.split("/").map(Number);
    const fps = den ? num / den : 0;
    return {
      filePath,
      width: stream.width || 0,
      height: stream.height || 0,
      fps,
      pixelsPerSecond: (stream.width || 0) * (stream.height || 0) * fps,
    };
  } catch {
    return null;
  }
}

function selectDiscoveredSamples(root, probeLimit) {
  const samples = discoverMediaFiles(root)
    .slice(0, probeLimit)
    .map(probeMedia)
    .filter(Boolean)
    .sort((left, right) => right.pixelsPerSecond - left.pixelsPerSecond);
  const selected = [];
  for (const sample of samples) {
    if (!selected.some((current) => current.width === sample.width && current.height === sample.height && Math.round(current.fps) === Math.round(sample.fps))) {
      selected.push(sample);
    }
    if (selected.length >= 5) break;
  }
  return selected;
}

async function diagnoseMedia(currentMediaPath, { includeSubtitles = true, nativeSize = false } = {}) {
  const result = {
    ok: false,
    mediaPath: currentMediaPath,
    render: {},
    seek: {},
    subtitles: {},
    artifacts: {},
  };

  const info = await request({ type: "info" });
  const textureProbe = await request({ type: "probeWebglTextureRenderer" });
  const loadStartedAt = performance.now();
  const load = await request({ type: "load", path: currentMediaPath, generation: 1 }, 30000);
  const loadMs = performance.now() - loadStartedAt;
  await sleep(500);
  const state = await request({ type: "state" });
  const frameSize = frameSizeFromState(state, { nativeSize });

  const measurement = await renderMeasurementFrame(load, frameSize);
  const frame = measurement.frame;
  const renderMs = measurement.ms;
  const renderSamples = [renderMs];
  for (let index = 0; index < 11; index += 1) {
    const sampleStart = performance.now();
    await request({ type: "renderFrame", width: frameSize.width, height: frameSize.height }, 30000);
    renderSamples.push(performance.now() - sampleStart);
  }
  const avgRenderMs = renderSamples.reduce((sum, value) => sum + value, 0) / renderSamples.length;
  const playback = await measureContinuousPlayback(frameSize, state.fps, playbackMs);
  result.render = {
    renderMode: textureProbe.ok
      ? "webglTexture"
      : `libmpv-${info.build?.renderApi || info.probe?.renderApi || "unknown"}-diagnostic`,
    textureProbe,
    fallback: textureProbe.ok ? null : textureProbe.fallback,
    fallbackReason: textureProbe.ok ? null : textureProbe.error,
    width: frame.width,
    height: frame.height,
    sizeMode: nativeSize ? "native" : "diagnostic",
    ms: Number(renderMs.toFixed(2)),
    measuredAt: measurement.position,
    framePosition: Number((frame.position ?? measurement.position).toFixed(3)),
    framePositionDelta: Number(Math.abs((frame.position ?? measurement.position) - measurement.position).toFixed(3)),
    avgMs: Number(avgRenderMs.toFixed(2)),
    maxMs: Number(Math.max(...renderSamples).toFixed(2)),
    sourceFrameBudgetMs: Number((1000 / Math.max(1, state.fps || 24)).toFixed(2)),
    generation: load.generation,
    fileReady: load.fileReady,
    loadMs: Number(loadMs.toFixed(2)),
    ...frameStats(frame),
  };
  result.playback = playback;

  const seekStart = performance.now();
  const seekTarget = Math.min(300, Math.max(20, (load.duration || 600) / 3));
  await request({ type: "seek", position: seekTarget }, 30000);
  const seekDone = performance.now();
  const seekFrame = await waitForNewFrame(frameSize.width, frameSize.height, "seek frame", seekTarget);
  const seekFramePosition = typeof seekFrame.position === "number" ? seekFrame.position : seekTarget;
  result.seek = {
    commandMs: Number((seekDone - seekStart).toFixed(2)),
    firstFrameMs: Number((performance.now() - seekDone).toFixed(2)),
    targetPosition: Number(seekTarget.toFixed(3)),
    framePosition: Number(seekFramePosition.toFixed(3)),
    framePositionDelta: Number(Math.abs(seekFramePosition - seekTarget).toFixed(3)),
    firstFrameBudgetMs: exactSeekFirstFrameBudgetMs,
  };

  const seekPositions = [30, 90, 180, 300, 420].filter((position) => position < (load.duration || Infinity));
  const seekSamples = [];
  for (const position of seekPositions) {
    const start = performance.now();
    await request({ type: "seek", position }, 30000);
    const seekCommandDone = performance.now();
    const seekFrame = await waitForNewFrame(frameSize.width, frameSize.height, `continuous seek frame at ${position}s`, position);
    const frameDone = performance.now();
    const framePosition = typeof seekFrame.position === "number" ? seekFrame.position : position;
    await sleep(seekSettleSampleMs);
    const settledState = await request({ type: "state" });
    const settledFramePosition = typeof settledState.position === "number" ? settledState.position : framePosition;
    seekSamples.push({
      position,
      framePosition: Number(framePosition.toFixed(3)),
      framePositionDelta: Number(Math.abs(framePosition - position).toFixed(3)),
      settledFramePosition: Number(settledFramePosition.toFixed(3)),
      settledFramePositionDelta: Number(Math.abs(settledFramePosition - position).toFixed(3)),
      commandMs: Number((seekCommandDone - start).toFixed(2)),
      firstFrameMs: Number((frameDone - seekCommandDone).toFixed(2)),
    });
  }
  result.continuousSeek = {
    samples: seekSamples,
    maxFirstFrameMs: Number(Math.max(...seekSamples.map((sample) => sample.firstFrameMs)).toFixed(2)),
    firstFrameBudgetMs: exactSeekFirstFrameBudgetMs,
  };

  const selectedSubtitle = includeSubtitles
    ? state.subtitleTracks?.find((track) => track.selected) || state.subtitleTracks?.[0]
    : null;
  if (selectedSubtitle) {
    const candidates = [12, 30, 60, 90, 120, 180, 300, 420].filter((value) => value < (load.duration || Infinity));
    let best = null;
    for (const position of candidates) {
      const { withSubtitle, withSubtitleAgain, withoutSubtitle } = await renderSameFrameWithSubtitle(
        position,
        selectedSubtitle.id,
        frameSize,
      );
      const baseline = diffFrames(withSubtitle.frame, withSubtitleAgain.frame);
      const diff = diffFrames(withSubtitle.frame, withoutSubtitle.frame);
      const score = Math.max(0, diff.changedPixels - baseline.changedPixels);
      const totalPixels = withSubtitle.frame.width * withSubtitle.frame.height;
      const suspiciousWholeFrameDiff = diff.changedPixels > totalPixels * 0.7;
      const usableScore = suspiciousWholeFrameDiff ? 0 : score;
      if (!best || usableScore > best.usableScore || (best.usableScore === 0 && score > best.score)) {
        best = { position, withSubtitle, withoutSubtitle, baseline, diff, score, usableScore, suspiciousWholeFrameDiff };
      }
    }

    if (best) {
      const prefix = `/tmp/nexplay-subtitle-check-${process.pid}`;
      const withPath = `${prefix}-with-sub.ppm`;
      const withoutPath = `${prefix}-without-sub.ppm`;
      writePpm(withPath, best.withSubtitle.frame);
      writePpm(withoutPath, best.withoutSubtitle.frame);
      result.subtitles = {
        trackId: selectedSubtitle.id,
        title: selectedSubtitle.title,
        lang: selectedSubtitle.lang,
        codec: selectedSubtitle.codec,
        bestPosition: best.position,
        baselineChangedPixels: best.baseline.changedPixels,
        changedPixels: best.diff.changedPixels,
        netChangedPixels: best.score,
        totalDelta: best.diff.totalDelta,
        suspiciousWholeFrameDiff: best.suspiciousWholeFrameDiff,
        detected: best.score > Math.max(800, best.baseline.changedPixels * 3) && !best.suspiciousWholeFrameDiff,
      };
      result.artifacts = {
        withSubtitle: withPath,
        withoutSubtitle: withoutPath,
      };
    }
  }

  result.ok =
    info.probe?.ok === true &&
    result.render.nonBlack > 0 &&
    result.render.width === frameSize.width &&
    result.render.height === frameSize.height &&
    result.render.avgMs < Math.max(16.7, 1000 / Math.max(1, state.fps || 24)) &&
    result.playback.achievedFps >= Math.max(1, (state.fps || 24) * 0.92) &&
    result.playback.lateFrames <= Math.max(2, Math.ceil(result.playback.renderedFrames * 0.08)) &&
    result.seek.firstFrameMs < exactSeekFirstFrameBudgetMs &&
    result.continuousSeek.maxFirstFrameMs < exactSeekFirstFrameBudgetMs &&
    (!includeSubtitles || (state.subtitleTracks?.length ? result.subtitles.detected === true : true));
  result.state = {
    duration: load.duration,
    fps: state.fps,
    videoWidth: state.videoWidth,
    videoHeight: state.videoHeight,
    audioTracks: state.audioTracks?.length || 0,
    subtitleTracks: state.subtitleTracks?.length || 0,
  };

  return result;
}

async function main() {
  if (discover) {
    const root = discoverDirArg || "/mnt/media/entertainment/bangumi";
    const samples = selectDiscoveredSamples(root, discoverProbeLimit);
    const results = [];
    for (const sample of samples) {
      results.push(await diagnoseMedia(sample.filePath, { includeSubtitles: false, nativeSize }));
    }
    const output = {
      ok: results.every((result) => result.ok),
      root,
      probeLimit: discoverProbeLimit,
      playbackMs,
      samples: results,
    };
    console.log(JSON.stringify(output, null, 2));
    daemon.kill();
    if (!output.ok) process.exit(1);
    return;
  }

  const result = await diagnoseMedia(mediaPath, { includeSubtitles, nativeSize });
  console.log(JSON.stringify(result, null, 2));
  daemon.kill();
  if (!result.ok) {
    process.exit(1);
  }
}

main().catch((error) => {
  daemon.kill();
  console.error(JSON.stringify({ ok: false, error: error.message }, null, 2));
  process.exit(1);
});
