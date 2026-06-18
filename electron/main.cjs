const { app, BrowserWindow, ipcMain, nativeTheme, net, protocol } = require("electron");
const { spawn } = require("node:child_process");
const path = require("node:path");
const { pathToFileURL } = require("node:url");

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
    },
  },
]);

function registerAssetProtocol() {
  protocol.handle("nexplay-asset", (request) => {
    const url = new URL(request.url);
    if (url.hostname !== "local") {
      return new Response("unsupported asset host", { status: 400 });
    }

    const filePath = decodeURIComponent(url.pathname.slice(1));
    return net.fetch(pathToFileURL(filePath).toString());
  });
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

ipcMain.handle("backend:snapshot", () => runBackend("snapshot"));
ipcMain.handle("backend:scan", (event) => runBackendWithEvents("scan", event.sender));
ipcMain.handle("backend:settings", () => runBackend("settings"));
ipcMain.handle("backend:save-settings", (_event, payload) => runBackend("save-settings", payload));
ipcMain.handle("backend:open-media", (_event, mediaId) => runBackend("open-media", { mediaId }));

function createMainWindow() {
  nativeTheme.themeSource = "system";
  const backgroundColor = nativeTheme.shouldUseDarkColors ? "#111827" : "#f6f3ec";

  const window = new BrowserWindow({
    width: 1320,
    height: 860,
    minWidth: 1040,
    minHeight: 680,
    title: "NexPlay",
    backgroundColor,
    titleBarStyle: process.platform === "darwin" ? "hiddenInset" : "default",
    webPreferences: {
      preload: path.join(__dirname, "preload.cjs"),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: false,
    },
  });

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
  if (process.platform !== "darwin") {
    app.quit();
  }
});
