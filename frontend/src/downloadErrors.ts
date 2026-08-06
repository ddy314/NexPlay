export function friendlyDownloadError(error: unknown): string {
  const raw = error instanceof Error ? error.message : String(error ?? "");
  const message = raw.trim();
  const normalized = message.replace(/^(?:api|http|config) error:\s*/i, "").trim();
  const lower = normalized.toLowerCase();

  if (!message) return "下载失败，请稍后再试。";
  if (normalized.includes("BT ")) return normalized;
  if (
    lower.includes("api/v2/auth/login") ||
    lower.includes("error sending request") ||
    lower.includes("connection refused") ||
    lower.includes("failed to connect")
  ) {
    return "BT 未启动或无法连接，请先启动 qBittorrent。";
  }
  if (
    lower.includes("login rejected") ||
    lower.includes("login failed") ||
    lower.includes("authentication failed")
  ) {
    return "BT 登录失败，请检查 qBittorrent 的用户名和密码。";
  }
  if (lower.includes("integration is disabled")) {
    return "BT 下载未启用，请到设置 > 下载中启用 qBittorrent。";
  }
  if (lower.includes("invalid qbittorrent") || lower.includes("missing origin")) {
    return "BT 地址设置有误，请检查 qBittorrent WebUI 地址。";
  }
  if (lower.includes("did not expose torrent files")) {
    return "BT 还在准备种子文件，请稍后再试。";
  }
  if (lower.includes("torrents/add")) {
    return "BT 添加资源失败，请检查 qBittorrent 设置。";
  }
  if (lower.includes("qbittorrent") || lower.includes("torrent")) {
    return "BT 操作失败，请检查 qBittorrent 设置后重试。";
  }
  return "下载失败，请稍后再试。";
}
