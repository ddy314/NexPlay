import { useDeferredValue, useMemo, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { RefreshCw, Search, Sparkles } from "lucide-react";
import type { BackendLogEntry, ScanStatus } from "../backend";
import type { Subject } from "../data";
import type { Route } from "../NavRail";
import { MediaCard } from "../MediaCard";
import { useIncrementalItems } from "../hooks/useIncrementalItems";
import { appleSpringBouncy, appleSpringSoft } from "../motion";
import { Button } from "../ui";
import { cn } from "../utils/cn";

type CatalogRoute = Exclude<Route, "settings">;

const pageCopy: Record<CatalogRoute, { title: string; subtitle?: string }> = {
  search: {
    title: "搜索",
    subtitle: "查找标题、文件名或标签",
  },
  home: {
    title: "主页",
  },
  library: {
    title: "媒体库",
    subtitle: "你的本地番剧收藏",
  },
};

export function LibraryPage({
  route,
  subjects,
  scanStatus,
  logs,
  loading,
  error,
  onOpen,
  onSnack,
  onScan,
}: {
  route: CatalogRoute;
  subjects: Subject[];
  scanStatus: ScanStatus;
  logs: BackendLogEntry[];
  loading?: boolean;
  error?: string | null;
  onOpen: (s: Subject) => void;
  onSnack: (text: string, tone?: "neutral" | "success" | "danger") => void;
  onScan: () => void | Promise<void>;
}) {
  const [query, setQuery] = useState("");
  const [heroIndex, setHeroIndex] = useState(0);
  const deferredQuery = useDeferredValue(query);

  const watching = useMemo(
    () => subjects.filter((subject) => subject.progress > 0 && subject.progress < 1),
    [subjects]
  );
  const completed = useMemo(
    () => subjects.filter((subject) => subject.progress >= 1),
    [subjects]
  );
  const recent = useMemo(() => subjects.slice(0, 18), [subjects]);
  const heroSubject = watching[heroIndex % Math.max(1, watching.length)] ?? subjects[0];

  const searchResults = useMemo(() => {
    const q = deferredQuery.trim().toLowerCase();
    if (!q) return [];
    return subjects.filter(
      (subject) =>
        subject.title.toLowerCase().includes(q) ||
        subject.titleCn.toLowerCase().includes(q) ||
        subject.fileSummary.toLowerCase().includes(q) ||
        subject.tags.some((tag) => tag.toLowerCase().includes(q))
    );
  }, [deferredQuery, subjects]);

  const displayItems = route === "search" ? searchResults : subjects;
  const {
    hasMore,
    loadMore,
    sentinelRef,
    visibleCount,
    visibleItems,
  } = useIncrementalItems(displayItems, {
    initialCount: route === "library" ? 72 : 36,
    step: 36,
    resetKey: `${route}:${deferredQuery}:${displayItems.length}`,
  });
  const remainingCount = Math.max(0, displayItems.length - visibleCount);
  const copy = pageCopy[route];

  return (
    <div className="h-full overflow-y-auto overflow-x-hidden">
      <AnimatePresence mode="wait">
        <motion.div
          key={route}
          className="page-shell"
          initial={{ opacity: 0, y: 8 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: -8 }}
          transition={appleSpringSoft}
        >
          {route === "search" && (
            <SearchHeader query={query} setQuery={setQuery} />
          )}

          {route !== "search" && (
            <PageHeader
              title={copy.title}
              subtitle={copy.subtitle}
              action={
                route === "library" ? (
                  <Button
                    icon={<RefreshCw size={16} className={scanStatus.running ? "animate-spin" : ""} />}
                    loading={scanStatus.running}
                    onClick={() => {
                      onSnack("已开始扫描媒体目录...");
                      void onScan();
                    }}
                    className="h-10 px-4 text-[13px]"
                  >
                    {scanStatus.running ? "扫描中" : "扫描"}
                  </Button>
                ) : null
              }
            />
          )}

          {route === "search" ? (
            <SearchContent
              query={deferredQuery}
              results={visibleItems}
              total={searchResults.length}
              hasMore={hasMore}
              remainingCount={remainingCount}
              sentinelRef={sentinelRef}
              onLoadMore={loadMore}
              onOpen={onOpen}
            />
          ) : !subjects.length ? (
            <EmptyState
              title={loading ? "正在读取媒体库" : "这里还没有番剧"}
              desc={error ?? "请先在设置页确认媒体目录，然后到媒体库执行扫描。"}
            />
          ) : route === "home" ? (
            <>
              {heroSubject && (
                <HeroBanner
                  subject={heroSubject}
                  onOpen={() => onOpen(heroSubject)}
                  current={heroIndex}
                  total={watching.length || 1}
                  onNext={() => setHeroIndex((index) => (index + 1) % Math.max(1, watching.length))}
                />
              )}
              {watching.length > 0 && (
                <Section title="继续观看">
                  <CardGrid subjects={watching} onOpen={onOpen} />
                </Section>
              )}
              <Section title="最近添加">
                <CardGrid subjects={recent} onOpen={onOpen} offset={watching.length} />
              </Section>
            </>
          ) : (
            <>
              {(scanStatus.running || logs.length > 0) && (
                <div className="mt-5">
                  <ScanPanel status={scanStatus} logs={logs} />
                </div>
              )}
              {completed.length > 0 && (
                <Section title="已完成">
                  <CardGrid subjects={completed.slice(0, 18)} onOpen={onOpen} />
                </Section>
              )}
              <Section title="全部番剧">
                <CardGrid subjects={visibleItems} onOpen={onOpen} />
                {hasMore && (
                  <LoadMore
                    remainingCount={remainingCount}
                    sentinelRef={sentinelRef}
                    onLoadMore={loadMore}
                  />
                )}
              </Section>
            </>
          )}
        </motion.div>
      </AnimatePresence>
    </div>
  );
}

function SearchHeader({
  query,
  setQuery,
}: {
  query: string;
  setQuery: (value: string) => void;
}) {
  return (
    <div className="search-page-header">
      <label className="search-field flex h-12 min-w-0 flex-1 items-center gap-3 rounded-[var(--radius-pill)] px-4">
        <Search size={22} className="text-[var(--color-text-tertiary)]" strokeWidth={2.1} />
        <input
          autoFocus
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder="搜索标题、文件名、标签..."
          className="min-w-0 flex-1 bg-transparent text-[17px] font-semibold text-[var(--color-text-primary)] outline-none placeholder:text-[var(--color-text-tertiary)]/85"
        />
      </label>
      <div className="search-scope">
        <button type="button" className="active">NexPlay</button>
        <button type="button">资料库</button>
      </div>
    </div>
  );
}

function SearchContent({
  query,
  results,
  total,
  hasMore,
  remainingCount,
  sentinelRef,
  onLoadMore,
  onOpen,
}: {
  query: string;
  results: Subject[];
  total: number;
  hasMore: boolean;
  remainingCount: number;
  sentinelRef: (node: HTMLDivElement | null) => void;
  onLoadMore: () => void;
  onOpen: (subject: Subject) => void;
}) {
  if (!query.trim()) {
    return (
      <EmptyState
        title="搜索你的资料库"
        desc="输入标题、文件名或标签后，结果会显示在这里。"
      />
    );
  }

  return (
    <Section title="搜索结果" subtitle={`${total} 部`}>
      {results.length ? (
        <>
          <CardGrid subjects={results} onOpen={onOpen} />
          {hasMore && (
            <LoadMore
              remainingCount={remainingCount}
              sentinelRef={sentinelRef}
              onLoadMore={onLoadMore}
            />
          )}
        </>
      ) : (
        <EmptyState title="没有匹配的番剧" desc="换一个搜索词再试。" compact />
      )}
    </Section>
  );
}

function PageHeader({
  title,
  subtitle,
  action,
}: {
  title: string;
  subtitle?: string;
  action?: React.ReactNode;
}) {
  return (
    <header className="page-header">
      <div>
        <motion.h1
          className="text-[42px] font-bold leading-[1] tracking-tight text-[var(--color-text-primary)]"
          initial={{ opacity: 0, x: -8 }}
          animate={{ opacity: 1, x: 0 }}
          transition={appleSpringSoft}
        >
          {title}
        </motion.h1>
        {subtitle && (
          <motion.p
            className="mt-2.5 text-[17px] font-medium text-[var(--color-text-secondary)]"
            initial={{ opacity: 0, x: -8 }}
            animate={{ opacity: 1, x: 0 }}
            transition={appleSpringSoft}
          >
            {subtitle}
          </motion.p>
        )}
      </div>
      {action}
    </header>
  );
}

function HeroBanner({
  subject,
  onOpen,
  total,
  current,
  onNext,
}: {
  subject: Subject;
  onOpen: () => void;
  total: number;
  current: number;
  onNext: () => void;
}) {
  const heroAsset = subject.hero || subject.poster;
  const heroSrc = heroAsset ? window.nexplay?.resolveAssetUrl(heroAsset) ?? heroAsset : "";

  return (
    <motion.button
      type="button"
      className="group relative mt-6 h-[300px] w-full cursor-pointer overflow-hidden rounded-[var(--radius-panel)] text-left"
      onClick={onOpen}
      whileHover={{ scale: 1.003 }}
      whileTap={{ scale: 0.985 }}
      transition={appleSpringBouncy}
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      key={subject.id}
    >
      {heroSrc ? (
        <motion.img
          src={heroSrc}
          alt={subject.title}
          className="absolute inset-0 size-full object-cover"
          initial={{ scale: 1.025, opacity: 0 }}
          animate={{ scale: 1, opacity: 1 }}
          transition={appleSpringSoft}
        />
      ) : (
        <div className="absolute inset-0 bg-gradient-to-br from-[#ff2d55] via-[#fb395f] to-[#b20d35]" />
      )}
      <div className="absolute inset-0 bg-gradient-to-r from-black/58 via-black/22 to-transparent" />
      <div className="absolute inset-0 bg-gradient-to-t from-black/48 via-transparent to-black/8" />

      <div className="absolute inset-0 flex flex-col justify-end p-7">
        <motion.div
          className="max-w-xl"
          initial={{ opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={appleSpringSoft}
        >
          <h2 className="text-[28px] font-bold leading-tight tracking-tight text-white">
            {subject.title}
          </h2>
          <p className="mt-2 line-clamp-2 max-w-lg text-[14px] font-medium leading-relaxed text-white/68">
            {subject.summary || subject.titleCn || subject.fileSummary}
          </p>
        </motion.div>
      </div>

      {total > 1 && (
        <div className="absolute bottom-5 right-7 flex items-center gap-1.5">
          {Array.from({ length: Math.min(total, 8) }).map((_, index) => (
            <span
              key={index}
              onClick={(event) => {
                event.stopPropagation();
                onNext();
              }}
              className={cn(
                "rounded-full transition-all duration-300",
                index === current % Math.min(total, 8)
                  ? "h-1.5 w-5 bg-white/90"
                  : "size-1.5 bg-white/30 hover:bg-white/50"
              )}
            />
          ))}
        </div>
      )}
    </motion.button>
  );
}

function Section({
  title,
  subtitle,
  children,
}: {
  title: string;
  subtitle?: string;
  children: React.ReactNode;
}) {
  return (
    <section className="mt-9">
      <div className="mb-5 flex items-end gap-2">
        <h2 className="text-[25px] font-bold tracking-tight text-[var(--color-text-primary)]">{title}</h2>
        {subtitle && <span className="pb-0.5 text-[13px] font-semibold text-[var(--color-text-tertiary)]">{subtitle}</span>}
      </div>
      {children}
    </section>
  );
}

function CardGrid({
  subjects,
  onOpen,
  offset = 0,
}: {
  subjects: Subject[];
  onOpen: (subject: Subject) => void;
  offset?: number;
}) {
  return (
    <div className="grid grid-cols-[repeat(auto-fill,minmax(178px,1fr))] gap-x-7 gap-y-8 pb-2">
      {subjects.map((subject, index) => (
        <MediaCard
          key={subject.id}
          subject={subject}
          index={index + offset}
          onClick={() => onOpen(subject)}
          imageLoading={index < 8 ? "eager" : "lazy"}
          imageFetchPriority={index < 8 ? "high" : "auto"}
        />
      ))}
    </div>
  );
}

function LoadMore({
  remainingCount,
  sentinelRef,
  onLoadMore,
}: {
  remainingCount: number;
  sentinelRef: (node: HTMLDivElement | null) => void;
  onLoadMore: () => void;
}) {
  return (
    <div ref={sentinelRef} className="flex justify-center py-7">
      <button
        type="button"
        onClick={onLoadMore}
        className="glass-light h-10 rounded-[var(--radius-pill)] px-4 text-[13px] font-medium text-[var(--color-text-secondary)] transition-colors hover:text-[var(--color-text-primary)]"
      >
        继续加载剩余 {remainingCount} 项
      </button>
    </div>
  );
}

function ScanPanel({
  status,
  logs,
}: {
  status: ScanStatus;
  logs: BackendLogEntry[];
}) {
  const recentLogs = logs.slice(-4).reverse();
  return (
    <div className="glass-light rounded-[var(--radius-card)] px-4 py-3">
      <div className="flex items-center gap-2 text-[13px] font-semibold text-[var(--color-text-primary)]">
        <RefreshCw size={14} className={status.running ? "animate-spin text-[var(--color-accent)]" : "text-[var(--color-text-tertiary)]"} />
        {status.running ? "正在扫描媒体目录" : "最近扫描日志"}
      </div>
      {recentLogs.length > 0 && (
        <div className="mt-2 grid gap-1">
          {recentLogs.map((entry, index) => (
            <div key={`${entry.id}-${index}`} className="truncate text-[11.5px] text-[var(--color-text-tertiary)]">
              {entry.text}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function EmptyState({
  title,
  desc,
  compact,
}: {
  title: string;
  desc: string;
  compact?: boolean;
}) {
  return (
    <div className={cn("flex flex-col items-center justify-center text-[var(--color-text-tertiary)]", compact ? "py-16" : "min-h-[360px] py-24")}>
      <div className="glass mb-5 flex size-20 items-center justify-center rounded-[var(--radius-card)]">
        <Sparkles size={28} className="text-[var(--color-accent)]" />
      </div>
      <p className="text-[17px] font-bold text-[var(--color-text-secondary)]">{title}</p>
      <p className="mt-1.5 max-w-md text-center text-[13px] font-medium opacity-70">{desc}</p>
    </div>
  );
}
