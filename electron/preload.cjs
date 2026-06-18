const { contextBridge, ipcRenderer } = require("electron");

contextBridge.exposeInMainWorld("nexplay", {
  appName: "NexPlay",
  getSnapshot: () => ipcRenderer.invoke("backend:snapshot"),
  scanLibrary: () => ipcRenderer.invoke("backend:scan"),
  getSettings: () => ipcRenderer.invoke("backend:settings"),
  saveSettings: (settings) => ipcRenderer.invoke("backend:save-settings", settings),
  openMedia: (mediaId) => ipcRenderer.invoke("backend:open-media", mediaId),
  onBackendEvent: (callback) => {
    const listener = (_event, payload) => callback(payload);
    ipcRenderer.on("backend:event", listener);
    return () => ipcRenderer.removeListener("backend:event", listener);
  },
  resolveAssetUrl: (value) => {
    if (typeof value !== "string" || !value.startsWith("file://")) {
      return value;
    }

    const filePath = decodeURIComponent(value.slice("file://".length));
    return `nexplay-asset://local/${encodeURIComponent(filePath)}`;
  },
});
