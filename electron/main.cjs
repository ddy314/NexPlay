const path = require("node:path");
const { app, BrowserWindow, Menu, nativeTheme, protocol } = require("electron");

const { registerAssetProtocol } = require("./asset-protocol.cjs");
const { BackendRpcClient } = require("./backend-rpc-client.cjs");
const { registerBackendIpc } = require("./backend-ipc.cjs");
const { PlayerControl } = require("./player-control.cjs");
const { RenderBridge } = require("./render-bridge.cjs");

const isDev = !app.isPackaged;
const useDevRenderer = isDev && process.env.NEXPLAY_RENDERER_MODE !== "production";
const projectRoot = app.isPackaged ? process.resourcesPath : path.join(__dirname, "..");

if (app.isPackaged && !process.env.NEXPLAY_CONFIG) {
  process.env.NEXPLAY_CONFIG = path.join(app.getPath("userData"), "config.toml");
}

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

const backendClient = new BackendRpcClient({ projectRoot });
const renderBridge = new RenderBridge({ projectRoot });
const playerControl = new PlayerControl({
  projectRoot,
  backendClient,
  renderBridge,
});

registerBackendIpc(backendClient);
playerControl.registerIpc();

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
  playerControl.shutdown();
  renderBridge.shutdown();
  backendClient.shutdown();
  if (process.platform !== "darwin") {
    app.quit();
  }
});
