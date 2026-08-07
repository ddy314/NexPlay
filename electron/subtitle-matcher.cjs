const fs = require("node:fs");
const path = require("node:path");

const SUBTITLE_EXTENSIONS = new Set([".ass", ".ssa", ".srt", ".vtt", ".sub", ".idx", ".sup"]);
const LANGUAGE_TOKENS = new Set([
  "zh", "zho", "chi", "cn", "chs", "cht", "sc", "tc", "gb", "big5",
  "zhcn", "zhtw", "hans", "hant", "简", "繁", "简体", "繁体", "中文",
  "en", "eng", "english", "jp", "jpn", "ja", "日文", "日语",
]);
const NOISE_TOKENS = new Set([
  ...LANGUAGE_TOKENS,
  "subtitle", "subtitles", "sub", "subs", "forced", "default", "sign", "signs", "dialogue",
  "web", "webrip", "webdl", "bluray", "bdrip", "brrip", "dvdrip", "remux",
  "x264", "x265", "h264", "h265", "hevc", "avc", "av1", "10bit", "8bit", "hdr", "sdr",
  "aac", "flac", "opus", "ddp", "atmos", "dual", "audio",
  "480p", "576p", "720p", "1080p", "1440p", "2160p", "4k",
]);

function stemOf(fileName) {
  return path.basename(fileName, path.extname(fileName));
}

function normalize(value) {
  return value.normalize("NFKC").toLocaleLowerCase("en-US");
}

function tokensOf(value) {
  return normalize(value).match(/[\p{L}\p{N}]+/gu) || [];
}

function meaningfulTokens(value) {
  return tokensOf(value).filter((token) => !NOISE_TOKENS.has(token) && !/^\d{3,4}p$/.test(token));
}

function episodeNumber(value) {
  const normalized = normalize(value);
  const explicitPatterns = [
    /(?:^|[^\p{L}\p{N}])s\s*0*\d{1,2}\s*e(?:p)?\s*0*(\d{1,4})(?=$|[^\p{L}\p{N}])/iu,
    /(?:^|[^\p{L}\p{N}])(?:ep?|episode)\s*[._ -]*0*(\d{1,4})(?=$|[^\p{L}\p{N}])/iu,
    /第\s*0*(\d{1,4})\s*[话話集]/u,
  ];
  for (const pattern of explicitPatterns) {
    const match = normalized.match(pattern);
    if (match) return Number(match[1]);
  }

  const candidates = [...normalized.matchAll(/(?:^|[\s._\-[\]()])0*(\d{1,3})(?=$|[\s._\-[\]()])/gu)]
    .map((match) => Number(match[1]))
    .filter((number) => number > 0 && number < 400);
  return candidates.length ? candidates.at(-1) : null;
}

function languagePreference(fileName) {
  const tokens = new Set(tokensOf(stemOf(fileName)));
  if (["chs", "sc", "zhcn", "hans", "简", "简体"].some((token) => tokens.has(token))) return 5;
  if (["zh", "zho", "chi", "cn", "中文"].some((token) => tokens.has(token))) return 4;
  if (["cht", "tc", "zhtw", "hant", "繁", "繁体"].some((token) => tokens.has(token))) return 3;
  if (["en", "eng", "english"].some((token) => tokens.has(token))) return 1;
  return 2;
}

function similarity(left, right) {
  const leftSet = new Set(meaningfulTokens(left).filter((token) => !/^\d+$/.test(token)));
  const rightSet = new Set(meaningfulTokens(right).filter((token) => !/^\d+$/.test(token)));
  if (!leftSet.size || !rightSet.size) return 0;
  let intersection = 0;
  for (const token of leftSet) {
    if (rightSet.has(token)) intersection += 1;
  }
  return intersection / Math.max(leftSet.size, rightSet.size);
}

function scoreSubtitle(videoFileName, subtitleFileName) {
  const videoStem = normalize(stemOf(videoFileName));
  const subtitleStem = normalize(stemOf(subtitleFileName));
  const videoEpisode = episodeNumber(videoStem);
  const subtitleEpisode = episodeNumber(subtitleStem);

  if (videoEpisode !== null && subtitleEpisode !== null && videoEpisode !== subtitleEpisode) {
    return Number.NEGATIVE_INFINITY;
  }

  let score = 0;
  if (videoStem === subtitleStem) {
    score += 220;
  } else if (
    subtitleStem.startsWith(`${videoStem}.`)
    || subtitleStem.startsWith(`${videoStem} `)
    || subtitleStem.startsWith(`${videoStem}-`)
    || subtitleStem.startsWith(`${videoStem}_`)
  ) {
    score += 180;
  }

  const nameSimilarity = similarity(videoStem, subtitleStem);
  score += Math.round(nameSimilarity * 100);
  if (videoEpisode !== null && subtitleEpisode === videoEpisode) score += 70;
  score += languagePreference(subtitleFileName);
  return score;
}

function chooseMatchingSubtitle(videoFileName, entries) {
  const candidates = entries
    .filter((entry) => SUBTITLE_EXTENSIONS.has(path.extname(entry).toLocaleLowerCase("en-US")))
    .filter((entry) => {
      if (path.extname(entry).toLocaleLowerCase("en-US") !== ".sub") return true;
      const idxName = `${stemOf(entry)}.idx`;
      return !entries.some((candidate) => candidate.toLocaleLowerCase("en-US") === idxName.toLocaleLowerCase("en-US"));
    })
    .map((fileName) => ({ fileName, score: scoreSubtitle(videoFileName, fileName) }))
    .filter((candidate) => candidate.score >= 65)
    .sort((left, right) => right.score - left.score || left.fileName.localeCompare(right.fileName));
  return candidates[0]?.fileName || null;
}

async function findMatchingSubtitle(mediaPath) {
  if (typeof mediaPath !== "string" || !mediaPath) return null;
  const directory = path.dirname(mediaPath);
  let entries;
  try {
    entries = await fs.promises.readdir(directory, { withFileTypes: true });
  } catch {
    return null;
  }
  const fileNames = entries.filter((entry) => entry.isFile()).map((entry) => entry.name);
  const match = chooseMatchingSubtitle(path.basename(mediaPath), fileNames);
  return match ? path.join(directory, match) : null;
}

module.exports = {
  chooseMatchingSubtitle,
  findMatchingSubtitle,
  scoreSubtitle,
};
