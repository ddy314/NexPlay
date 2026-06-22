const assetUrlCache = new Map<string, string>();
const MAX_ASSET_URL_CACHE = 1600;

export function resolveAssetUrl(value?: string | null) {
  if (!value) {
    return "";
  }

  const cached = assetUrlCache.get(value);
  if (cached) {
    assetUrlCache.delete(value);
    assetUrlCache.set(value, cached);
    return cached;
  }

  const resolved = window.nexplay?.resolveAssetUrl(value) ?? value;
  assetUrlCache.set(value, resolved);
  while (assetUrlCache.size > MAX_ASSET_URL_CACHE) {
    const oldest = assetUrlCache.keys().next().value;
    if (!oldest) break;
    assetUrlCache.delete(oldest);
  }
  return resolved;
}
