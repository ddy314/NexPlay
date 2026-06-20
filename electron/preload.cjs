const { contextBridge, ipcRenderer } = require("electron");

function resolveAssetUrl(value) {
  if (typeof value !== "string" || !value.startsWith("file://")) {
    return value;
  }

  const filePath = decodeURIComponent(value.slice("file://".length));
  return `nexplay-asset://local/${encodeURIComponent(filePath)}`;
}

contextBridge.exposeInMainWorld("nexplay", {
  appName: "NexPlay",
  getSnapshot: () => ipcRenderer.invoke("backend:snapshot"),
  scanLibrary: () => ipcRenderer.invoke("backend:scan"),
  getSettings: () => ipcRenderer.invoke("backend:settings"),
  saveSettings: (settings) => ipcRenderer.invoke("backend:save-settings", settings),
  openMedia: (mediaId) => ipcRenderer.invoke("backend:open-media", mediaId),
  getMediaSource: async (mediaId) => {
    const source = await ipcRenderer.invoke("backend:media-source", mediaId);
    return {
      ...source,
      sourceUrl: resolveAssetUrl(source.sourceUrl),
    };
  },
  mpvLoad: (mediaId) => ipcRenderer.invoke("mpv:load", mediaId),
  mpvSetTrack: (kind, id) => ipcRenderer.invoke("mpv:set-track", { kind, id }),
  mpvSetPause: (paused) => ipcRenderer.invoke("mpv:set-pause", paused),
  mpvSeek: (position) => ipcRenderer.invoke("mpv:seek", position),
  mpvSetVolume: (volume) => ipcRenderer.invoke("mpv:set-volume", volume),
  mpvStop: () => ipcRenderer.invoke("mpv:stop"),
  mpvState: () => ipcRenderer.invoke("mpv:state"),
  mpvRenderInfo: () => ipcRenderer.invoke("mpv-render:info"),
  mpvProbeWebglTextureRenderer: () => ipcRenderer.invoke("mpv-render:probe-webgl-texture"),
  mpvRenderFrame: (width, height) => ipcRenderer.invoke("mpv-render:frame", { width, height }),
  onBackendEvent: (callback) => {
    const listener = (_event, payload) => callback(payload);
    ipcRenderer.on("backend:event", listener);
    return () => ipcRenderer.removeListener("backend:event", listener);
  },
  resolveAssetUrl,
});
