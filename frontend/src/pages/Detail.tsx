import { useMemo } from "react";
import { type Subject } from "../data";
import { Card, Chip, Progress } from "../ui";
import { Poster } from "../MediaCard";
import { ArrowLeft, CheckIcon, FileIcon, StarIcon } from "../icons";
import { cn } from "../utils/cn";

type EpisodeRowData = {
  key: string;
  episode: number;
  title: string;
  titleCn: string;
  airDate: string;
  cached: boolean;
  mediaId?: number;
  fileName?: string;
  fileSize?: string;
};

export function DetailPage({
  subject,
  onBack,
  onSnack,
}: {
  subject: Subject;
  onBack: () => void;
  onSnack: (text: string, tone?: "neutral" | "success" | "danger") => void;
}) {
  const rows = useMemo(() => makeEpisodeRows(subject), [subject]);
  const cachedCount = rows.filter((row) => row.cached).length;
  const heroAsset = subject.hero || subject.poster;
  const heroSrc = heroAsset ? window.nexplay?.resolveAssetUrl(heroAsset) ?? heroAsset : "";
  const isUnmatched = subject.status === "unmatched" || subject.status === "failed";
  const visibleTags = subject.tags.slice(0, 6);

  const openEpisode = async (row: EpisodeRowData) => {
    if (!row.mediaId) {
      onSnack(`第 ${row.episode} 集没有对应的本地文件`, "danger");
      return;
    }
    if (!window.nexplay) {
      onSnack("当前不是 Electron 环境，无法打开本地文件", "danger");
      return;
    }

    try {
      await window.nexplay.openMedia(row.mediaId);
      onSnack(`已打开第 ${row.episode} 集`, "success");
    } catch (caught) {
      const message = caught instanceof Error ? caught.message : String(caught);
      onSnack(`打开失败：${message}`, "danger");
    }
  };

  return (
    <div className="relative pb-20">
      <div className="relative h-[420px] w-full overflow-hidden">
        {heroSrc ? (
          <img src={heroSrc} alt={subject.title} className="absolute inset-0 size-full object-cover" />
        ) : (
          <div className="absolute inset-0 bg-gradient-to-br from-[var(--color-surface-3)] via-[var(--color-surface-2)] to-[var(--color-surface)]" />
        )}
        <div className="absolute inset-0 bg-gradient-to-t from-[var(--color-bg)] via-[var(--color-bg)]/75 to-transparent" />
        <div className="absolute inset-0 bg-gradient-to-r from-[var(--color-bg)]/75 via-[var(--color-bg)]/20 to-[var(--color-bg)]/20" />

        <div className="absolute top-0 inset-x-0 flex items-center px-8 py-5">
          <button
            onClick={onBack}
            className="inline-flex items-center gap-2 h-9 px-3 rounded-full bg-black/40 backdrop-blur-md text-white text-[13px] hover:bg-black/60 transition-colors"
          >
            <ArrowLeft className="size-4" />
            返回媒体库
          </button>
        </div>
      </div>

      <div className="px-10 -mt-40 relative grid grid-cols-[240px_1fr] gap-10 items-start">
        <div className="relative">
          <div className="aspect-[2/3] rounded-2xl overflow-hidden ring-1 ring-black/40 shadow-2xl shadow-black/60">
            <Poster src={subject.poster} alt={subject.title} className="size-full" />
          </div>
          {subject.rating > 0 && (
            <Card className="mt-4 p-4">
              <div className="flex items-center gap-1.5 text-amber-300">
                <StarIcon className="size-4" />
                <span className="text-[22px] font-semibold tabular-nums">{subject.rating.toFixed(1)}</span>
                {subject.rank > 0 && (
                  <span className="text-[12px] text-[var(--color-on-surface-faint)] ml-auto">
                    #{subject.rank}
                  </span>
                )}
              </div>
              <div className="text-[11px] text-[var(--color-on-surface-faint)] mt-1">Bangumi 评分</div>
            </Card>
          )}
        </div>

        <div className="pt-14 min-w-0">
          <h1 className="text-[44px] font-semibold tracking-tight leading-[1.05]">
            {isUnmatched ? subject.fileSummary.split(".")[0] : subject.title}
          </h1>
          {!isUnmatched && (
            <div className="text-[18px] text-[var(--color-on-surface-muted)] mt-1 font-light">
              {subject.titleCn} · {subject.year || "年份未知"} · {subject.airDate || "日期未知"}
            </div>
          )}

          {!isUnmatched && visibleTags.length > 0 && (
            <div className="mt-5 flex flex-wrap gap-2">
              {visibleTags.map((tag) => (
                <Chip key={tag}>{tag}</Chip>
              ))}
            </div>
          )}

          {!isUnmatched && subject.summary && (
            <p className="text-[15px] leading-relaxed text-[var(--color-on-surface-muted)] mt-6 max-w-3xl line-clamp-5">
              {subject.summary}
            </p>
          )}

          <div className="mt-6 flex flex-wrap items-center gap-x-5 gap-y-2 text-[13px] text-[var(--color-on-surface-muted)]">
            <span className="tabular-nums">{subject.files} 个本地文件</span>
            <span className="size-1 rounded-full bg-[var(--color-on-surface-faint)]" />
            <span className="tabular-nums">{subject.totalSize}</span>
            <span className="size-1 rounded-full bg-[var(--color-on-surface-faint)]" />
            <span className="tabular-nums">{cachedCount}/{rows.length || subject.episodes || "?"} 集已缓存</span>
          </div>

          {subject.progress > 0 && subject.progress < 1 && (
            <div className="mt-6 max-w-lg">
              <div className="flex items-center justify-between text-[12px] text-[var(--color-on-surface-faint)] mb-1.5">
                <span>{subject.watchedEpisodes}/{subject.episodes || "?"} 集</span>
                <span className="tabular-nums">{Math.round(subject.progress * 100)}%</span>
              </div>
              <Progress value={subject.progress} />
            </div>
          )}
        </div>
      </div>

      <section className="px-10 mt-14">
        <div className="flex items-end justify-between mb-4">
          <div>
            <h2 className="text-[22px] font-semibold tracking-tight">分集与本地缓存</h2>
            <div className="text-[13px] text-[var(--color-on-surface-faint)] mt-1">
              {subject.files} 个本地文件 · {subject.totalSize}
            </div>
          </div>
        </div>

        <Card className="overflow-hidden">
          <div className="grid grid-cols-[88px_1fr_120px_160px] gap-4 px-5 py-3 text-[11px] uppercase tracking-wider text-[var(--color-on-surface-faint)] border-b border-[var(--color-outline-soft)] bg-[var(--color-surface)]">
            <div>EP</div>
            <div>Title</div>
            <div>Cache</div>
            <div className="text-right">File</div>
          </div>
          <div className="divide-y divide-[var(--color-outline-soft)]">
            {rows.map((row) => (
              <EpisodeCacheRow key={row.key} row={row} onOpen={() => void openEpisode(row)} />
            ))}
          </div>
        </Card>
      </section>
    </div>
  );
}

function EpisodeCacheRow({ row, onOpen }: { row: EpisodeRowData; onOpen: () => void }) {
  const title = row.titleCn || row.title || `Episode ${row.episode}`;

  return (
    <button
      type="button"
      onClick={onOpen}
      disabled={!row.cached}
      className={cn(
        "w-full text-left grid grid-cols-[88px_1fr_120px_160px] gap-4 px-5 py-4 items-center text-[13px] transition-colors",
        row.cached
          ? "hover:bg-[var(--color-surface-2)] cursor-pointer"
          : "cursor-default"
      )}
    >
      <div className="tabular-nums text-[var(--color-on-surface-muted)]">
        EP{String(row.episode).padStart(2, "0")}
      </div>
      <div className="min-w-0">
        <div className="text-[15px] font-medium truncate">{title}</div>
        <div className="text-[12px] text-[var(--color-on-surface-faint)] mt-0.5 truncate">
          {row.titleCn && row.title && row.titleCn !== row.title ? row.title : row.airDate || "暂无播出日期"}
        </div>
      </div>
      <div>
        {row.cached ? (
          <span className="inline-flex items-center gap-1.5 h-7 px-2.5 rounded-full bg-emerald-500/15 text-emerald-300 ring-1 ring-inset ring-emerald-400/30">
            <CheckIcon className="size-3.5" />
            本地
          </span>
        ) : (
          <span className="inline-flex items-center gap-1.5 h-7 px-2.5 rounded-full bg-white/5 text-[var(--color-on-surface-faint)] ring-1 ring-inset ring-[var(--color-outline-soft)]">
            <FileIcon className="size-3.5" />
            缺失
          </span>
        )}
      </div>
      <div className="min-w-0 text-right">
        <div className="truncate font-mono text-[12px] text-[var(--color-on-surface-muted)]">
          {row.fileName || "-"}
        </div>
        {row.fileSize && (
          <div className="text-[11px] text-[var(--color-on-surface-faint)] mt-0.5 tabular-nums">
            {row.fileSize}
          </div>
        )}
      </div>
    </button>
  );
}

function makeEpisodeRows(subject: Subject): EpisodeRowData[] {
  if (subject.episodesDetail?.length) {
    return subject.episodesDetail.map((episode) => ({
      key: String(episode.mediaId || `episode-${episode.episode}`),
      episode: episode.episode,
      title: episode.title,
      titleCn: episode.titleCn,
      airDate: episode.airDate,
      cached: episode.cached,
      mediaId: episode.mediaId,
      fileName: episode.fileName,
      fileSize: episode.fileSize,
    }));
  }

  if (subject.localFiles?.length) {
    return subject.localFiles.map((file, index) => ({
      key: String(file.mediaId || `${file.fileName}-${index}`),
      episode: file.episode || index + 1,
      title: `Episode ${file.episode || index + 1}`,
      titleCn: "",
      airDate: "",
      cached: true,
      mediaId: file.mediaId,
      fileName: file.fileName,
      fileSize: file.fileSize,
    }));
  }

  return Array.from({ length: subject.episodes || subject.files }, (_, index) => ({
    key: `${subject.id}-${index}`,
    episode: index + 1,
    title: `Episode ${index + 1}`,
    titleCn: "",
    airDate: "",
    cached: false,
  }));
}
