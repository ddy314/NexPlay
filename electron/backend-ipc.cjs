const { BrowserWindow, ipcMain } = require("electron");

function registerBackendIpc(backendClient) {
  backendClient.on("backend:event", (event) => {
    for (const window of BrowserWindow.getAllWindows()) {
      window.webContents.send("backend:event", event);
    }
  });

  ipcMain.handle("backend:snapshot", () => backendClient.request("snapshot"));
  ipcMain.handle("backend:scan", () => backendClient.request("scanLibrary"));
  ipcMain.handle("backend:settings", () => backendClient.request("getSettings"));
  ipcMain.handle("backend:save-settings", (_event, payload) => (
    backendClient.request("saveSettings", payload)
  ));
  ipcMain.handle("backend:open-media", (_event, mediaId) => (
    backendClient.request("openMedia", { mediaId })
  ));
  ipcMain.handle("backend:media-source", (_event, mediaId) => (
    backendClient.request("mediaSource", { mediaId })
  ));
  ipcMain.handle("backend:danmaku-track", (_event, mediaId) => (
    backendClient.request("danmakuTrack", { mediaId })
  ));
}

module.exports = {
  registerBackendIpc,
};
