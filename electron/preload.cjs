const path = require("node:path");
const { fileURLToPath } = require("node:url");
const { contextBridge, ipcRenderer } = require("electron");

const { RenderBridge } = require("./render-bridge.cjs");
const { findMatchingSubtitles } = require("./subtitle-matcher.cjs");

const projectRoot = process.env.NEXPLAY_PROJECT_ROOT || path.join(__dirname, "..");
const renderBridge = new RenderBridge({ projectRoot });
let activeMpvMode = null;
let activeTextureProbe = null;
let renderInfoPromise = null;
let textureProbePromise = null;
let requestedPlaybackGeneration = 0;
let activePlaybackGeneration = 0;
const preparedPlaybackRequests = new Map();
const EMBEDDED_MPV_UNAVAILABLE_MESSAGE = "内嵌播放器不可用，已禁用外部 mpv fallback";

function resolveAssetUrl(value) {
  if (typeof value !== "string" || !value.startsWith("file://")) {
    return value;
  }

  const filePath = decodeURIComponent(value.slice("file://".length));
  return `nexplay-asset://local/${encodeURIComponent(filePath)}`;
}

function mediaPathFromSourceUrl(sourceUrl) {
  return typeof sourceUrl === "string" && sourceUrl.startsWith("file://")
    ? fileURLToPath(sourceUrl)
    : sourceUrl;
}

async function normalizeMediaSource(source) {
  const mediaPath = mediaPathFromSourceUrl(source.sourceUrl);
  const subtitleCandidates = await findMatchingSubtitles(mediaPath);
  return {
    ...source,
    sourceUrl: resolveAssetUrl(source.sourceUrl),
    autoSubtitlePath: subtitleCandidates[0]?.path || null,
    subtitleCandidates,
  };
}

function preparePlaybackSource(mediaId, generation) {
  const normalizedGeneration = Math.max(1, Math.trunc(Number(generation) || 1));
  requestedPlaybackGeneration = Math.max(requestedPlaybackGeneration, normalizedGeneration);
  const key = `${mediaId}:${normalizedGeneration}`;
  for (const existingKey of preparedPlaybackRequests.keys()) {
    if (existingKey.startsWith(`${mediaId}:`) && existingKey !== key) preparedPlaybackRequests.delete(existingKey);
  }
  if (!preparedPlaybackRequests.has(key)) {
    const request = ipcRenderer.invoke("backend:media-source", mediaId)
      .then(async (rawSource) => {
        const mediaPath = mediaPathFromSourceUrl(rawSource.sourceUrl);
        const source = await normalizeMediaSource(rawSource);
        return {
          generation: normalizedGeneration,
          mediaId,
          mediaPath,
          source,
          resumePosition: Number(source.playbackPosition) || 0,
          subtitleCandidates: source.subtitleCandidates || [],
        };
      })
      .catch((error) => {
        preparedPlaybackRequests.delete(key);
        throw error;
      });
    preparedPlaybackRequests.set(key, request);
  }
  return preparedPlaybackRequests.get(key);
}

function embeddedMpvUnavailableError(detail) {
  return new Error(detail
    ? `${EMBEDDED_MPV_UNAVAILABLE_MESSAGE}：${detail}`
    : EMBEDDED_MPV_UNAVAILABLE_MESSAGE);
}

function renderUnavailableDetail(bridgeInfo, textureProbe) {
  return textureProbe?.error
    || bridgeInfo?.textureProbe?.error
    || bridgeInfo?.probe?.error
    || bridgeInfo?.reason
    || null;
}

function unloadedMpvState() {
  return {
    ok: true,
    loaded: false,
    audioTracks: [],
    subtitleTracks: [],
    paused: true,
    volume: 100,
  };
}

function getRenderInfoCached() {
  if (!renderInfoPromise) {
    renderInfoPromise = renderBridge.getInfo().catch((error) => {
      renderInfoPromise = null;
      throw error;
    });
  }
  return renderInfoPromise;
}

function probeWebglTextureRendererCached() {
  if (!textureProbePromise) {
    textureProbePromise = renderBridge.probeWebglTextureRenderer().catch((error) => {
      textureProbePromise = null;
      throw error;
    });
  }
  return textureProbePromise;
}

async function loadMpvMedia(prepared) {
  if (!prepared || !prepared.source || !prepared.mediaPath) {
    throw new Error("mpvLoad requires a PreparedPlaybackSource");
  }
  const { generation, mediaPath, source, subtitleCandidates = [] } = prepared;
  requestedPlaybackGeneration = Math.max(requestedPlaybackGeneration, generation);
  const [bridgeInfo, textureProbe] = await Promise.all([
    getRenderInfoCached(),
    probeWebglTextureRendererCached(),
  ]);
  const canUseTextureRenderer = Boolean(bridgeInfo.available && textureProbe.ok);

  if (!canUseTextureRenderer) {
    activeMpvMode = null;
    activeTextureProbe = textureProbe;
    throw embeddedMpvUnavailableError(renderUnavailableDetail(bridgeInfo, textureProbe));
  }

  let state = await renderBridge.request({
    type: "load",
    path: mediaPath,
    generation,
  });
  if (state && state.ok === false) {
    throw new Error(state.error || "libmpv failed to load media");
  }

  if (generation !== requestedPlaybackGeneration) {
    return { ...state, stale: true, generation };
  }
  activeMpvMode = "webglTexture";
  activePlaybackGeneration = generation;
  activeTextureProbe = textureProbe;
  const autoSubtitlePath = subtitleCandidates[0]?.path || null;
  if (autoSubtitlePath) {
    try {
      state = await controlMpv({ type: "addSubtitle", path: autoSubtitlePath, select: false });
    } catch (error) {
      console.warn(`[subtitle] failed to load ${autoSubtitlePath}: ${error instanceof Error ? error.message : String(error)}`);
    }
  }
  return {
    ...state,
    source: {
      ...source,
      autoSubtitlePath,
    },
    renderMode: "webglTexture",
    textureProbe,
  };
}

async function addSubtitleToMpv(path) {
  const state = await controlMpv({ type: "addSubtitle", path });
  return { state, path };
}

async function controlMpv(command) {
  if (activeMpvMode !== "webglTexture") {
    throw embeddedMpvUnavailableError(activeTextureProbe?.error);
  }

  const state = await renderBridge.request(command);
  if (state && state.ok === false) {
    throw new Error(state.error || "libmpv renderer command failed");
  }
  return state;
}

window.addEventListener("beforeunload", () => {
  renderBridge.shutdown();
});

contextBridge.exposeInMainWorld("nexplay", {
  appName: "NexPlay",
  getSnapshot: () => ipcRenderer.invoke("backend:snapshot"),
  scanLibrary: () => ipcRenderer.invoke("backend:scan"),
  getSettings: () => ipcRenderer.invoke("backend:settings"),
  saveSettings: (settings) => ipcRenderer.invoke("backend:save-settings", settings),
  searchCatalog: (payload) => ipcRenderer.invoke("backend:search-catalog", payload),
  onlineSubject: (payload) => ipcRenderer.invoke("backend:online-subject", payload),
  resolveSubject: (payload) => ipcRenderer.invoke("backend:resolve-subject", payload),
  refreshSubjectMetadata: (payload) => ipcRenderer.invoke("backend:refresh-subject-metadata", payload),
  episodeResources: (payload) => ipcRenderer.invoke("backend:episode-resources", payload),
  startResourceDownload: (payload) => ipcRenderer.invoke("backend:start-resource-download", payload),
  prepareResourceDownload: (payload) => ipcRenderer.invoke("backend:prepare-resource-download", payload),
  confirmResourceDownload: (payload) => ipcRenderer.invoke("backend:confirm-resource-download", payload),
  downloadTasks: () => ipcRenderer.invoke("backend:download-tasks"),
  controlDownloadTask: (payload) => ipcRenderer.invoke("backend:control-download-task", payload),
  bangumiAuthStatus: () => ipcRenderer.invoke("backend:bangumi-auth-status"),
  startBangumiLogin: () => ipcRenderer.invoke("backend:start-bangumi-login"),
  completeBangumiOAuth: (payload) => ipcRenderer.invoke("backend:complete-bangumi-oauth", payload),
  logoutBangumi: () => ipcRenderer.invoke("backend:logout-bangumi"),
  syncBangumiNow: () => ipcRenderer.invoke("backend:sync-bangumi-now"),
  syncBangumiSubject: (payload) => ipcRenderer.invoke("backend:sync-bangumi-subject", payload),
  updateBangumiCollection: (payload) => ipcRenderer.invoke("backend:update-bangumi-collection", payload),
  updateBangumiEpisode: (payload) => ipcRenderer.invoke("backend:update-bangumi-episode", payload),
  batchUpdateBangumiEpisodes: (payload) => ipcRenderer.invoke("backend:batch-update-bangumi-episodes", payload),
  reportPlaybackProgress: (payload) => ipcRenderer.invoke("backend:report-playback-progress", payload),
  startPlaybackSession: (payload) => ipcRenderer.invoke("backend:start-playback-session", payload),
  heartbeatPlaybackSession: (payload) => ipcRenderer.invoke("backend:heartbeat-playback-session", payload),
  recordPlaybackSessionEvent: (payload) => ipcRenderer.invoke("backend:record-playback-session-event", payload),
  finishPlaybackSession: (payload) => ipcRenderer.invoke("backend:finish-playback-session", payload),
  insightsDashboard: (payload) => ipcRenderer.invoke("backend:insights-dashboard", payload),
  clearPlaybackAnalytics: () => ipcRenderer.invoke("backend:clear-playback-analytics"),
  homeFeed: () => ipcRenderer.invoke("backend:home-feed"),
  testQbittorrentConnection: () => ipcRenderer.invoke("backend:test-qbittorrent"),
  openMedia: (mediaId) => ipcRenderer.invoke("backend:open-media", mediaId),
  getMediaSource: async (mediaId) => normalizeMediaSource(await ipcRenderer.invoke("backend:media-source", mediaId)),
  preparePlaybackSource,
  danmakuTrack: (mediaId, videoDuration = 0) => ipcRenderer.invoke("backend:danmaku-track", { mediaId, videoDuration }),
  rematchDanmaku: (mediaId, videoDuration = 0) => ipcRenderer.invoke("backend:rematch-danmaku", { mediaId, videoDuration }),
  danmakuBinding: (mediaId, videoDuration = 0) => ipcRenderer.invoke("backend:danmaku-binding", { mediaId, videoDuration }),
  setDanmakuOffset: (mediaId, offsetMs) => ipcRenderer.invoke("backend:set-danmaku-offset", { mediaId, offsetMs }),
  mpvLoad: (prepared) => loadMpvMedia(prepared),
  mpvSetTrack: (kind, id) => controlMpv({ type: "setTrack", kind, id }),
  mpvAddSubtitle: async () => {
    const path = await ipcRenderer.invoke("dialog:select-subtitle");
    if (!path) {
      return null;
    }
    return addSubtitleToMpv(path);
  },
  mpvAddSubtitlePath: (path) => addSubtitleToMpv(path),
  mpvSetPause: (paused) => controlMpv({ type: "setPause", paused }),
  mpvSeek: (position) => controlMpv({ type: "seek", position }),
  mpvSetVolume: (volume) => controlMpv({ type: "setVolume", volume }),
  mpvStop: async (generation) => {
    await new Promise((resolve) => setTimeout(resolve, 0));
    if (Number(generation) !== activePlaybackGeneration || requestedPlaybackGeneration > Number(generation)) {
      return unloadedMpvState();
    }
    if (activeMpvMode !== "webglTexture") {
      activeMpvMode = null;
      activeTextureProbe = null;
      activePlaybackGeneration = 0;
      return unloadedMpvState();
    }
    try {
      return await controlMpv({ type: "stop" });
    } finally {
      activeMpvMode = null;
      activeTextureProbe = null;
    }
  },
  mpvState: () => activeMpvMode === "webglTexture"
    ? controlMpv({ type: "state" })
    : Promise.resolve(unloadedMpvState()),
  mpvRenderInfo: () => getRenderInfoCached(),
  mpvProbeWebglTextureRenderer: async () => {
    textureProbePromise = renderBridge.probeWebglTextureRenderer().catch((error) => {
      textureProbePromise = null;
      throw error;
    });
    activeTextureProbe = await textureProbePromise;
    return activeTextureProbe;
  },
  mpvRenderFrame: async (width, height, generation) => {
    const requestedAt = performance.now();
    if (activeMpvMode !== "webglTexture") {
      return ipcRenderer.invoke("mpv-render:frame", { width, height });
    }
    const frame = await renderBridge.request({ type: "renderFrame", width, height, generation });
    return { ...frame, requestMs: performance.now() - requestedAt };
  },
  onBackendEvent: (callback) => {
    const listener = (_event, payload) => callback(payload);
    ipcRenderer.on("backend:event", listener);
    return () => ipcRenderer.removeListener("backend:event", listener);
  },
  resolveAssetUrl,
});

queueMicrotask(() => {
  void getRenderInfoCached();
  void probeWebglTextureRendererCached();
});
