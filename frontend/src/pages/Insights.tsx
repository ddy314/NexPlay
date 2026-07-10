import { useEffect, useState } from "react";
import { Activity, CalendarCheck2, Clock3, Eye, Sparkles, TimerReset } from "lucide-react";
import { motion } from "framer-motion";
import { fetchInsightsDashboard, type InsightsDashboard, type InsightsRange } from "../backend";
import { appleEase } from "../motion";

export function InsightsPage() {
  const [range, setRange] = useState<InsightsRange>("week");
  const [data, setData] = useState<InsightsDashboard | null>(null);
  useEffect(() => {
    let cancelled = false;
    fetchInsightsDashboard(range).then((next) => { if (!cancelled) setData(next); });
    return () => { cancelled = true; };
  }, [range]);

  return (
    <div className="h-full overflow-y-auto overflow-x-hidden">
      <motion.div className="nx-page" initial={{ opacity: 0 }} animate={{ opacity: 1 }} transition={appleEase}>
        <header className="nx-page-header">
          <div>
            <div className="nx-eyebrow"><Activity size={14} /> Local activity</div>
            <h1 className="nx-page-title">观看洞察</h1>
            <p className="nx-page-subtitle">不是 BGM 状态清单，而是从本机真实播放会话中得到的时间、节奏和完成趋势。</p>
          </div>
          <div className="flex rounded-[14px] bg-[var(--nx-plane-2)] p-1">
            {(["week", "month", "year"] as InsightsRange[]).map((value) => (
              <button key={value} type="button" onClick={() => setRange(value)} className={`h-9 rounded-[11px] px-3 text-[12px] font-semibold ${range === value ? "bg-[var(--nx-plane)] text-[var(--nx-blue)] shadow-sm" : "text-[var(--nx-ink-2)]"}`}>
                {{ week: "周", month: "月", year: "年" }[value]}
              </button>
            ))}
          </div>
        </header>

        <section className="nx-mosaic">
          <div className="nx-plane col-span-8 min-h-[340px] max-[1100px]:col-span-6">
            <div className="flex items-center justify-between px-6 pt-5"><div><div className="text-[17px] font-bold">目标圆环</div><div className="mt-1 text-[11px] text-[var(--nx-ink-3)]">今天与本周的有效观看</div></div><Eye size={20} className="text-[var(--nx-blue)]" /></div>
            <div className="nx-insight-rings">{(data?.rings ?? []).map((ring, index) => <ActivityRing key={ring.id} ring={ring} size={index === 0 ? 150 : 126} />)}</div>
          </div>
          <MetricPlane icon={<Clock3 size={18} />} label="有效观看" value={`${Math.round(data?.totalMinutes ?? 0)}`} unit="分钟" tone="blue" />
          <MetricPlane icon={<CalendarCheck2 size={18} />} label="完成集数" value={`${data?.completedEpisodes ?? 0}`} unit="集" tone="green" />
          <MetricPlane icon={<TimerReset size={18} />} label="平均会话" value={`${Math.round(data?.averageSessionMinutes ?? 0)}`} unit="分钟" tone="purple" />
          <MetricPlane icon={<Activity size={18} />} label="连续活跃" value={`${data?.streakDays ?? 0}`} unit="天" tone="orange" />
        </section>

        <section className="nx-mosaic nx-section">
          <div className="nx-plane col-span-7 p-5 max-[1100px]:col-span-6">
            <h2 className="nx-section-title"><span><Activity size={16} /></span>观看时间线</h2>
            {(data?.daily.length ?? 0) > 0
              ? <div className="nx-chart">{(data?.daily ?? []).map((point) => <div key={point.label} className="nx-chart-column"><div className="nx-chart-bar" style={{ height: `${Math.max(3, Math.min(100, point.value / Math.max(1, ...(data?.daily ?? []).map((item) => item.value)) * 100))}%` }} title={`${point.value.toFixed(0)} 分钟`} /><span>{point.label.slice(5)}</span></div>)}</div>
              : <EmptyVisualization label="播放后会在这里形成时间曲线" />}
          </div>
          <div className="nx-plane col-span-5 p-5 max-[1100px]:col-span-6">
            <h2 className="nx-section-title"><span><Clock3 size={16} /></span>一天中的节奏</h2>
            <div className="mt-7 grid gap-4">{(data?.dayparts ?? []).map((item) => <DistributionRow key={item.label} label={item.label} value={item.value} total={(data?.dayparts ?? []).reduce((sum, row) => sum + row.value, 0)} color={item.color} />)}</div>
          </div>
          <div className="nx-plane col-span-5 p-5 max-[1100px]:col-span-6">
            <h2 className="nx-section-title"><span><Sparkles size={16} /></span>兴趣构成</h2>
            {(data?.tags.length ?? 0) > 0
              ? <div className="mt-7 grid gap-4">{(data?.tags ?? []).map((item) => <DistributionRow key={item.label} label={item.label} value={item.value} total={(data?.tags ?? []).reduce((sum, row) => sum + row.value, 0)} color={item.color} />)}</div>
              : <EmptyVisualization label="完成几次观看后生成兴趣分布" compact />}
          </div>
          <div className="nx-plane nx-plane-dark col-span-7 p-6 max-[1100px]:col-span-6">
            <div className="nx-eyebrow !text-[#64d2ff]"><Sparkles size={14} /> Insight</div>
            <div className="mt-6 grid gap-5">{(data?.highlights ?? []).map((item) => <div key={item.title}><h3 className="text-[20px] font-bold text-white">{item.title}</h3><p className="mt-2 text-[12px] leading-relaxed text-white/58">{item.detail}</p></div>)}</div>
          </div>
        </section>
      </motion.div>
    </div>
  );
}

function ActivityRing({ ring, size }: { ring: InsightsDashboard["rings"][number]; size: number }) {
  const stroke = size > 140 ? 15 : 13;
  const radius = (size - stroke) / 2;
  const circumference = Math.PI * 2 * radius;
  const ratio = Math.min(1, ring.value / Math.max(1, ring.goal));
  return <div className="nx-ring" style={{ width: size, height: size }}><svg width={size} height={size}><circle cx={size / 2} cy={size / 2} r={radius} fill="none" stroke="var(--nx-plane-2)" strokeWidth={stroke} /><circle cx={size / 2} cy={size / 2} r={radius} fill="none" stroke={ring.color} strokeWidth={stroke} strokeLinecap="round" strokeDasharray={`${circumference * ratio} ${circumference}`} /></svg><div className="nx-ring-center"><strong>{Math.round(ring.value)}</strong><small>{ring.unit} / {ring.goal}</small></div></div>;
}

function MetricPlane({ icon, label, value, unit, tone }: { icon: React.ReactNode; label: string; value: string; unit: string; tone: string }) {
  return <div className="nx-plane nx-stat col-span-2 max-[1100px]:col-span-3"><div className="nx-stat-label" style={{ color: `var(--nx-${tone})` }}>{icon}{label}</div><div className="nx-stat-value">{value}</div><div className="nx-stat-hint">{unit}</div></div>;
}

function DistributionRow({ label, value, total, color }: { label: string; value: number; total: number; color: string }) {
  const ratio = total > 0 ? value / total : 0;
  return <div><div className="mb-2 flex justify-between text-[11px] font-semibold"><span>{label}</span><span className="text-[var(--nx-ink-3)]">{Math.round(ratio * 100)}%</span></div><div className="h-2 overflow-hidden rounded-full bg-[var(--nx-plane-2)]"><div className="h-full rounded-full" style={{ width: `${ratio * 100}%`, background: color }} /></div></div>;
}

function EmptyVisualization({ label, compact = false }: { label: string; compact?: boolean }) {
  return <div className={`flex ${compact ? "min-h-[150px]" : "min-h-[250px]"} flex-col items-center justify-center text-center`}>
    <div className="flex h-16 items-end gap-1.5" aria-hidden>{[22, 38, 29, 52, 35, 45].map((height, index) => <span key={height} className="w-2 rounded-full bg-[var(--nx-blue-soft)]" style={{ height, opacity: 0.45 + index * 0.07 }} />)}</div>
    <span className="mt-4 text-[11px] font-medium text-[var(--nx-ink-3)]">{label}</span>
  </div>;
}
