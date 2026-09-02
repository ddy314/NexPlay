import { useEffect, useMemo, useState, type CSSProperties } from "react";
import { ArrowRight, Clock3, Compass, HardDrive, Play, Sparkles } from "lucide-react";
import { motion } from "framer-motion";
import { fetchHomeFeed, type HomeFeed } from "../backend";
import { subjectDisplayTitle, type Subject } from "../data";
import { Poster } from "../MediaCard";
import { appleEase } from "../motion";
import { resolveAssetUrl } from "../utils/assets";
import { cn } from "../utils/cn";

const HOME_FEED_TTL_MS = 5 * 60_000;
let homeFeedCache: { feed: HomeFeed; fetchedAt: number } | null = null;
let homeFeedRequest: Promise<HomeFeed> | null = null;

function requestHomeFeed() {
  if (homeFeedRequest) return homeFeedRequest;
  const request = fetchHomeFeed().finally(() => {
    if (homeFeedRequest === request) homeFeedRequest = null;
  });
  homeFeedRequest = request;
  return request;
}

export function HomePage({
  subjects,
  onOpen,
  onNavigate,
}: {
  subjects: Subject[];
  onOpen: (subject: Subject) => void;
  onNavigate: (route: "discover" | "library" | "insights") => void;
}) {
  const [feed, setFeed] = useState<HomeFeed | null>(() => homeFeedCache?.feed ?? null);
  const [loading, setLoading] = useState(() => !homeFeedCache);

  useEffect(() => {
    const cachedAtMount = homeFeedCache;
    if (cachedAtMount && Date.now() - cachedAtMount.fetchedAt < HOME_FEED_TTL_MS) {
      setLoading(false);
      return;
    }
    let cancelled = false;
    const hasStableFeed = Boolean(cachedAtMount?.feed);
    if (!hasStableFeed) setLoading(true);
    requestHomeFeed()
      .then((next) => {
        homeFeedCache = { feed: next, fetchedAt: Date.now() };
        // A stale feed stays visible for this mount. The refreshed ordering is used
        // on the next navigation, so cards never swap identities under the cursor.
        if (!cancelled && !hasStableFeed) setFeed(next);
      })
      .catch(() => {
        // The local fallback is revealed below only when no stable feed exists.
      })
      .finally(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, []);

  const fallback = useMemo<HomeFeed>(() => ({
    generatedAt: Date.now(),
    sections: subjects.length ? [{
      id: "local",
      kind: "stable",
      title: "你的媒体库",
      subtitle: "本地可播放内容",
      layout: "poster",
      items: subjects.slice(0, 12).map((subject) => ({ subject, reason: "本地可播放" })),
    }] : [],
  }), [subjects]);
  const resolvedFeed = useMemo(() => {
    const source = feed?.sections.length ? feed : loading ? null : fallback;
    if (!source) return null;
    const currentSubjects = new Map(subjects.map((subject) => [subject.canonicalKey, subject]));
    return {
      ...source,
      sections: source.sections.map((section) => ({
        ...section,
        items: section.items.map((item) => ({
          ...item,
          subject: currentSubjects.get(item.subject.canonicalKey) ?? item.subject,
        })),
      })),
    };
  }, [fallback, feed, loading, subjects]);
  const continueSection = resolvedFeed?.sections.find((section) => section.id === "continue");
  const hero = continueSection?.items[0] ?? resolvedFeed?.sections.flatMap((section) => section.items)[0];
  const remaining = resolvedFeed?.sections.filter((section) => section.id !== "continue") ?? [];

  return (
    <div className="h-full overflow-y-auto overflow-x-hidden">
      <motion.div className="nx-page" initial={{ opacity: 0 }} animate={{ opacity: 1 }} transition={appleEase}>
        <header className="nx-page-header">
          <div>
            <div className="nx-eyebrow"><Sparkles size={14} /> Personal station</div>
            <h1 className="nx-page-title">现在，想看点什么？</h1>
            <p className="nx-page-subtitle">继续上次进度，或者从真正理解你偏好的分区里找到下一部。</p>
          </div>
          <button type="button" className="nx-button secondary" onClick={() => onNavigate("discover")}>
            <Compass size={17} /> 打开发现
          </button>
        </header>

        {hero ? (
          <section className="nx-mosaic">
            <HeroPlane item={hero} onOpen={() => onOpen(hero.subject)} />
            <ActionPlane
              tone="blue"
              icon={<Compass size={28} />}
              index="A1"
              title="发现下一部"
              copy="跳出已有收藏，查看本季时间带和趋势内容。"
              onClick={() => onNavigate("discover")}
            />
            <ActionPlane
              tone="green"
              icon={<HardDrive size={26} />}
              index="A2"
              title={`${subjects.filter((subject) => subject.local).length} 部本地可播`}
              copy="进入按年份和状态组织的媒体库。"
              onClick={() => onNavigate("library")}
            />
            <ActionPlane
              tone="purple"
              icon={<Clock3 size={26} />}
              index="A3"
              title="本周观看洞察"
              copy="有效时长、完成集数和活跃节奏都在本机统计。"
              onClick={() => onNavigate("insights")}
            />
          </section>
        ) : (
          <div className="nx-empty">
            <div>
              <div className="nx-empty-icon"><HardDrive size={28} /></div>
              <h2>{loading ? "正在组织首页" : "从你的第一部番剧开始"}</h2>
              <p>配置媒体目录并扫描后，NexPlay 会建立继续观看、最近加入和个性化分区。</p>
              <button type="button" className="nx-button" style={{ marginTop: 18 }} onClick={() => onNavigate("library")}>打开媒体库</button>
            </div>
          </div>
        )}

        {remaining.map((section) => (
          <FeedSection key={section.id} section={section} onOpen={onOpen} />
        ))}
      </motion.div>
    </div>
  );
}

function HeroPlane({ item, onOpen }: { item: HomeFeed["sections"][number]["items"][number]; onOpen: () => void }) {
  const subject = item.subject;
  const image = resolveAssetUrl(subject.hero || subject.poster);
  return (
    <button type="button" className="nx-plane nx-plane-dark group col-span-7 row-span-3 min-h-[390px] text-left max-[1100px]:col-span-6" onClick={onOpen}>
      {image && <Poster src={subject.hero || subject.poster} alt="" className="absolute inset-0 size-full opacity-65 transition-transform duration-300 group-hover:scale-[1.015]" loading="eager" fetchPriority="high" />}
      <div className="absolute inset-0 bg-gradient-to-r from-black/85 via-black/50 to-black/10" />
      <div className="relative flex h-full min-h-[390px] max-w-[620px] flex-col justify-end p-8">
        <span className="mb-auto flex w-fit items-center gap-2 rounded-full bg-white/14 px-3 py-1.5 text-[11px] font-semibold text-white/85 backdrop-blur-md">
          <Play size={12} fill="currentColor" /> CONTINUE 01
        </span>
        <div className="text-[12px] font-semibold text-white/68">{item.reason}</div>
        <h2 className="mt-2 text-[34px] font-bold leading-[1.05] tracking-[-0.035em] text-white">{subject.titleCn || subject.title}</h2>
        <p className="mt-3 line-clamp-2 text-[14px] leading-relaxed text-white/68">{subject.summary || subject.fileSummary}</p>
        <div className="mt-6 flex items-center gap-3">
          <span className="flex size-44 h-11 w-auto items-center gap-2 rounded-[14px] bg-white px-4 text-[13px] font-bold text-black">
            <Play size={16} fill="currentColor" /> 继续观看
          </span>
          <span className="text-[12px] font-semibold text-white/62">{Math.round(subject.progress * 100)}%</span>
        </div>
      </div>
    </button>
  );
}

function ActionPlane({ tone, icon, index, title, copy, onClick }: { tone: "blue" | "green" | "purple"; icon: React.ReactNode; index: string; title: string; copy: string; onClick: () => void }) {
  const style = { gridColumn: "span 5" } as CSSProperties;
  return (
    <button type="button" className={cn("nx-plane min-h-[119px] p-5 text-left max-[1100px]:col-span-6", `nx-plane-${tone}`)} style={style} onClick={onClick}>
      <div className="flex items-start justify-between">
        <span className="opacity-90">{icon}</span><span className="text-[10px] font-bold tracking-[.12em] opacity-60">{index}</span>
      </div>
      <div className="mt-7 flex items-end justify-between gap-4">
        <div><h3 className="text-[20px] font-bold leading-tight">{title}</h3><p className="mt-1 text-[11px] leading-relaxed opacity-68">{copy}</p></div>
        <ArrowRight size={18} className="shrink-0 opacity-70" />
      </div>
    </button>
  );
}

function FeedSection({ section, onOpen }: { section: HomeFeed["sections"][number]; onOpen: (subject: Subject) => void }) {
  return (
    <section className="nx-section">
      <div className="nx-section-head">
        <div>
          <h2 className="nx-section-title"><span><Sparkles size={16} /></span>{section.title}</h2>
          <p className="nx-section-copy">{section.subtitle}</p>
        </div>
        <span className="text-[10px] font-bold tracking-[.12em] text-[var(--nx-ink-3)]">{section.kind.toUpperCase()}</span>
      </div>
      <div className="nx-scroll-row">
        {section.items.map((item) => (
          <button key={item.subject.canonicalKey} type="button" className={cn("nx-media-tile", section.layout === "wide" && "wide")} onClick={() => onOpen(item.subject)}>
            <div className="nx-media-art">
              <Poster src={item.subject.poster || item.subject.hero} alt={subjectDisplayTitle(item.subject)} className="size-full" />
              <span className="nx-reason">{item.reason}</span>
            </div>
            <div className="nx-media-title">{subjectDisplayTitle(item.subject)}</div>
            <div className="nx-media-meta">{item.subject.local ? "本地可播" : "Bangumi"}{item.subject.rating > 0 ? ` · ${item.subject.rating.toFixed(1)}` : ""}</div>
          </button>
        ))}
      </div>
    </section>
  );
}
