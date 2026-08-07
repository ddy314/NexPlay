const assert = require("node:assert/strict");

const { chooseMatchingSubtitle } = require("../electron/subtitle-matcher.cjs");

const cases = [
  {
    video: "[SubsPlease] Frieren - 02 (1080p).mkv",
    entries: ["Frieren - 01.zh-CN.ass", "Frieren - 02.zh-CN.ass", "Frieren - 03.zh-CN.ass"],
    expected: "Frieren - 02.zh-CN.ass",
  },
  {
    video: "Show.S01E07.1080p.WEB-DL.mkv",
    entries: ["Show.S01E06.srt", "Show.S01E07.zh-Hans.srt", "Other.S01E07.srt"],
    expected: "Show.S01E07.zh-Hans.srt",
  },
  {
    video: "葬送的芙莉莲 第12话.mp4",
    entries: ["葬送的芙莉莲 第11话.ass", "葬送的芙莉莲 第12话.ass"],
    expected: "葬送的芙莉莲 第12话.ass",
  },
  {
    video: "Movie Name.mkv",
    entries: ["Movie Name.en.srt", "Movie Name.chs.ass", "unrelated.ass"],
    expected: "Movie Name.chs.ass",
  },
  {
    video: "Series - 08.mkv",
    entries: ["Series - 08.sub", "Series - 08.idx"],
    expected: "Series - 08.idx",
  },
  {
    video: "Series - 08.mkv",
    entries: ["Completely Different.ass"],
    expected: null,
  },
];

for (const testCase of cases) {
  assert.equal(
    chooseMatchingSubtitle(testCase.video, testCase.entries),
    testCase.expected,
    testCase.video,
  );
}

console.log(`subtitle matcher: ${cases.length} cases passed`);
