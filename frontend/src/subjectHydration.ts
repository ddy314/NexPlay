import { loadOnlineSubject, searchCatalog } from "./backend";
import type { Subject } from "./data";

export function shouldHydrateSubject(subject: Subject) {
  return subject.provider !== "manual"
    && subject.providerSubjectId.trim().length > 0
    && (
      !hasUsableSummary(subject.summary)
      || !subject.poster.trim()
      || !subject.hero.trim()
      || subject.tags.length === 0
    );
}

export async function loadSubjectDetailWithFallback(subject: Subject) {
  let detail: Subject | null = null;
  let detailError: unknown = null;
  try {
    detail = await loadOnlineSubject(subject.provider, subject.providerSubjectId);
  } catch (caught) {
    detailError = caught;
  }

  let base = detail ?? subject;
  const publicFallback = await loadBangumiPublicSubject(base);
  if (publicFallback) {
    base = mergeMissingDisplayMetadata(base, publicFallback);
  }

  if (hasCompleteDisplayMetadata(base)) {
    return base;
  }

  const fallback = await findSubjectMetadataFallback(base);
  if (fallback) {
    return mergeMissingDisplayMetadata(base, fallback);
  }

  if (detail) {
    return detail;
  }
  throw detailError instanceof Error ? detailError : new Error(String(detailError || "读取在线详情失败"));
}

export function hasUsableSummary(summary: string) {
  const text = summary.trim();
  if (!text) return false;
  return text !== "No summary cached yet."
    && text !== "No summary in candidate."
    && text !== "No summary cached yet"
    && text !== "No summary in candidate";
}

export function mergeMissingDisplayMetadata(base: Subject, fallback: Subject): Subject {
  return {
    ...base,
    summary: hasUsableSummary(base.summary) ? base.summary : fallback.summary,
    poster: base.poster || fallback.poster,
    hero: base.hero || fallback.hero || fallback.poster,
    rating: base.rating || fallback.rating,
    rank: base.rank || fallback.rank,
    year: base.year || fallback.year,
    airDate: base.airDate || fallback.airDate,
    tags: base.tags.length ? base.tags : fallback.tags,
    aliases: base.aliases.length ? base.aliases : fallback.aliases,
    title: base.title || fallback.title,
    titleCn: base.titleCn || fallback.titleCn,
    episodes: base.episodes || fallback.episodes,
    metadataReady: base.metadataReady || fallback.metadataReady,
  };
}

export function mergeLocalSubjectDetail(local: Subject, detail: Subject): Subject {
  return {
    ...mergeMissingDisplayMetadata(local, detail),
    episodesDetail: local.episodesDetail.length ? local.episodesDetail : detail.episodesDetail,
    metadataReady: local.metadataReady || detail.metadataReady,
  };
}

export function mergeCloudSubjectDetail(cached: Subject, detail: Subject): Subject {
  return {
    ...detail,
    id: cached.id,
    source: cached.source,
    local: false,
    title: detail.title || cached.title,
    titleCn: detail.titleCn || cached.titleCn,
    summary: hasUsableSummary(detail.summary) ? detail.summary : cached.summary,
    poster: detail.poster || cached.poster,
    hero: detail.hero || cached.hero || detail.poster || cached.poster,
    bgmCollectionType: cached.bgmCollectionType,
    bgmCollectionLabel: cached.bgmCollectionLabel,
    bgmRate: cached.bgmRate,
    bgmPending: cached.bgmPending,
    watchedEpisodes: cached.watchedEpisodes,
    currentEpisode: cached.currentEpisode,
    progress: cached.progress,
    files: cached.files,
    totalSize: cached.totalSize,
    fileSummary: cached.fileSummary,
    localFiles: cached.localFiles,
    episodesDetail: detail.episodesDetail.length ? detail.episodesDetail.map((episode) => {
      const cachedEpisode = cached.episodesDetail.find((item) => (
        (episode.bgmEpisodeId && item.bgmEpisodeId === episode.bgmEpisodeId)
        || item.episode === episode.episode
      ));
      return cachedEpisode ? {
        ...episode,
        bgmCollectionType: cachedEpisode.bgmCollectionType,
        bgmCollectionLabel: cachedEpisode.bgmCollectionLabel,
        bgmPending: cachedEpisode.bgmPending,
      } : episode;
    }) : cached.episodesDetail,
  };
}

async function findSubjectMetadataFallback(subject: Subject) {
  const queries = uniqueNonEmpty([
    subject.titleCn,
    subject.title,
    ...subject.aliases,
  ]);

  for (const query of queries) {
    try {
      const response = await searchCatalog(query, 12);
      const candidate = response.subjects.find((item) => (
        hasUsableSummary(item.summary)
        && isLikelySameSubject(subject, item)
      ));
      if (candidate) {
        return candidate;
      }
    } catch {
      // A fallback source should not block opening the already-loaded detail.
    }
  }
  return null;
}

async function loadBangumiPublicSubject(subject: Subject) {
  if (subject.provider !== "bangumi" || !subject.providerSubjectId.trim()) {
    return null;
  }

  try {
    const response = await fetch(`https://api.bgm.tv/v0/subjects/${encodeURIComponent(subject.providerSubjectId)}`, {
      headers: {
        Accept: "application/json",
      },
    });
    if (!response.ok) {
      return null;
    }
    const data = await response.json() as BangumiPublicSubject;
    const images = data.images ?? {};
    const airDate = typeof data.date === "string" ? data.date : "";
    return {
      ...subject,
      title: nonEmpty(data.name) || subject.title,
      titleCn: nonEmpty(data.name_cn) || subject.titleCn,
      summary: nonEmpty(data.summary) || subject.summary,
      airDate: airDate || subject.airDate,
      year: yearFromDate(airDate) || subject.year,
      rating: finiteNumber(data.rating?.score) || subject.rating,
      rank: finiteNumber(data.rank) || finiteNumber(data.rating?.rank) || subject.rank,
      poster: nonEmpty(images.large) || nonEmpty(images.common) || nonEmpty(images.medium) || subject.poster,
      hero: nonEmpty(images.common) || nonEmpty(images.large) || nonEmpty(images.medium) || subject.hero,
      tags: Array.isArray(data.tags)
        ? data.tags.map((tag) => nonEmpty(tag.name)).filter((tag): tag is string => Boolean(tag))
        : subject.tags,
      aliases: mergeAliases(subject.aliases, aliasesFromInfobox(data.infobox)),
      episodes: finiteNumber(data.total_episodes) || finiteNumber(data.eps) || subject.episodes,
      metadataReady: true,
    } satisfies Subject;
  } catch {
    return null;
  }
}

function hasCompleteDisplayMetadata(subject: Subject) {
  return hasUsableSummary(subject.summary)
    && subject.poster.trim().length > 0
    && subject.hero.trim().length > 0
    && subject.tags.length > 0;
}

function uniqueNonEmpty(values: string[]) {
  const seen = new Set<string>();
  const result: string[] = [];
  for (const value of values) {
    const trimmed = value.trim();
    const key = normalizeSubjectText(trimmed);
    if (!trimmed || !key || seen.has(key)) continue;
    seen.add(key);
    result.push(trimmed);
  }
  return result;
}

function isLikelySameSubject(left: Subject, right: Subject) {
  if (
    left.provider === right.provider
    && left.providerSubjectId
    && left.providerSubjectId === right.providerSubjectId
  ) {
    return true;
  }

  const leftKeys = subjectIdentityKeys(left);
  const rightKeys = subjectIdentityKeys(right);
  return leftKeys.some((key) => rightKeys.includes(key));
}

function subjectIdentityKeys(subject: Subject) {
  return uniqueNonEmpty([
    subject.title,
    subject.titleCn,
    ...subject.aliases,
  ]).map(normalizeSubjectText);
}

function normalizeSubjectText(value: string) {
  return value
    .normalize("NFKC")
    .toLowerCase()
    .replace(/[\s"'`'.,:;!?()[\]{}<>《》「」『』【】ー・·_\-–—]+/g, "");
}

function nonEmpty(value: unknown) {
  return typeof value === "string" && value.trim() ? value.trim() : "";
}

function finiteNumber(value: unknown) {
  return typeof value === "number" && Number.isFinite(value) ? value : 0;
}

function yearFromDate(value: string) {
  const year = value.slice(0, 4);
  const parsed = Number(year);
  return Number.isFinite(parsed) ? parsed : 0;
}

function mergeAliases(primary: string[], secondary: string[]) {
  return uniqueNonEmpty([...primary, ...secondary]);
}

function aliasesFromInfobox(infobox: BangumiPublicSubject["infobox"]) {
  if (!Array.isArray(infobox)) return [];
  const aliases: string[] = [];
  for (const item of infobox) {
    if (item?.key !== "别名") continue;
    if (typeof item.value === "string") {
      aliases.push(item.value);
    } else if (Array.isArray(item.value)) {
      for (const alias of item.value) {
        if (typeof alias?.v === "string") aliases.push(alias.v);
      }
    }
  }
  return aliases;
}

type BangumiPublicSubject = {
  date?: string;
  name?: string;
  name_cn?: string;
  summary?: string;
  images?: {
    large?: string;
    common?: string;
    medium?: string;
  };
  tags?: Array<{ name?: string }>;
  rating?: {
    score?: number;
    rank?: number;
  };
  rank?: number;
  eps?: number;
  total_episodes?: number;
  infobox?: Array<{
    key?: string;
    value?: string | Array<{ v?: string }>;
  }>;
};
