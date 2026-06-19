const { app, BrowserWindow, Menu, ipcMain, nativeTheme, protocol } = require("electron");
const fs = require("node:fs");
const { fork, spawn } = require("node:child_process");
const path = require("node:path");
const { Readable } = require("node:stream");
const { fileURLToPath } = require("node:url");

const isDev = !app.isPackaged;
const useDevRenderer = isDev && process.env.NEXPLAY_RENDERER_MODE !== "production";
const projectRoot = path.join(__dirname, "..");

app.commandLine.appendSwitch("no-sandbox");

protocol.registerSchemesAsPrivileged([
  {
    scheme: "nexplay-asset",
    privileges: {
      standard: true,
      secure: true,
      supportFetchAPI: true,
      stream: true,
    },
  },
]);

function registerAssetProtocol() {
  protocol.handle("nexplay-asset", async (request) => {
    const url = new URL(request.url);
    if (url.hostname !== "local") {
      return new Response("unsupported asset host", { status: 400 });
    }

    const filePath = decodeURIComponent(url.pathname.slice(1));
    return streamLocalFile(filePath, request);
  });
}

function streamLocalFile(filePath, request) {
  let stat;
  try {
    stat = fs.statSync(filePath);
  } catch {
    return new Response("asset not found", { status: 404 });
  }

  if (!stat.isFile()) {
    return new Response("asset is not a file", { status: 404 });
  }

  const range = request.headers.get("range");
  const contentType = contentTypeForPath(filePath);
  const baseHeaders = {
    "Content-Type": contentType,
    "Accept-Ranges": "bytes",
    "Cache-Control": "no-store",
  };

  if (range) {
    const match = /^bytes=(\d*)-(\d*)$/.exec(range);
    if (!match) {
      return new Response("invalid range", {
        status: 416,
        headers: {
          ...baseHeaders,
          "Content-Range": `bytes */${stat.size}`,
        },
      });
    }

    const start = match[1] ? Number(match[1]) : 0;
    const end = match[2] ? Math.min(Number(match[2]), stat.size - 1) : stat.size - 1;
    if (!Number.isFinite(start) || !Number.isFinite(end) || start > end || start >= stat.size) {
      return new Response("range not satisfiable", {
        status: 416,
        headers: {
          ...baseHeaders,
          "Content-Range": `bytes */${stat.size}`,
        },
      });
    }

    const stream = fs.createReadStream(filePath, { start, end });
    return new Response(Readable.toWeb(stream), {
      status: 206,
      headers: {
        ...baseHeaders,
        "Content-Length": String(end - start + 1),
        "Content-Range": `bytes ${start}-${end}/${stat.size}`,
      },
    });
  }

  const stream = fs.createReadStream(filePath);
  return new Response(Readable.toWeb(stream), {
    status: 200,
    headers: {
      ...baseHeaders,
      "Content-Length": String(stat.size),
    },
  });
}

function contentTypeForPath(filePath) {
  switch (path.extname(filePath).toLowerCase()) {
    case ".jpg":
    case ".jpeg":
      return "image/jpeg";
    case ".png":
      return "image/png";
    case ".webp":
      return "image/webp";
    case ".gif":
      return "image/gif";
    case ".mp4":
    case ".m4v":
      return "video/mp4";
    case ".webm":
      return "video/webm";
    case ".mkv":
      return "video/x-matroska";
    case ".mov":
      return "video/quicktime";
    case ".avi":
      return "video/x-msvideo";
    default:
      return "application/octet-stream";
  }
}

function backendArgs(command) {
  const backendBin = process.env.NEXPLAY_BACKEND_BIN;
  const executable = backendBin || "cargo";
  const args = backendBin ? [command] : ["run", "--quiet", "--", command];
  return { executable, args };
}

function runBackend(command, payload) {
  const { executable, args } = backendArgs(command);

  return new Promise((resolve, reject) => {
    const child = spawn(executable, args, {
      cwd: projectRoot,
      env: process.env,
      stdio: ["pipe", "pipe", "pipe"],
    });

    if (payload === undefined) {
      child.stdin.end();
    } else {
      child.stdin.end(JSON.stringify(payload));
    }

    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
    });
    child.on("error", reject);
    child.on("close", (code) => {
      if (code !== 0) {
        reject(new Error(stderr || `backend exited with code ${code}`));
        return;
      }

      try {
        resolve(JSON.parse(stdout));
      } catch (error) {
        reject(new Error(`failed to parse backend JSON: ${error.message}\n${stderr}`));
      }
    });
  });
}

function runBackendWithEvents(command, sender, payload) {
  const { executable, args } = backendArgs(command);

  return new Promise((resolve, reject) => {
    const child = spawn(executable, args, {
      cwd: projectRoot,
      env: process.env,
      stdio: ["pipe", "pipe", "pipe"],
    });

    if (payload === undefined) {
      child.stdin.end();
    } else {
      child.stdin.end(JSON.stringify(payload));
    }

    let stdout = "";
    let stderr = "";
    let eventBuffer = "";

    child.stdout.on("data", (chunk) => {
      stdout += chunk;
    });
    child.stderr.on("data", (chunk) => {
      eventBuffer += chunk.toString();
      const lines = eventBuffer.split(/\r?\n/);
      eventBuffer = lines.pop() || "";
      for (const line of lines) {
        const trimmed = line.trim();
        if (!trimmed) continue;
        try {
          const event = JSON.parse(trimmed);
          sender.send("backend:event", event);
        } catch {
          stderr += `${line}\n`;
        }
      }
    });
    child.on("error", reject);
    child.on("close", (code) => {
      const trailing = eventBuffer.trim();
      if (trailing) {
        try {
          sender.send("backend:event", JSON.parse(trailing));
        } catch {
          stderr += `${eventBuffer}\n`;
        }
      }

      if (code !== 0) {
        reject(new Error(stderr || `backend exited with code ${code}`));
        return;
      }

      try {
        resolve(JSON.parse(stdout));
      } catch (error) {
        reject(new Error(`failed to parse backend JSON: ${error.message}\n${stderr}`));
      }
    });
  });
}

let playerDaemon = null;
let nextPlayerRequestId = 1;
const pendingPlayerRequests = new Map();
let renderDaemon = null;
let nextRenderRequestId = 1;
const pendingRenderRequests = new Map();
let mpvRenderBridgeInfo = null;

function ensureRenderDaemon() {
  if (renderDaemon && !renderDaemon.killed && renderDaemon.connected) {
    return renderDaemon;
  }

  const daemonPath = path.join(projectRoot, "native/mpv-render-bridge/renderer-daemon.cjs");
  const addonPath = path.join(projectRoot, "native/mpv-render-bridge/build/Release/mpv_render_bridge.node");
  if (!fs.existsSync(daemonPath) || !fs.existsSync(addonPath)) {
    throw new Error("native render bridge is not built");
  }

  const child = fork(daemonPath, [], {
    cwd: projectRoot,
    env: process.env,
    execPath: process.env.NEXPLAY_NODE_BIN || "node",
    serialization: "advanced",
    stdio: ["ignore", "pipe", "pipe", "ipc"],
  });

  renderDaemon = child;
  child.stdout.on("data", (chunk) => {
    const text = chunk.toString().trim();
    if (text) {
      console.log(`[mpv-render] ${text}`);
    }
  });
  child.stderr.on("data", (chunk) => {
    const text = chunk.toString().trim();
    if (text) {
      console.warn(`[mpv-render] ${text}`);
    }
  });
  child.on("message", (message) => {
    const pending = pendingRenderRequests.get(message?.id);
    if (!pending) return;
    pendingRenderRequests.delete(message.id);
    if (message.ok) {
      pending.resolve(message.payload);
    } else {
      pending.reject(new Error(message.error || "native render bridge request failed"));
    }
  });
  child.on("close", () => {
    if (renderDaemon === child) {
      renderDaemon = null;
      mpvRenderBridgeInfo = null;
    }
    for (const pending of pendingRenderRequests.values()) {
      pending.reject(new Error("native render bridge exited"));
    }
    pendingRenderRequests.clear();
  });
  child.on("error", (error) => {
    for (const pending of pendingRenderRequests.values()) {
      pending.reject(error);
    }
    pendingRenderRequests.clear();
  });

  return child;
}

function renderRequest(command) {
  const child = ensureRenderDaemon();
  const id = nextRenderRequestId++;
  return new Promise((resolve, reject) => {
    pendingRenderRequests.set(id, { resolve, reject });
    child.send({ id, command }, (error) => {
      if (error) {
        pendingRenderRequests.delete(id);
        reject(error);
      }
    });
  });
}

async function getMpvRenderBridgeInfo() {
  if (mpvRenderBridgeInfo) {
    return mpvRenderBridgeInfo;
  }

  try {
    const info = await renderRequest({ type: "info" });
    mpvRenderBridgeInfo = {
      ...info,
      available: true,
    };
    return mpvRenderBridgeInfo;
  } catch (error) {
    mpvRenderBridgeInfo = {
      available: false,
      reason: error instanceof Error ? error.message : String(error),
    };
    return mpvRenderBridgeInfo;
  }
}

function ensurePlayerDaemon() {
  if (playerDaemon && !playerDaemon.killed) {
    return playerDaemon;
  }

  const { executable, args } = backendArgs("player-daemon");
  const child = spawn(executable, args, {
    cwd: projectRoot,
    env: process.env,
    stdio: ["pipe", "pipe", "pipe"],
  });

  playerDaemon = child;
  let stdoutBuffer = "";

  child.stdout.on("data", (chunk) => {
    stdoutBuffer += chunk.toString();
    const lines = stdoutBuffer.split(/\r?\n/);
    stdoutBuffer = lines.pop() || "";
    for (const line of lines) {
      const trimmed = line.trim();
      if (!trimmed) continue;
      let response;
      try {
        response = JSON.parse(trimmed);
      } catch {
        continue;
      }
      const pending = pendingPlayerRequests.get(response.id);
      if (!pending) continue;
      pendingPlayerRequests.delete(response.id);
      if (response.ok) {
        pending.resolve(response.state || {});
      } else {
        pending.reject(new Error(response.error || "libmpv request failed"));
      }
    }
  });

  child.stderr.on("data", (chunk) => {
    const text = chunk.toString().trim();
    if (text) {
      console.warn(`[libmpv] ${text}`);
    }
  });

  child.on("close", () => {
    if (playerDaemon === child) {
      playerDaemon = null;
    }
    for (const pending of pendingPlayerRequests.values()) {
      pending.reject(new Error("libmpv daemon exited"));
    }
    pendingPlayerRequests.clear();
  });

  child.on("error", (error) => {
    for (const pending of pendingPlayerRequests.values()) {
      pending.reject(error);
    }
    pendingPlayerRequests.clear();
  });

  return child;
}

function playerRequest(command) {
  const child = ensurePlayerDaemon();
  const id = nextPlayerRequestId++;
  const payload = JSON.stringify({ id, command });
  return new Promise((resolve, reject) => {
    pendingPlayerRequests.set(id, { resolve, reject });
    child.stdin.write(`${payload}\n`, (error) => {
      if (error) {
        pendingPlayerRequests.delete(id);
        reject(error);
      }
    });
  });
}

async function loadMpvMedia(mediaId) {
  const source = await runBackend("media-source", { mediaId });
  const path = source.sourceUrl.startsWith("file://")
    ? fileURLToPath(source.sourceUrl)
    : source.sourceUrl;
  const bridgeInfo = await getMpvRenderBridgeInfo();
  const state = bridgeInfo.available && bridgeInfo.probe?.ok
    ? await renderRequest({ type: "load", path })
    : await playerRequest({ type: "load", path });
  if (state && state.ok === false) {
    throw new Error(state.error || "libmpv renderer failed to load media");
  }
  return {
    ...state,
    source,
  };
}

async function nativeRendererAvailable() {
  const bridgeInfo = await getMpvRenderBridgeInfo();
  return Boolean(bridgeInfo.available && bridgeInfo.probe?.ok);
}

async function controlMpv(command, fallback) {
  if (await nativeRendererAvailable()) {
    let state;
    switch (command.type) {
      case "setTrack":
        state = await renderRequest({ type: "setTrack", kind: command.kind, id: command.id ?? null });
        break;
      case "setPause":
        state = await renderRequest({ type: "setPause", paused: Boolean(command.paused) });
        break;
      case "seek":
        state = await renderRequest({ type: "seek", position: command.position });
        break;
      case "setVolume":
        state = await renderRequest({ type: "setVolume", volume: command.volume });
        break;
      case "stop":
        state = await renderRequest({ type: "stop" });
        break;
      case "state":
        state = await renderRequest({ type: "state" });
        break;
      default:
        throw new Error(`unsupported native mpv command: ${command.type}`);
    }
    if (state && state.ok === false) {
      throw new Error(state.error || "libmpv renderer command failed");
    }
    return state;
  }
  return fallback();
}

ipcMain.handle("backend:snapshot", () => runBackend("snapshot"));
ipcMain.handle("backend:scan", (event) => runBackendWithEvents("scan", event.sender));
ipcMain.handle("backend:settings", () => runBackend("settings"));
ipcMain.handle("backend:save-settings", (_event, payload) => runBackend("save-settings", payload));
ipcMain.handle("backend:open-media", (_event, mediaId) => runBackend("open-media", { mediaId }));
ipcMain.handle("backend:media-source", (_event, mediaId) => runBackend("media-source", { mediaId }));
ipcMain.handle("mpv:load", (_event, mediaId) => loadMpvMedia(mediaId));
ipcMain.handle("mpv:set-track", (_event, payload) => controlMpv(
  { type: "setTrack", kind: payload.kind, id: payload.id ?? null },
  () => playerRequest({
    type: "setTrack",
    kind: payload.kind,
    id: payload.id ?? null,
  })
));
ipcMain.handle("mpv:set-pause", (_event, paused) => controlMpv(
  { type: "setPause", paused },
  () => playerRequest({ type: "setPause", paused })
));
ipcMain.handle("mpv:seek", (_event, position) => controlMpv(
  { type: "seek", position },
  () => playerRequest({ type: "seek", position })
));
ipcMain.handle("mpv:set-volume", (_event, volume) => controlMpv(
  { type: "setVolume", volume },
  () => playerRequest({ type: "setVolume", volume })
));
ipcMain.handle("mpv:stop", () => controlMpv(
  { type: "stop" },
  () => playerRequest({ type: "stop" })
));
ipcMain.handle("mpv:state", () => controlMpv(
  { type: "state" },
  () => playerRequest({ type: "state" })
));
ipcMain.handle("mpv-render:info", () => getMpvRenderBridgeInfo());
ipcMain.handle("mpv-render:frame", async (_event, payload) => {
  const frame = await renderRequest({ type: "renderFrame", width: payload.width, height: payload.height });
  if (frame && frame.ok === false) {
    throw new Error(frame.error || "failed to render libmpv frame");
  }
  return frame;
});

function createMainWindow() {
  nativeTheme.themeSource = "system";
  Menu.setApplicationMenu(null);
  const backgroundColor = nativeTheme.shouldUseDarkColors ? "#1c1c1e" : "#f2f2f7";

  const window = new BrowserWindow({
    width: 1320,
    height: 860,
    minWidth: 1040,
    minHeight: 680,
    title: "NexPlay",
    backgroundColor,
    autoHideMenuBar: true,
    titleBarStyle: process.platform === "darwin" ? "hiddenInset" : "default",
    webPreferences: {
      preload: path.join(__dirname, "preload.cjs"),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: false,
    },
  });
  window.setMenu(null);

  if (useDevRenderer) {
    window.loadURL("http://127.0.0.1:5173");
  } else {
    window.loadFile(path.join(__dirname, "../dist/renderer/index.html"));
  }
}

app.whenReady().then(() => {
  registerAssetProtocol();
  createMainWindow();

  app.on("activate", () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createMainWindow();
    }
  });
});

app.on("window-all-closed", () => {
  if (playerDaemon) {
    playerDaemon.kill();
    playerDaemon = null;
  }
  if (renderDaemon) {
    renderDaemon.kill();
    renderDaemon = null;
  }
  if (process.platform !== "darwin") {
    app.quit();
  }
});
