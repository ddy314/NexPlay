import { useEffect, useMemo, useState } from "react";
import {
  bangumiAuthStatus,
  clearPlaybackAnalytics,
  logoutBangumi,
  startBangumiLogin,
  syncBangumiNow,
  testQbittorrentConnection,
  type BangumiAuthStatus,
  type EditableSettings,
} from "../backend";
import { friendlyDownloadError } from "../downloadErrors";
import { Button, Card, Dropdown, Switch } from "../ui";
import { ChevronRight, KeyIcon } from "../icons";
import { cn } from "../utils/cn";

type Section = "libraries" | "accounts" | "playback" | "downloads" | "appearance" | "privacy" | "advanced";

const sections: { id: Section; label: string; desc: string }[] = [
  { id: "libraries", label: "媒体来源", desc: "目录与本地数据库" },
  { id: "accounts", label: "账户与元数据", desc: "Bangumi 与匹配" },
  { id: "playback", label: "播放与弹幕", desc: "DanDanPlay 凭证" },
  { id: "downloads", label: "下载", desc: "Nyaa 与 qBittorrent" },
  { id: "appearance", label: "外观与辅助功能", desc: "主题和动态效果" },
  { id: "privacy", label: "隐私与洞察", desc: "本地记录与目标" },
  { id: "advanced", label: "高级", desc: "接口、日志与构建" },
];

const emptySettings: EditableSettings = {
  mediaLibraries: [],
  databasePath: "data/nexplay.sqlite3",
  bangumiEnabled: true,
  bangumiBaseUrl: "https://api.bgm.tv",
  bangumiOauthBaseUrl: "https://bgm.tv",
  bangumiClientId: "",
  bangumiClientSecret: "",
  bangumiClientSecretConfigured: false,
  bangumiRedirectUri: "http://127.0.0.1:17654/bangumi/callback",
  bangumiAccessToken: "",
  bangumiAccessTokenConfigured: false,
  bangumiUserAgent: "NexPlay/0.1.0",
  bangumiRequestTimeoutSecs: 20,
  bangumiAutoMatch: true,
  bangumiCacheImages: true,
  dandanplayAppId: "",
  dandanplayAppSecret: "",
  dandanplayAppSecretConfigured: false,
  dandanplayApiKey: "",
  dandanplayApiKeyConfigured: false,
  nyaaEnabled: true,
  nyaaBaseUrl: "https://nyaa.si",
  nyaaCategory: "0_0",
  qbittorrentEnabled: false,
  qbittorrentBaseUrl: "http://127.0.0.1:8080",
  qbittorrentUsername: "admin",
  qbittorrentPassword: "",
  qbittorrentPasswordConfigured: false,
  qbittorrentSavePath: "",
  qbittorrentCategory: "NexPlay",
  qbittorrentTags: "nexplay",
  theme: "system",
  reducedMotion: false,
  analyticsEnabled: true,
  dailyMinutesGoal: 45,
  weeklyEpisodesGoal: 5,
  weeklyActiveDaysGoal: 4,
  loggingLevel: "info",
};

export function SettingsPage({
  onSnack,
  onDirtyChange,
}: {
  onSnack: (text: string, tone?: "neutral" | "success" | "danger") => void;
  onDirtyChange?: (dirty: boolean) => void;
}) {
  const [section, setSection] = useState<Section>("libraries");
  const [settings, setSettings] = useState<EditableSettings>(emptySettings);
  const [librariesText, setLibrariesText] = useState("");
  const [loading, setLoading] = useState(Boolean(window.nexplay));
  const [saving, setSaving] = useState(false);
  const [testingQbit, setTestingQbit] = useState(false);
  const [bangumiAuth, setBangumiAuth] = useState<BangumiAuthStatus | null>(null);
  const [bangumiBusy, setBangumiBusy] = useState<"login" | "sync" | "logout" | null>(null);
  const [showSecrets, setShowSecrets] = useState(false);
  const [savedSignature, setSavedSignature] = useState("");

  useEffect(() => {
    let alive = true;
    if (!window.nexplay) {
      setLoading(false);
      onSnack("当前页面没有连接到 NexPlay 后端，请从应用窗口使用设置页。", "danger");
      return;
    }

    window.nexplay
      .getSettings()
      .then((next) => {
        if (!alive) return;
        setSettings(next);
        setLibrariesText(next.mediaLibraries.join("\n"));
        setSavedSignature(JSON.stringify(next));
        void bangumiAuthStatus().then((status) => {
          if (alive) setBangumiAuth(status);
        });
      })
      .catch((caught) => {
        const message = caught instanceof Error ? caught.message : String(caught);
        onSnack(`读取设置失败：${message}`, "danger");
      })
      .finally(() => {
        if (alive) setLoading(false);
      });

    return () => {
      alive = false;
    };
  }, []);

  const normalizedSettings = useMemo(
    () => ({
      ...settings,
      mediaLibraries: librariesText
        .split("\n")
        .map((item) => item.trim())
        .filter(Boolean),
    }),
    [librariesText, settings]
  );
  const dirty = savedSignature.length > 0 && JSON.stringify(normalizedSettings) !== savedSignature;

  useEffect(() => {
    onDirtyChange?.(dirty);
    const beforeUnload = (event: BeforeUnloadEvent) => {
      if (!dirty) return;
      event.preventDefault();
      event.returnValue = "";
    };
    window.addEventListener("beforeunload", beforeUnload);
    return () => window.removeEventListener("beforeunload", beforeUnload);
  }, [dirty, onDirtyChange]);
  useEffect(() => () => onDirtyChange?.(false), [onDirtyChange]);

  const save = async () => {
    if (!window.nexplay) {
      onSnack("当前不是 Electron 环境，无法保存设置。", "danger");
      return;
    }

    setSaving(true);
    try {
      const saved = await window.nexplay.saveSettings(normalizedSettings);
      setSettings(saved);
      setLibrariesText(saved.mediaLibraries.join("\n"));
      setSavedSignature(JSON.stringify(saved));
      window.dispatchEvent(new CustomEvent("nexplay-settings-changed", { detail: saved }));
      onSnack("设置已保存到后端配置。", "success");
    } catch (caught) {
      const message = caught instanceof Error ? caught.message : String(caught);
      onSnack(`保存设置失败：${message}`, "danger");
    } finally {
      setSaving(false);
    }
  };

  const update = <K extends keyof EditableSettings>(key: K, value: EditableSettings[K]) => {
    setSettings((current) => ({ ...current, [key]: value }));
  };

  const testQbit = async () => {
    if (!window.nexplay) {
      onSnack("当前不是 Electron 环境，无法测试 qBittorrent。", "danger");
      return;
    }

    setTestingQbit(true);
    try {
      const saved = await window.nexplay.saveSettings(normalizedSettings);
      setSettings(saved);
      setLibrariesText(saved.mediaLibraries.join("\n"));
      setSavedSignature(JSON.stringify(saved));
      window.dispatchEvent(new CustomEvent("nexplay-settings-changed", { detail: saved }));
      const result = await testQbittorrentConnection();
      onSnack(result.message, result.ok ? "success" : "danger");
    } catch (caught) {
      const message = friendlyDownloadError(caught);
      onSnack(message, "danger");
    } finally {
      setTestingQbit(false);
    }
  };

  const startBangumiOAuth = async () => {
    if (!window.nexplay) {
      onSnack("当前不是 Electron 环境，无法启动 Bangumi 登录。", "danger");
      return;
    }
    setBangumiBusy("login");
    try {
      const saved = await window.nexplay.saveSettings(normalizedSettings);
      setSettings(saved);
      setLibrariesText(saved.mediaLibraries.join("\n"));
      setSavedSignature(JSON.stringify(saved));
      window.dispatchEvent(new CustomEvent("nexplay-settings-changed", { detail: saved }));
      const login = await startBangumiLogin();
      onSnack(`已打开 Bangumi 登录页面，回调地址：${login.redirectUri}`, "success");
    } catch (caught) {
      const message = caught instanceof Error ? caught.message : String(caught);
      onSnack(`启动 Bangumi 登录失败：${message}`, "danger");
    } finally {
      setBangumiBusy(null);
    }
  };

  const refreshBangumiAuth = async () => {
    try {
      setBangumiAuth(await bangumiAuthStatus());
    } catch {
      // Auth status is best-effort in the settings view.
    }
  };

  const syncBangumi = async () => {
    setBangumiBusy("sync");
    try {
      const result = await syncBangumiNow();
      await refreshBangumiAuth();
      onSnack(result.message, "success");
    } catch (caught) {
      const message = caught instanceof Error ? caught.message : String(caught);
      onSnack(`Bangumi 同步失败：${message}`, "danger");
    } finally {
      setBangumiBusy(null);
    }
  };

  const logout = async () => {
    setBangumiBusy("logout");
    try {
      const status = await logoutBangumi();
      setBangumiAuth(status);
      onSnack("已退出 Bangumi 账号。", "success");
    } catch (caught) {
      const message = caught instanceof Error ? caught.message : String(caught);
      onSnack(`退出 Bangumi 失败：${message}`, "danger");
    } finally {
      setBangumiBusy(null);
    }
  };

  useEffect(() => {
    if (!window.nexplay?.onBackendEvent) return;
    return window.nexplay.onBackendEvent((event) => {
      if (event.type === "bangumiOAuthCompleted") {
        void refreshBangumiAuth();
        onSnack(`Bangumi 登录完成：${event.message ?? ""}`, "success");
      }
      if (event.type === "bangumiOAuthFailed") {
        onSnack(`Bangumi 登录失败：${event.message ?? ""}`, "danger");
      }
    });
  }, [onSnack]);

  return (
    <div className="page-shell h-full overflow-y-auto">
      <div className="mb-8 flex flex-wrap items-end justify-between gap-5">
        <div>
          <div className="nx-eyebrow">System map</div>
          <h1 className="nx-page-title">设置</h1>
          <div className="nx-page-subtitle">
            按任务组织媒体来源、账户、播放、下载、外观与隐私。
          </div>
        </div>
        <Button onClick={save} loading={saving} disabled={loading || !dirty} className="h-10 px-5 text-[13px]">
          {dirty ? "保存设置" : "已保存"}
        </Button>
      </div>

      <div className="grid grid-cols-1 items-start gap-6 lg:grid-cols-[220px_minmax(0,1fr)]">
        <Card className="settings-section-list p-2 lg:sticky lg:top-6">
          {sections.map((item) => (
            <button
              key={item.id}
              onClick={() => setSection(item.id)}
              className={cn(
                "settings-option group flex w-full items-center gap-3 rounded-[var(--radius-card)] px-3.5 py-3 text-left transition-all",
                section === item.id
                  ? "bg-[var(--color-primary-soft)] text-[var(--color-primary)]"
                  : "text-[var(--color-on-surface)] hover:bg-black/[0.045]"
              )}
            >
              <div className="flex-1 min-w-0">
                <div className="text-[14px] font-semibold">{item.label}</div>
                <div
                  className={cn(
                    "mt-0.5 text-[11.5px] font-medium",
                    section === item.id
                      ? "text-[var(--color-primary)]/70"
                      : "text-[var(--color-on-surface-faint)] group-hover:text-[var(--color-on-surface-muted)]"
                  )}
                >
                  {item.desc}
                </div>
              </div>
              <ChevronRight className="size-4 opacity-50" />
            </button>
          ))}
        </Card>

        <div className="space-y-6">
          {section === "libraries" && (
            <Group title="媒体目录" desc="每行一个目录；保存时后端会校验目录必须存在。">
              <div className="px-6 py-5">
                <textarea
                  value={librariesText}
                  onChange={(event) => setLibrariesText(event.target.value)}
                  spellCheck={false}
                  className="min-h-36 w-full resize-y rounded-[var(--radius-card)] bg-[var(--color-surface-3)] ring-1 ring-inset ring-[var(--color-outline-soft)] focus:ring-[var(--color-primary)]/40 px-3 py-3 text-[13px] outline-none font-mono"
                  placeholder="/path/to/anime/library"
                />
              </div>
              <SettingsRow
                title="数据库路径"
                desc="后端 SQLite 数据库位置；保存后重启 NexPlay 才会切换。"
                effect="重启后生效"
                control={
                  <TextInput
                    value={settings.databasePath}
                    onChange={(value) => update("databasePath", value)}
                    className="w-full font-mono"
                  />
                }
              />
              <SettingsRow
                title="日志级别"
                desc="写入配置，供后端日志层读取。"
                effect="保存后生效"
                control={
                  <Dropdown
                    size="sm"
                    value={settings.loggingLevel}
                    onChange={(value) => update("loggingLevel", value)}
                    matchWidth={false}
                    className="min-w-[112px]"
                    options={[
                      { value: "error", label: "error" },
                      { value: "warn", label: "warn" },
                      { value: "info", label: "info" },
                      { value: "debug", label: "debug" },
                      { value: "trace", label: "trace" },
                    ]}
                  />
                }
              />
            </Group>
          )}

          {section === "accounts" && (
            <Group title="Bangumi" desc="账号状态同步、条目查询、自动匹配和图片缓存。">
              <div className="mx-6 mt-5 rounded-[var(--radius-card)] bg-[var(--color-accent-soft)] px-4 py-3 text-[12px] leading-relaxed text-[var(--color-on-surface-muted)]">
                <span className="font-semibold text-[var(--color-accent)]">数据来源说明：</span>
                登录 Bangumi 后，你的收藏状态、评分与单集进度都以账号云端为准。打开条目详情会自动拉取该番剧的单集状态；看完一集（进度 ≥ 90%）会自动标记为「看过」并回写云端，离线时进入待同步队列、下次同步自动重试。
              </div>
              <div className="px-6 py-5">
                <div className="flex flex-wrap items-center justify-between gap-3">
                  <div className="min-w-0">
                    <div className="text-[14px] font-semibold">
                      {bangumiAuth?.authenticated
                        ? `已登录 ${bangumiAuth.nickname || bangumiAuth.username || "Bangumi"}`
                        : "未登录 Bangumi"}
                    </div>
                    <div className="mt-1 text-[12px] font-medium text-[var(--color-on-surface-faint)]">
                      {bangumiAuth?.pendingSyncCount
                        ? `${bangumiAuth.pendingSyncCount} 个修改待同步`
                        : bangumiAuth?.clientConfigured
                          ? "OAuth 客户端已配置"
                          : "需要先配置 Client ID 和 Client Secret"}
                    </div>
                    {bangumiAuth?.lastError && (
                      <div className="mt-1 text-[12px] font-medium text-rose-600">
                        {bangumiAuth.lastError}
                      </div>
                    )}
                  </div>
                  <div className="flex flex-wrap items-center gap-2">
                    <Button
                      onClick={startBangumiOAuth}
                      loading={bangumiBusy === "login"}
                      className="h-9 px-4 text-[13px]"
                    >
                      {bangumiAuth?.authenticated ? "重新登录" : "登录"}
                    </Button>
                    <Button
                      variant="tonal"
                      onClick={syncBangumi}
                      loading={bangumiBusy === "sync"}
                      disabled={!bangumiAuth?.authenticated}
                      className="h-9 px-4 text-[13px]"
                    >
                      立即同步
                    </Button>
                    <Button
                      variant="text"
                      onClick={logout}
                      loading={bangumiBusy === "logout"}
                      disabled={!bangumiAuth?.authenticated}
                      className="h-9 px-3 text-[13px]"
                    >
                      退出
                    </Button>
                  </div>
                </div>
              </div>
              <SettingsRow
                title="启用 Bangumi"
                control={<Switch checked={settings.bangumiEnabled} onChange={(value) => update("bangumiEnabled", value)} />}
              />
              <SettingsRow
                title="自动匹配"
                desc="扫描后自动查询 Bangumi 并写入匹配结果。"
                control={<Switch checked={settings.bangumiAutoMatch} onChange={(value) => update("bangumiAutoMatch", value)} />}
              />
              <SettingsRow
                title="缓存图片"
                desc="保存海报和头图到本地缓存。"
                control={<Switch checked={settings.bangumiCacheImages} onChange={(value) => update("bangumiCacheImages", value)} />}
              />
              <SettingsRow
                title="API 地址"
                desc="Bangumi OpenAPI 主机。"
                control={
                  <TextInput
                    value={settings.bangumiBaseUrl}
                    onChange={(value) => update("bangumiBaseUrl", value)}
                    className="w-full font-mono"
                  />
                }
              />
              <SettingsRow
                title="OAuth 地址"
                desc="用于 authorize/access_token，默认 https://bgm.tv。"
                control={
                  <TextInput
                    value={settings.bangumiOauthBaseUrl}
                    onChange={(value) => update("bangumiOauthBaseUrl", value)}
                    className="w-full font-mono"
                  />
                }
              />
              <SettingsRow
                title="Client ID"
                control={
                  <TextInput
                    value={settings.bangumiClientId}
                    onChange={(value) => update("bangumiClientId", value)}
                    className="w-full font-mono"
                  />
                }
              />
              <SettingsRow
                title="Client Secret"
                desc={settings.bangumiClientSecretConfigured ? "已配置；留空保存不会覆盖旧值。" : "本地保存，不会提交到仓库。"}
                control={
                  <SecretInput
                    value={settings.bangumiClientSecret}
                    show={showSecrets}
                    onToggleShow={() => setShowSecrets((value) => !value)}
                    placeholder={settings.bangumiClientSecretConfigured ? "已配置，输入新值以修改" : ""}
                    onChange={(value) => update("bangumiClientSecret", value)}
                  />
                }
              />
              <SettingsRow
                title="回调地址"
                desc="Bangumi 应用后台需要登记同一个 redirect URI。"
                control={
                  <TextInput
                    value={settings.bangumiRedirectUri}
                    onChange={(value) => update("bangumiRedirectUri", value)}
                    className="w-full font-mono"
                  />
                }
              />
              <SettingsRow
                title="User Agent"
                control={
                  <TextInput
                    value={settings.bangumiUserAgent}
                    onChange={(value) => update("bangumiUserAgent", value)}
                    className="w-full font-mono"
                  />
                }
              />
              <SettingsRow
                title="超时"
                desc="单位：秒"
                control={
                  <input
                    type="number"
                    min={1}
                    value={settings.bangumiRequestTimeoutSecs}
                    onChange={(event) => update("bangumiRequestTimeoutSecs", Number(event.target.value) || 1)}
                    className="h-9 w-24 rounded-[var(--radius-control)] bg-[var(--color-surface-3)] px-3 text-[13px] outline-none ring-1 ring-inset ring-[var(--color-outline-soft)]"
                  />
                }
              />
              <SettingsRow
                title="手动 Access Token"
                desc={settings.bangumiAccessTokenConfigured ? "已配置；仅用于元数据查询，账号同步请用上方 OAuth 登录。留空保存不会覆盖旧值。" : "可选，仅用于公开元数据查询。账号同步请用 OAuth 登录，无需手动填写。"}
                control={
                  <SecretInput
                    value={settings.bangumiAccessToken}
                    show={showSecrets}
                    onToggleShow={() => setShowSecrets((value) => !value)}
                    placeholder={settings.bangumiAccessTokenConfigured ? "已配置，输入新值以修改" : ""}
                    onChange={(value) => update("bangumiAccessToken", value)}
                  />
                }
              />
            </Group>
          )}

          {section === "playback" && (
            <Group title="DanDanPlay" desc="用于按单集文件名和哈希匹配弹幕。">
              <SettingsRow
                title="App ID"
                control={
                  <TextInput
                    value={settings.dandanplayAppId}
                    onChange={(value) => update("dandanplayAppId", value)}
                    className="w-full font-mono"
                  />
                }
              />
              <SettingsRow
                title="App Secret"
                control={
                  <SecretInput
                    value={settings.dandanplayAppSecret}
                    show={showSecrets}
                    onToggleShow={() => setShowSecrets((value) => !value)}
                    onChange={(value) => update("dandanplayAppSecret", value)}
                  />
                }
              />
              <SettingsRow
                title="API Key"
                desc="当前后端保留该字段；弹幕签名主要使用 App ID 和 App Secret。"
                control={
                  <SecretInput
                    value={settings.dandanplayApiKey}
                    show={showSecrets}
                    onToggleShow={() => setShowSecrets((value) => !value)}
                    onChange={(value) => update("dandanplayApiKey", value)}
                  />
                }
              />
            </Group>
          )}

          {section === "downloads" && (
            <Group title="Nyaa" desc="用于在线搜索番剧资源候选。">
              <SettingsRow
                title="启用 Nyaa"
                control={<Switch checked={settings.nyaaEnabled} onChange={(value) => update("nyaaEnabled", value)} />}
              />
              <SettingsRow
                title="RSS 地址"
                desc="默认使用 nyaa.si 的 RSS 查询。"
                control={
                  <TextInput
                    value={settings.nyaaBaseUrl}
                    onChange={(value) => update("nyaaBaseUrl", value)}
                    className="w-full font-mono"
                  />
                }
              />
              <SettingsRow
                title="分类"
                desc="Nyaa 分类 ID，默认 0_0 为全分类搜索。"
                control={
                  <TextInput
                    value={settings.nyaaCategory}
                    onChange={(value) => update("nyaaCategory", value)}
                    className="w-full font-mono"
                  />
                }
              />
            </Group>
          )}

          {section === "downloads" && (
            <Group title="qBittorrent" desc="通过 WebUI API 添加种子并回读任务状态。">
              <SettingsRow
                title="启用下载器"
                control={<Switch checked={settings.qbittorrentEnabled} onChange={(value) => update("qbittorrentEnabled", value)} />}
              />
              <SettingsRow
                title="WebUI 地址"
                control={
                  <TextInput
                    value={settings.qbittorrentBaseUrl}
                    onChange={(value) => update("qbittorrentBaseUrl", value)}
                    className="w-full font-mono"
                  />
                }
              />
              <SettingsRow
                title="用户名"
                control={
                  <TextInput
                    value={settings.qbittorrentUsername}
                    onChange={(value) => update("qbittorrentUsername", value)}
                    className="w-full font-mono"
                  />
                }
              />
              <SettingsRow
                title="密码"
                control={
                  <SecretInput
                    value={settings.qbittorrentPassword}
                    show={showSecrets}
                    onToggleShow={() => setShowSecrets((value) => !value)}
                    onChange={(value) => update("qbittorrentPassword", value)}
                  />
                }
              />
              <SettingsRow
                title="保存路径"
                desc="留空时使用 qBittorrent 默认路径。"
                control={
                  <TextInput
                    value={settings.qbittorrentSavePath}
                    onChange={(value) => update("qbittorrentSavePath", value)}
                    className="w-full font-mono"
                  />
                }
              />
              <SettingsRow
                title="分类"
                control={
                  <TextInput
                    value={settings.qbittorrentCategory}
                    onChange={(value) => update("qbittorrentCategory", value)}
                    className="w-full font-mono"
                  />
                }
              />
              <SettingsRow
                title="标签"
                control={
                  <TextInput
                    value={settings.qbittorrentTags}
                    onChange={(value) => update("qbittorrentTags", value)}
                    className="w-full font-mono"
                  />
                }
              />
              <div className="px-6 py-4">
                <Button onClick={testQbit} loading={testingQbit} className="h-9 px-4 text-[13px]">
                  测试连接
                </Button>
              </div>
            </Group>
          )}

          {section === "appearance" && (
            <Group title="外观与辅助功能" desc="主题状态应用到整个文档，包括浮层、对话框与原生表单。">
              <SettingsRow title="界面主题" desc="跟随系统会在系统外观变化时使用对应主题。" effect="保存后立即生效" control={<Dropdown size="sm" value={settings.theme} onChange={(value) => update("theme", value)} matchWidth={false} className="min-w-[132px]" options={[{ value: "system", label: "跟随系统" }, { value: "light", label: "浅色" }, { value: "dark", label: "深色" }]} />} />
              <SettingsRow title="减少动态效果" desc="关闭空间位移、缩放和非必要的过渡动画。" effect="保存后立即生效" control={<Switch checked={settings.reducedMotion} onChange={(value) => update("reducedMotion", value)} />} />
            </Group>
          )}

          {section === "privacy" && (
            <Group title="隐私与观看洞察" desc="所有播放会话与洞察数据只保存在本机 SQLite 数据库。">
              <SettingsRow title="记录本地观看会话" desc="记录低频开始、暂停、跳转、完成和 30 秒心跳；不记录逐帧数据。" control={<Switch checked={settings.analyticsEnabled} onChange={(value) => update("analyticsEnabled", value)} />} />
              <SettingsRow title="每日观看目标" desc="洞察红色圆环的分钟目标。" control={<GoalInput value={settings.dailyMinutesGoal} min={1} max={1440} unit="分钟" onChange={(value) => update("dailyMinutesGoal", value)} />} />
              <SettingsRow title="每周完成目标" desc="洞察绿色圆环的完成集数目标。" control={<GoalInput value={settings.weeklyEpisodesGoal} min={1} max={100} unit="集" onChange={(value) => update("weeklyEpisodesGoal", value)} />} />
              <SettingsRow title="每周活跃目标" desc="洞察青色圆环的活跃天数目标。" control={<GoalInput value={settings.weeklyActiveDaysGoal} min={1} max={7} unit="天" onChange={(value) => update("weeklyActiveDaysGoal", value)} />} />
              <div className="flex justify-end border-t border-[var(--nx-line)] px-6 py-4"><button type="button" className="nx-button danger" onClick={async () => { if (!window.confirm("确定清除所有本地观看洞察记录？现有断点进度不会删除。")) return; await clearPlaybackAnalytics(); onSnack("本地观看洞察记录已清除。", "success"); }}>清除洞察历史</button></div>
            </Group>
          )}

          {section === "advanced" && (
            <Card className="p-8">
              <div className="text-[20px] font-semibold">NexPlay · 本地番剧库</div>
              <div className="text-[13px] text-[var(--color-on-surface-muted)] mt-2">
                配置、扫描和媒体库快照都由本地 Rust 后端统一处理。
              </div>
            </Card>
          )}
        </div>
      </div>
    </div>
  );
}

function GoalInput({ value, min, max, unit, onChange }: { value: number; min: number; max: number; unit: string; onChange: (value: number) => void }) {
  return <label className="flex h-10 items-center gap-2 rounded-[12px] bg-[var(--nx-plane-2)] px-3"><input type="number" value={value} min={min} max={max} onChange={(event) => onChange(Math.min(max, Math.max(min, Number(event.target.value) || min)))} className="w-16 bg-transparent text-right text-[13px] font-semibold outline-none" /><span className="text-[11px] text-[var(--nx-ink-3)]">{unit}</span></label>;
}

function Group({ title, desc, children }: { title: string; desc?: string; children: React.ReactNode }) {
  return (
    <Card>
      <div className="px-6 pt-5 pb-4 border-b border-[var(--color-outline-soft)]">
        <div className="text-[17px] font-semibold tracking-tight">{title}</div>
        {desc && <div className="mt-1 text-[12.5px] font-medium text-[var(--color-on-surface-faint)]">{desc}</div>}
      </div>
      <div className="divide-y divide-[var(--color-outline-soft)]">{children}</div>
    </Card>
  );
}

function SettingsRow({
  title,
  desc,
  effect,
  control,
}: {
  title: string;
  desc?: string;
  effect?: string;
  control: React.ReactNode;
}) {
  return (
    <div className="grid grid-cols-1 items-center gap-3 px-6 py-4 sm:grid-cols-[180px_minmax(0,1fr)] sm:gap-5">
      <div className="flex-1 min-w-0">
        <div className="text-[14px] font-semibold">{title}</div>
        {desc && <div className="mt-0.5 text-[12px] font-medium text-[var(--color-on-surface-faint)]">{desc}</div>}
        {effect && <div className="mt-1 inline-flex rounded-full bg-[var(--color-primary-soft)] px-2 py-0.5 text-[10px] font-semibold text-[var(--color-primary)]">{effect}</div>}
      </div>
      <div className="min-w-0 justify-self-stretch">{control}</div>
    </div>
  );
}

function TextInput({
  value,
  onChange,
  className,
}: {
  value: string;
  onChange: (value: string) => void;
  className?: string;
}) {
  return (
    <input
      value={value}
      onChange={(event) => onChange(event.target.value)}
      className={cn(
        "h-9 rounded-[var(--radius-control)] bg-[var(--color-surface-3)] px-3 text-[13px] outline-none ring-1 ring-inset ring-[var(--color-outline-soft)] focus:ring-[var(--color-primary)]/40",
        className
      )}
    />
  );
}

function SecretInput({
  value,
  show,
  onToggleShow,
  onChange,
  placeholder,
}: {
  value: string;
  show: boolean;
  onToggleShow: () => void;
  onChange: (value: string) => void;
  placeholder?: string;
}) {
  return (
    <div className="flex items-center gap-2">
      <div className="relative w-full">
        <input
          type={show ? "text" : "password"}
          value={value}
          placeholder={placeholder}
          onChange={(event) => onChange(event.target.value)}
          className="h-9 w-full rounded-[var(--radius-control)] bg-[var(--color-surface-3)] pl-9 pr-3 font-mono text-[13px] outline-none ring-1 ring-inset ring-[var(--color-outline-soft)] focus:ring-[var(--color-primary)]/40"
        />
        <KeyIcon className="size-4 absolute left-3 top-1/2 -translate-y-1/2 text-[var(--color-on-surface-faint)]" />
      </div>
      <Button variant="text" size="sm" onClick={onToggleShow}>
        {show ? "隐藏" : "显示"}
      </Button>
    </div>
  );
}
