import { memo, useCallback, useDeferredValue, useMemo, useState } from "react";
import { STATUS_LABEL, STATUS_COLOR, type Subject, type MatchStatus } from "../data";
import type { BackendLogEntry, LibraryStats, ScanStatus } from "../backend";
import { Badge, Button, Card, Progress, SearchField, Segmented } from "../ui";
import { MediaCard, Poster } from "../MediaCard";
import { useIncrementalItems } from "../hooks/useIncrementalItems";
import {
  FolderPlus,
  GridIcon,
  ListIcon,
  ScanIcon,
  SortIcon,
  ChevronDown,
  CheckIcon,
} from "../icons";
import { cn } from "../utils/cn";

export function LibraryPage({
  subjects,
  stats,
  scanStatus,
  logs,
  onOpen,
  onSnack,
  onScan,
}: {
  subjects: Subject[];
  stats: LibraryStats;
  scanStatus: ScanStatus;
  logs: BackendLogEntry[];
  onOpen: (s: Subject) => void;
  onSnack: (text: string, tone?: "neutral" | "success" | "danger") => void;
  onScan: () => void | Promise<void>;
}) {
  const [sort, setSort] = useState<"title" | "date" | "progress" | "match">("date");
  const [view, setView] = useState<"grid" | "list">("grid");
  const [query, setQuery] = useState("");
  const deferredQuery = useDeferredValue(query);

  const filtered = useMemo(() => {
    let list = subjects.slice();
    const normalizedQuery = deferredQuery.trim();
    if (normalizedQuery) {
      const q = normalizedQuery.toLowerCase();
      list = list.filter(
        (s) =>
          s.title.toLowerCase().includes(q) ||
          s.titleCn.toLowerCase().includes(q) ||
          s.fileSummary.toLowerCase().includes(q)
      );
    }
    switch (sort) {
      case "title":
        list.sort((a, b) => a.title.localeCompare(b.title));
        break;
      case "progress":
        list.sort((a, b) => b.progress - a.progress);
        break;
      case "match":
        const order: Record<MatchStatus, number> = { matched: 0, tentative: 1, unmatched: 2, failed: 3 };
        list.sort((a, b) => order[a.status] - order[b.status]);
        break;
    }
    return list;
  }, [sort, deferredQuery, subjects]);

  const {
    hasMore,
    loadMore,
    sentinelRef,
    visibleCount,
    visibleItems,
  } = useIncrementalItems(filtered, {
    initialCount: view === "grid" ? 48 : 80,
    step: view === "grid" ? 36 : 80,
    resetKey: `${view}:${sort}:${deferredQuery}:${subjects.length}`,
  });

  const showEmpty = filtered.length === 0;
  const remainingCount = Math.max(0, filtered.length - visibleCount);

  return (
    <div className="px-10 py-10 pb-20">
      {/* Header */}
      <div className="flex items-end justify-between mb-2">
        <div>
          <h1 className="text-[36px] font-semibold tracking-tight leading-tight">媒体库</h1>
          <div className="text-[14px] text-[var(--color-on-surface-muted)] mt-2">
            <span className="tabular-nums">{stats.total}</span> items ·{" "}
            <span className="text-emerald-300 tabular-nums">{stats.matched}</span> matched ·{" "}
            <span className="text-amber-300 tabular-nums">{stats.unmatched}</span> unmatched
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Button
            icon={<ScanIcon className="size-4" />}
            loading={scanStatus.running}
            onClick={() => {
              onSnack("已开始扫描媒体目录…");
              void onScan();
            }}
          >
            {scanStatus.running ? "Scanning" : "Scan Now"}
          </Button>
        </div>
      </div>

      {/* Command Bar */}
      <Card className="mt-7 p-3 flex flex-wrap items-center gap-3 acrylic">
        <SearchField
          value={query}
          onChange={setQuery}
          placeholder="搜索标题、文件名、标签…"
          className="min-w-[280px] flex-1 max-w-md"
        />
        <div className="ml-auto flex items-center gap-2">
          <SortMenu sort={sort} onChange={setSort} />
          <Segmented
            value={view}
            onChange={setView}
            options={[
              { value: "grid", label: "", icon: <GridIcon className="size-4" /> },
              { value: "list", label: "", icon: <ListIcon className="size-4" /> },
            ]}
          />
        </div>
      </Card>

      {(scanStatus.running || logs.length > 0) && (
        <ScanPanel status={scanStatus} logs={logs} />
      )}

      {/* Body */}
      <div className="mt-8">
        <div>
          {showEmpty ? (
            <EmptyState query={query} onClear={() => setQuery("")} />
          ) : view === "grid" ? (
            <div className="grid grid-cols-[repeat(auto-fill,minmax(180px,1fr))] gap-x-5 gap-y-8">
              {visibleItems.map((s, index) => (
                <MediaGridItem
                  key={s.id}
                  subject={s}
                  onOpen={onOpen}
                  priority={index < 8}
                />
              ))}
              {hasMore && (
                <GridLoadMore
                  remainingCount={remainingCount}
                  sentinelRef={sentinelRef}
                  onLoadMore={loadMore}
                />
              )}
            </div>
          ) : (
            <Card className="overflow-hidden">
              <ListHeader />
              {visibleItems.map((s, i) => (
                <ListRow
                  key={s.id}
                  subject={s}
                  isLast={!hasMore && i === visibleItems.length - 1}
                  onOpenSubject={onOpen}
                />
              ))}
              {hasMore && (
                <ListLoadMore
                  remainingCount={remainingCount}
                  sentinelRef={sentinelRef}
                  onLoadMore={loadMore}
                />
              )}
            </Card>
          )}
        </div>
      </div>
    </div>
  );
}

const MediaGridItem = memo(function MediaGridItem({
  subject,
  onOpen,
  priority,
}: {
  subject: Subject;
  onOpen: (s: Subject) => void;
  priority: boolean;
}) {
  const handleOpen = useCallback(() => onOpen(subject), [onOpen, subject]);

  return (
    <div className="flex justify-center cv-media-card">
      <MediaCard
        subject={subject}
        onClick={handleOpen}
        imageLoading={priority ? "eager" : "lazy"}
        imageFetchPriority={priority ? "high" : "auto"}
      />
    </div>
  );
});

function GridLoadMore({
  remainingCount,
  sentinelRef,
  onLoadMore,
}: {
  remainingCount: number;
  sentinelRef: (node: HTMLDivElement | null) => void;
  onLoadMore: () => void;
}) {
  return (
    <div ref={sentinelRef} className="col-span-full flex justify-center py-2">
      <button
        type="button"
        onClick={onLoadMore}
        className="h-9 rounded-full px-4 text-[12px] text-[var(--color-on-surface-muted)] hover:bg-white/[0.06] hover:text-[var(--color-on-surface)]"
      >
        继续加载剩余 {remainingCount} 项
      </button>
    </div>
  );
}

function ListLoadMore({
  remainingCount,
  sentinelRef,
  onLoadMore,
}: {
  remainingCount: number;
  sentinelRef: (node: HTMLDivElement | null) => void;
  onLoadMore: () => void;
}) {
  return (
    <div
      ref={sentinelRef}
      className="px-5 py-4 text-center bg-[var(--color-surface)]"
    >
      <button
        type="button"
        onClick={onLoadMore}
        className="h-8 rounded-full px-3 text-[12px] text-[var(--color-on-surface-muted)] hover:bg-white/[0.06] hover:text-[var(--color-on-surface)]"
      >
        继续加载剩余 {remainingCount} 项
      </button>
    </div>
  );
}

function SortMenu({
  sort,
  onChange,
}: {
  sort: "title" | "date" | "progress" | "match";
  onChange: (v: any) => void;
}) {
  const [open, setOpen] = useState(false);
  const items = [
    { v: "title", l: "Title" },
    { v: "date", l: "Date Added" },
    { v: "progress", l: "Progress" },
    { v: "match", l: "Match Status" },
  ] as const;
  const current = items.find((i) => i.v === sort)?.l;
  return (
    <div className="relative">
      <button
        onClick={() => setOpen((v) => !v)}
        className="inline-flex items-center gap-2 h-9 px-3 text-[13px] rounded-full bg-[var(--color-surface-2)] ring-1 ring-inset ring-[var(--color-outline-soft)] hover:bg-[var(--color-surface-3)] text-[var(--color-on-surface)]"
      >
        <SortIcon className="size-4 text-[var(--color-on-surface-faint)]" />
        Sort · <span className="text-[var(--color-on-surface)] font-medium">{current}</span>
        <ChevronDown className="size-3.5" />
      </button>
      {open && (
        <>
          <div className="fixed inset-0 z-10" onClick={() => setOpen(false)} />
          <div className="absolute right-0 top-full mt-2 z-20 w-44 rounded-xl bg-[var(--color-surface-3)] ring-1 ring-inset ring-[var(--color-outline)] shadow-xl py-1.5">
            {items.map((it) => (
              <button
                key={it.v}
                onClick={() => {
                  onChange(it.v);
                  setOpen(false);
                }}
                className="w-full flex items-center justify-between px-3 py-2 text-[13px] text-left hover:bg-white/[0.06]"
              >
                {it.l}
                {sort === it.v && <CheckIcon className="size-4 text-[var(--color-primary)]" />}
              </button>
            ))}
          </div>
        </>
      )}
    </div>
  );
}

function ListHeader() {
  return (
    <div className="grid grid-cols-[64px_1fr_140px_120px_140px] gap-4 px-5 py-3 text-[11px] uppercase tracking-wider text-[var(--color-on-surface-faint)] border-b border-[var(--color-outline-soft)] bg-[var(--color-surface)]">
      <div></div>
      <div>Title</div>
      <div>Status</div>
      <div>Progress</div>
      <div>Last Played</div>
    </div>
  );
}

const ListRow = memo(function ListRow({
  subject,
  onOpenSubject,
  isLast,
}: {
  subject: Subject;
  onOpenSubject: (s: Subject) => void;
  isLast: boolean;
}) {
  const handleOpen = useCallback(() => onOpenSubject(subject), [onOpenSubject, subject]);

  return (
    <div
      onClick={handleOpen}
      className={cn(
        "grid grid-cols-[64px_1fr_140px_120px_140px] gap-4 px-5 py-3 items-center cursor-pointer transition-colors hover:bg-white/[0.04] cv-list-row",
        !isLast && "border-b border-[var(--color-outline-soft)]"
      )}
    >
      <div className="aspect-[2/3] w-12 rounded-md overflow-hidden ring-1 ring-black/40">
        <Poster src={subject.poster} alt={subject.title} className="size-full" />
      </div>
      <div className="min-w-0">
        <div className="text-[14px] font-medium truncate flex items-center gap-2">
          {subject.title}
          {subject.newEpisode && <Badge tone="primary">NEW</Badge>}
        </div>
        <div className="text-[12px] text-[var(--color-on-surface-faint)] truncate font-mono">
          {subject.fileSummary}
        </div>
      </div>
      <div>
        <span
          className={cn(
            "inline-flex items-center px-2 h-6 rounded-full text-[11px] font-medium ring-1 ring-inset",
            STATUS_COLOR[subject.status]
          )}
        >
          {STATUS_LABEL[subject.status]}
        </span>
      </div>
      <div className="flex flex-col gap-1 text-[12px]">
        <Progress value={subject.progress} />
        <span className="text-[var(--color-on-surface-faint)] tabular-nums">
          {subject.watchedEpisodes}/{subject.episodes || "?"} ep
        </span>
      </div>
      <div className="text-[12px] text-[var(--color-on-surface-muted)]">
        {subject.lastPlayed ?? "—"}
      </div>
    </div>
  );
});

function EmptyState({ query, onClear }: { query: string; onClear: () => void }) {
  if (query) {
    return (
      <Card className="p-12 grid place-items-center text-center">
        <div className="size-14 rounded-2xl bg-[var(--color-surface-3)] grid place-items-center mb-4">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.6} className="size-6 text-[var(--color-on-surface-muted)]">
            <circle cx="11" cy="11" r="7" />
            <path d="M20 20l-3.5-3.5" />
          </svg>
        </div>
        <div className="text-[16px] font-medium">没有找到 "{query}"</div>
        <div className="text-[13px] text-[var(--color-on-surface-faint)] mt-1">
          换个关键词试试，或者清除搜索查看全部。
        </div>
        <Button variant="tonal" className="mt-5" onClick={onClear}>
          Clear Search
        </Button>
      </Card>
    );
  }
  return (
    <Card className="p-12 grid place-items-center text-center">
      <div className="size-16 rounded-2xl bg-[var(--color-primary-soft)] grid place-items-center mb-4">
        <FolderPlus className="size-7 text-[var(--color-primary)]" />
      </div>
      <div className="text-[18px] font-medium">媒体库为空</div>
      <div className="text-[13px] text-[var(--color-on-surface-faint)] mt-1 max-w-sm">
        当前没有可显示的媒体。检查设置里的媒体目录后点击 Scan Now。
      </div>
    </Card>
  );
}

function ScanPanel({ status, logs }: { status: ScanStatus; logs: BackendLogEntry[] }) {
  const progress =
    status.stage === "done"
      ? 1
      : status.stage === "metadata" && status.total > 0
      ? status.processed / status.total
      : status.scanned > 0
      ? Math.min(0.95, status.indexed / Math.max(status.scanned, 1))
      : 0;

  return (
    <Card className="mt-4 overflow-hidden">
      <div className="px-4 py-3 border-b border-[var(--color-outline-soft)]">
        <div className="flex items-center justify-between gap-4">
          <div className="min-w-0">
            <div className="text-[14px] font-medium truncate">
              {status.message || "扫描状态"}
            </div>
            <div className="text-[12px] text-[var(--color-on-surface-faint)] mt-0.5 tabular-nums">
              文件 {status.indexed}/{status.scanned}
              {status.total > 0 ? ` · 元数据 ${status.processed}/${status.total}` : ""}
            </div>
          </div>
          <div className="text-[12px] text-[var(--color-on-surface-muted)]">
            {status.running ? "运行中" : status.stage === "failed" ? "失败" : "空闲"}
          </div>
        </div>
        <Progress value={progress} className="mt-3" />
      </div>
      {logs.length > 0 && (
        <div className="max-h-52 overflow-y-auto px-4 py-3 space-y-1 bg-[var(--color-surface)]">
          {logs.slice(-80).map((entry) => (
            <div
              key={entry.id}
              className={cn(
                "font-mono text-[11.5px] leading-relaxed",
                entry.tone === "danger"
                  ? "text-[var(--color-danger)]"
                  : "text-[var(--color-on-surface-muted)]"
              )}
            >
              {entry.text}
            </div>
          ))}
        </div>
      )}
    </Card>
  );
}
