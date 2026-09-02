import { useEffect, useMemo, useState } from "react";
import { CalendarDays, Compass, Flame, Map as MapIcon, Sparkles } from "lucide-react";
import { motion } from "framer-motion";
import { subjectDisplayTitle, type Subject } from "../data";
import { fetchBangumiDiscovery, type DiscoveryFeed } from "../discover";
import { Poster } from "../MediaCard";
import { appleEase } from "../motion";

export function ExplorePage({ onOpen }: { onOpen: (subject: Subject) => void }) {
  const [feed, setFeed] = useState<DiscoveryFeed | null>(null);
  const [loading, setLoading] = useState(true);
  useEffect(() => {
    let cancelled = false;
    fetchBangumiDiscovery()
      .then((next) => { if (!cancelled) setFeed(next); })
      .finally(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, []);

  const spotlight = feed?.trending[0] ?? feed?.today[0];
  const themed = useMemo(() => {
    const items = [...(feed?.trending ?? []), ...(feed?.today ?? [])];
    return Array.from(new Map(items.map((item) => [`${item.provider}:${item.providerSubjectId}`, item])).values());
  }, [feed]);

  return (
    <div className="h-full overflow-y-auto overflow-x-hidden">
      <motion.div className="nx-page" initial={{ opacity: 0 }} animate={{ opacity: 1 }} transition={appleEase}>
        <header className="nx-page-header">
          <div>
            <div className="nx-eyebrow"><Compass size={14} /> Discovery map</div>
            <h1 className="nx-page-title">发现</h1>
            <p className="nx-page-subtitle">像看机场时刻表一样理解本季内容：今天更新什么、什么正在上升、下一站可以去哪。</p>
          </div>
        </header>

        {spotlight ? (
          <section className="nx-mosaic">
            <button type="button" className="nx-plane nx-plane-dark col-span-8 min-h-[330px] text-left max-[1100px]:col-span-6" onClick={() => onOpen(spotlight)}>
              <Poster src={spotlight.hero || spotlight.poster} alt={spotlight.title} className="absolute inset-0 size-full opacity-65" />
              <div className="absolute inset-0 bg-gradient-to-r from-black/88 via-black/52 to-black/10" />
              <div className="relative flex h-full min-h-[330px] max-w-[560px] flex-col justify-end p-7">
                <div className="nx-eyebrow !text-[#64d2ff]"><Flame size={14} /> Trending now</div>
                <h2 className="mt-3 text-[31px] font-bold leading-tight text-white">{spotlight.titleCn || spotlight.title}</h2>
                <p className="mt-3 line-clamp-2 text-[13px] leading-relaxed text-white/65">{spotlight.summary || "本季正在升温的内容"}</p>
              </div>
            </button>
            <div className="col-span-4 grid min-h-[330px] grid-rows-2 gap-4 max-[1100px]:col-span-6 max-[1100px]:grid-cols-2 max-[1100px]:grid-rows-1">
              <div className="nx-plane nx-plane-blue flex min-h-[157px] flex-col justify-between p-5">
                <CalendarDays size={27} /><div><strong className="text-[30px]">{feed?.today.length ?? 0}</strong><div className="mt-1 text-[12px] opacity-75">今日放送节点</div></div>
              </div>
              <div className="nx-plane nx-plane-purple flex min-h-[157px] flex-col justify-between p-5">
                <MapIcon size={27} /><div><strong className="text-[30px]">{feed?.trending.length ?? 0}</strong><div className="mt-1 text-[12px] opacity-75">趋势候选</div></div>
              </div>
            </div>
          </section>
        ) : (
          <div className="nx-empty"><div><div className="nx-empty-icon"><Compass size={28} /></div><h2>{loading ? "正在读取今日时刻表" : "暂时无法取得发现内容"}</h2><p>本地媒体库仍然可以正常使用，公开候选会在网络恢复后更新。</p></div></div>
        )}

        {feed?.today.length ? <ExploreShelf title="今日放送" copy="Bangumi 每日放送 · 今天更新" items={feed.today} icon={<CalendarDays size={16} />} onOpen={onOpen} /> : null}
        {feed?.trending.length ? <ExploreShelf title="正在上升" copy="公开评分、排名与当季热度" items={feed.trending} icon={<Flame size={16} />} wide onOpen={onOpen} /> : null}
        {themed.length ? <ExploreShelf title="换一条航线" copy="从今天和趋势中重新组合的探索队列" items={themed.slice().reverse()} icon={<Sparkles size={16} />} onOpen={onOpen} /> : null}
      </motion.div>
    </div>
  );
}

function ExploreShelf({ title, copy, items, icon, wide, onOpen }: { title: string; copy: string; items: Subject[]; icon: React.ReactNode; wide?: boolean; onOpen: (subject: Subject) => void }) {
  return (
    <section className="nx-section">
      <div className="nx-section-head"><div><h2 className="nx-section-title"><span>{icon}</span>{title}</h2><p className="nx-section-copy">{copy}</p></div></div>
      <div className="nx-scroll-row">
        {items.slice(0, 18).map((subject) => (
          <button key={`${subject.provider}-${subject.providerSubjectId}`} type="button" className={`nx-media-tile${wide ? " wide" : ""}`} onClick={() => onOpen(subject)}>
            <div className="nx-media-art"><Poster src={wide ? subject.hero || subject.poster : subject.poster} alt={subjectDisplayTitle(subject)} className="size-full" /></div>
            <div className="nx-media-title">{subjectDisplayTitle(subject)}</div>
            <div className="nx-media-meta">{subject.rating > 0 ? `${subject.rating.toFixed(1)} · ` : ""}Bangumi</div>
          </button>
        ))}
      </div>
    </section>
  );
}
