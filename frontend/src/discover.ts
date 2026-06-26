import type { Subject } from "./data";

// Public Bangumi discovery feed (每日放送 / 热门新番).
// The renderer already calls the public api.bgm.tv directly (see subjectHydration.ts),
// so recommendations are no longer limited to local/collected subjects.

const CALENDAR_URL = "https://api.bgm.tv/calendar";
const CACHE_KEY = "nexplay.discover.calendar.v1";
const CACHE_TTL_MS = 6 * 60 * 60 * 1000; // 6h

export type DiscoveryFeed = {
  today: Subject[];
  trending: Subject[];
};

type CalendarItem = {
  id?: number;
  name?: string;
  name_cn?: string;
  summary?: string;
  air_date?: string;
  air_weekday?: number;
  eps?: number;
  rank?: number;
  collection?: { collect?: number; doing?: number; wish?: number };
  rating?: { score?: number; rank?: number; total?: number };
  images?: { large?: string; common?: string; medium?: string; grid?: string };
};

type CalendarDay = {
  weekday?: { id?: number; en?: string; cn?: string };
  items?: CalendarItem[];
};

let inflight: Promise<DiscoveryFeed> | null = null;

export function fetchBangumiDiscovery(): Promise<DiscoveryFeed> {
  if (inflight) return inflight;
  inflight = loadFeed().catch((error) => {
    inflight = null; // allow retry on next mount
    throw error;
  });
  return inflight;
}

async function loadFeed(): Promise<DiscoveryFeed> {
  const cached = readCache();
  if (cached) return cached;

  const response = await fetch(CALENDAR_URL, { headers: { Accept: "application/json" } });
  if (!response.ok) {
    throw new Error(`日历接口返回 ${response.status}`);
  }
  const days = (await response.json()) as CalendarDay[];
  const feed = buildFeed(days);
  writeCache(feed);
  return feed;
}

function buildFeed(days: CalendarDay[]): DiscoveryFeed {
  const jsWeekday = new Date().getDay(); // 0=Sun..6=Sat
  const bgmWeekday = jsWeekday === 0 ? 7 : jsWeekday; // bgm: 1=Mon..7=Sun

  const all: Subject[] = [];
  let today: Subject[] = [];
  for (const day of days) {
    const items = (day.items ?? []).map(toSubject).filter((s) => s.title || s.titleCn);
    all.push(...items);
    if (day.weekday?.id === bgmWeekday) today = items;
  }

  const seen = new Set<string>();
  const trending = all
    .filter((s) => {
      if (seen.has(s.providerSubjectId)) return false;
      seen.add(s.providerSubjectId);
      return true;
    })
    .sort(trendingSorter);

  if (!today.length) today = trending.slice(0, 16);
  return { today, trending };
}

function trendingSorter(a: Subject, b: Subject) {
  // Popularity is dominated by collection counts; fall back to score then rank.
  const pop = (popularityOf(b) - popularityOf(a));
  if (pop) return pop;
  if (b.rating !== a.rating) return b.rating - a.rating;
  const ra = a.rank > 0 ? a.rank : Number.MAX_SAFE_INTEGER;
  const rb = b.rank > 0 ? b.rank : Number.MAX_SAFE_INTEGER;
  return ra - rb;
}

const popularityWeights = new WeakMap<Subject, number>();
function popularityOf(subject: Subject) {
  return popularityWeights.get(subject) ?? 0;
}

function toSubject(item: CalendarItem): Subject {
  const images = item.images ?? {};
  const airDate = typeof item.air_date === "string" ? item.air_date : "";
  const subject: Subject = {
    id: `discover-bgm-${item.id ?? Math.random().toString(36).slice(2)}`,
    mediaId: 0,
    subjectId: item.id ?? 0,
    source: "discover",
    provider: "bangumi",
    providerSubjectId: item.id ? String(item.id) : "",
    local: false,
    aliases: [],
    title: (item.name ?? "").trim(),
    titleCn: (item.name_cn ?? "").trim(),
    year: yearFromDate(airDate),
    airDate,
    rating: finite(item.rating?.score),
    rank: finite(item.rank) || finite(item.rating?.rank),
    tags: [],
    summary: (item.summary ?? "").trim(),
    poster: images.large || images.common || images.medium || images.grid || "",
    hero: images.common || images.large || images.medium || "",
    status: "unmatched",
    episodes: finite(item.eps),
    watchedEpisodes: 0,
    progress: 0,
    bgmCollectionLabel: "",
    bgmRate: 0,
    bgmPending: false,
    files: 0,
    totalSize: "",
    newEpisode: false,
    metadataReady: true,
    fileSummary: "",
    localFiles: [],
    episodesDetail: [],
  };
  const collection = item.collection ?? {};
  const popularity = finite(collection.collect) + finite(collection.doing) + finite(collection.wish) * 0.5;
  popularityWeights.set(subject, popularity);
  return subject;
}

function readCache(): DiscoveryFeed | null {
  try {
    const raw = window.sessionStorage.getItem(CACHE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as { at: number; feed: DiscoveryFeed };
    if (!parsed?.feed || Date.now() - parsed.at > CACHE_TTL_MS) return null;
    return parsed.feed;
  } catch {
    return null;
  }
}

function writeCache(feed: DiscoveryFeed) {
  try {
    window.sessionStorage.setItem(CACHE_KEY, JSON.stringify({ at: Date.now(), feed }));
  } catch {
    // ignore quota / private mode
  }
}

function finite(value: unknown) {
  return typeof value === "number" && Number.isFinite(value) ? value : 0;
}

function yearFromDate(value: string) {
  const parsed = Number(value.slice(0, 4));
  return Number.isFinite(parsed) ? parsed : 0;
}
