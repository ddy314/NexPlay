import { useMemo, type ReactNode } from "react";
import { motion } from "framer-motion";
import {
  Activity,
  CheckCircle2,
  Clock3,
  HardDrive,
  Layers3,
  PlayCircle,
  Star,
  Tags,
} from "lucide-react";
import type { BangumiAuthStatus } from "../backend";
import type { Subject } from "../data";
import { appleSpringSoft } from "../motion";
import { BarChart, EmptyState, Progress, RingChart, StatTile } from "../ui";
import { resolveAssetUrl } from "../utils/assets";
import { cn } from "../utils/cn";

const statusOrder = [
  { type: 3, label: "在看" },
  { type: 1, label: "想看" },
  { type: 2, label: "看过" },
  { type: 4, label: "搁置" },
  { type: 5, label: "抛弃" },
] as const;

export function ProfilePage({
  auth,
  subjects,
}: {
  auth: BangumiAuthStatus;
  subjects: Subject[];
}) {
  const profile = useMemo(() => buildProfile(subjects), [subjects]);
  const avatarInitial = (auth.nickname || auth.username || "B").slice(0, 1).toUpperCase();
  const empty = profile.total === 0;

  return (
    <div className="h-full overflow-y-auto overflow-x-hidden">
      <motion.div
        className="page-shell pb-12"
        initial={{ opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        exit={{ opacity: 0, y: -8 }}
        transition={appleSpringSoft}
      >
        <header className="page-header">
          <div className="flex min-w-0 items-center gap-4">
            {auth.avatarUrl ? (
              <img src={auth.avatarUrl} alt="" className="size-16 rounded-[8px] object-cover" />
            ) : (
              <div className="grid size-16 place-items-center rounded-[8px] bg-[var(--color-primary-soft)] text-[22px] font-bold text-[var(--color-primary)]">
                {avatarInitial}
              </div>
            )}
            <div className="min-w-0">
              <h1 className="truncate text-[42px] font-bold leading-[1] tracking-tight text-[var(--color-text-primary)]">
                {auth.authenticated ? (auth.nickname || auth.username) : "个人"}
              </h1>
              <p className="mt-2.5 text-[17px] font-medium text-[var(--color-text-secondary)]">
                {auth.authenticated ? `@${auth.username}` : "登录 Bangumi 后显示账号仪表盘"}
              </p>
            </div>
          </div>
        </header>

        <section className="mt-7 grid gap-5 xl:grid-cols-[minmax(0,1.4fr)_minmax(340px,0.7fr)]">
          <ProfileHero profile={profile} auth={auth} />
          <IdentityPanel profile={profile} auth={auth} />
        </section>

        <section className="mt-5 grid grid-cols-2 gap-3 lg:grid-cols-6">
          <StatTile icon={<Layers3 size={15} />} label="条目总数" value={empty ? "—" : profile.total} />
          <StatTile icon={<CheckCircle2 size={15} />} label="看过 / 完成" value={empty ? "—" : profile.finishedCount} />
          <StatTile icon={<Activity size={15} />} label="在看 / 进行" value={empty ? "—" : profile.inProgressCount} />
          <StatTile icon={<PlayCircle size={15} />} label="观看集数" value={empty ? "—" : profile.watchedEpisodes} accent />
          <StatTile icon={<Star size={15} />} label="平均评分" value={profile.averageRate ? profile.averageRate.toFixed(1) : "—"} />
          <StatTile
            icon={<HardDrive size={15} />}
            label="本地可播"
            value={empty ? "—" : profile.localPlayable.length}
            hint={auth.pendingSyncCount > 0 ? `${auth.pendingSyncCount} 待同步` : undefined}
          />
        </section>

        <section className="mt-7 grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)] gap-6 max-lg:grid-cols-1">
          <Panel title="观看概览" icon={<Layers3 size={17} />}>
            <ActivityOverview segments={profile.activitySegments} total={profile.total} empty={empty} />
          </Panel>

          <Panel title="评分分布" icon={<Star size={17} />}>
            {profile.rated.length ? (
              <BarChart
                height={176}
                data={profile.scoreBuckets.map((bucket) => ({ label: String(bucket.score), value: bucket.count }))}
              />
            ) : (
              <EmptyState
                className="py-10"
                icon={<Star size={22} />}
                title="还没有评分"
                description="给看过的作品打分后，这里会画出你的评分偏好曲线。"
              />
            )}
          </Panel>
        </section>

        <section className="mt-7 grid gap-6 xl:grid-cols-[minmax(0,0.9fr)_minmax(0,1.1fr)]">
          <Panel title="标签偏好" icon={<Tags size={17} />}>
            {profile.tags.length ? (
              <div className="flex flex-wrap gap-2">
                {profile.tags.slice(0, 18).map(([tag, weight]) => (
                  <span
                    key={tag}
                    className="rounded-full bg-black/[0.045] px-3 py-1.5 text-[12px] font-semibold text-[var(--color-text-secondary)]"
                  >
                    {tag} · {weight}
                  </span>
                ))}
              </div>
            ) : (
              <EmptyPanel title="暂无标签" desc="同步云端条目或刷新条目元数据后会补充标签画像。" compact />
            )}
          </Panel>

          <Panel title={profile.completed.length ? "看过回顾" : "最近条目"} icon={<Clock3 size={17} />}>
            <SubjectRows subjects={(profile.completed.length ? profile.completed : subjects).slice(0, 6)} />
          </Panel>
        </section>
      </motion.div>
    </div>
  );
}

function ProfileHero({
  profile,
  auth,
}: {
  profile: ProfileData;
  auth: BangumiAuthStatus;
}) {
  const featured = profile.featured;
  const title = profile.identity;
  const image = featured ? resolveAssetUrl(featured.hero || featured.poster) : "";
  return (
    <section className="relative min-h-[270px] overflow-hidden rounded-[8px] bg-[var(--color-surface-elevated)] ring-1 ring-inset ring-black/[0.05]">
      {image ? (
        <img src={image} alt={featured?.title ?? ""} className="absolute inset-0 size-full object-cover" />
      ) : null}
      <div className={cn("absolute inset-0", image ? "bg-gradient-to-r from-black/76 via-black/42 to-black/8" : "bg-transparent")} />
      <div className="relative flex min-h-[270px] flex-col justify-end p-7">
        <div className={cn("text-[13px] font-bold", image ? "text-white/72" : "text-[var(--color-text-tertiary)]")}>
          {auth.authenticated ? "Bangumi 画像" : "本地画像"}
        </div>
        <h2 className={cn("mt-2 max-w-2xl text-[34px] font-bold leading-[1.08] tracking-tight", image ? "text-white" : "text-[var(--color-text-primary)]")}>
          {title}
        </h2>
        <p className={cn("mt-3 max-w-xl text-[14px] font-medium leading-relaxed", image ? "text-white/72" : "text-[var(--color-text-secondary)]")}>
          {profile.summary}
        </p>
      </div>
    </section>
  );
}

function IdentityPanel({
  profile,
  auth,
}: {
  profile: ProfileData;
  auth: BangumiAuthStatus;
}) {
  return (
    <aside className="rounded-[8px] bg-[var(--color-surface-elevated)] p-5 ring-1 ring-inset ring-black/[0.05]">
      <div className="flex items-center justify-between">
        <h2 className="text-[15px] font-bold text-[var(--color-text-primary)]">账号状态</h2>
        <span className={cn(
          "rounded-full px-2.5 py-1 text-[11px] font-bold",
          auth.authenticated ? "bg-emerald-500/10 text-emerald-600" : "bg-black/[0.045] text-[var(--color-text-tertiary)]"
        )}>
          {auth.authenticated ? "已登录" : "未登录"}
        </span>
      </div>
      <div className="mt-5 grid gap-4">
        <IdentityLine label="账号" value={auth.username ? `@${auth.username}` : "未连接"} />
        <IdentityLine label="数据来源" value={profile.totalCollections ? "Bangumi 状态" : "本地媒体/空"} />
        <IdentityLine label="画像类型" value={profile.identity} />
        <IdentityLine label="同步队列" value={`${auth.pendingSyncCount} 个待处理`} warn={auth.pendingSyncCount > 0} />
      </div>
      <div className="mt-5">
        <div className="mb-2 flex justify-between text-[12px] font-semibold text-[var(--color-text-tertiary)]">
          <span>完成占比</span>
          <span>{Math.round(profile.completionRate * 100)}%</span>
        </div>
        <Progress value={profile.completionRate} tone="success" />
      </div>
    </aside>
  );
}

function Panel({
  title,
  icon,
  children,
}: {
  title: string;
  icon: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="rounded-[8px] bg-[var(--color-surface-elevated)] p-5 ring-1 ring-inset ring-black/[0.05]">
      <div className="mb-4 flex items-center gap-2">
        <span className="text-[var(--color-accent)]">{icon}</span>
        <h2 className="text-[17px] font-bold tracking-tight text-[var(--color-text-primary)]">{title}</h2>
      </div>
      {children}
    </section>
  );
}

function ActivityOverview({
  segments,
  total,
  empty,
}: {
  segments: Array<{ label: string; value: number; color: string }>;
  total: number;
  empty: boolean;
}) {
  if (empty) {
    return (
      <EmptyState
        className="py-8"
        icon={<Layers3 size={22} />}
        title="还没有观看数据"
        description="扫描本地媒体或登录 Bangumi 后，这里会显示你的观看进度环。"
      />
    );
  }
  const active = segments.filter((segment) => segment.value > 0);
  return (
    <div className="flex items-center gap-6">
      <RingChart
        segments={segments}
        centerLabel={total}
        centerSub="条目"
      />
      <div className="grid min-w-0 flex-1 gap-2.5">
        {(active.length ? active : segments).map((segment) => (
          <div key={segment.label} className="flex items-center gap-2.5">
            <span className="size-2.5 shrink-0 rounded-full" style={{ background: segment.color }} />
            <span className="flex-1 truncate text-[13px] font-medium text-[var(--color-text-secondary)]">{segment.label}</span>
            <span className="text-[13px] font-semibold tabular-nums text-[var(--color-text-primary)]">{segment.value}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

function SubjectRows({ subjects }: { subjects: Subject[] }) {
  if (!subjects.length) {
    return <EmptyPanel title="暂无条目" desc="同步云端条目或扫描媒体库后会显示最近内容。" compact />;
  }
  return (
    <div className="grid gap-2">
      {subjects.map((subject) => {
        const title = subject.titleCn || subject.title;
        return (
          <div key={`${subject.provider}-${subject.providerSubjectId}-${subject.id}`} className="flex min-w-0 items-center gap-3 rounded-[8px] px-2 py-2">
            <div className="size-12 shrink-0 overflow-hidden rounded-[8px] bg-black/[0.05]">
              {subject.poster ? (
                <img src={resolveAssetUrl(subject.poster)} alt={title} className="size-full object-cover" loading="lazy" />
              ) : null}
            </div>
            <div className="min-w-0">
              <div className="truncate text-[13px] font-semibold text-[var(--color-text-primary)]">{title}</div>
              <div className="mt-1 truncate text-[11px] font-medium text-[var(--color-text-tertiary)]">
                {statusLabel(subject)} · {subject.bgmRate > 0 ? `${subject.bgmRate}/10` : subject.year || "未评分"}
              </div>
            </div>
          </div>
        );
      })}
    </div>
  );
}

function IdentityLine({
  label,
  value,
  warn,
}: {
  label: string;
  value: string;
  warn?: boolean;
}) {
  return (
    <div className="flex justify-between gap-3 text-[13px]">
      <span className="font-medium text-[var(--color-text-tertiary)]">{label}</span>
      <span className={cn("truncate font-semibold text-[var(--color-text-primary)]", warn && "text-amber-600")}>{value}</span>
    </div>
  );
}

function EmptyPanel({
  title,
  desc,
  compact,
}: {
  title: string;
  desc: string;
  compact?: boolean;
}) {
  return (
    <div className={cn("flex flex-col items-center justify-center rounded-[8px] bg-black/[0.025] text-center", compact ? "min-h-[120px] p-4" : "min-h-[180px] p-6")}>
      <div className="text-[13px] font-bold text-[var(--color-text-secondary)]">{title}</div>
      <div className="mt-1 max-w-sm text-[12px] font-medium text-[var(--color-text-tertiary)]">{desc}</div>
    </div>
  );
}

type ProfileData = ReturnType<typeof buildProfile>;

function buildProfile(subjects: Subject[]) {
  const collectionSubjects = subjects.filter((subject) => subject.bgmCollectionType);
  const source = collectionSubjects.length ? collectionSubjects : subjects;
  const statusCounts = statusOrder.map((item) => ({
    ...item,
    count: collectionSubjects.filter((subject) => subject.bgmCollectionType === item.type).length,
  }));
  const rated = source.filter((subject) => subject.bgmRate > 0);
  const completed = source
    .filter((subject) => subject.bgmCollectionType === 2 || subject.progress >= 1)
    .sort(prioritySorter);
  const watching = source
    .filter((subject) => subject.bgmCollectionType === 3 || (subject.progress > 0 && subject.progress < 1))
    .sort(progressSorter);
  const localPlayable = source.filter((subject) => subject.local || subject.files > 0);
  const scoreBuckets = Array.from({ length: 10 }, (_, index) => {
    const score = index + 1;
    return {
      score,
      count: rated.filter((subject) => subject.bgmRate === score).length,
    };
  });
  const tags = buildTags(source);
  const totalCollections = collectionSubjects.length;
  const total = source.length;

  // Local-derived activity stats — these work even with no Bangumi account, so the
  // dashboard never renders blank.
  const watchedEpisodes = source.reduce((sum, subject) => sum + Math.max(0, subject.watchedEpisodes || 0), 0);
  const finishedCount = source.filter((subject) => subject.bgmCollectionType === 2 || subject.progress >= 1).length;
  const inProgressCount = source.filter((subject) => subject.progress > 0 && subject.progress < 1).length;
  const plannedCount = source.filter((subject) => subject.bgmCollectionType === 1).length;
  const notStarted = Math.max(0, total - finishedCount - inProgressCount - plannedCount);
  const activitySegments = [
    { label: "看过 / 完成", value: finishedCount, color: "var(--color-success)" },
    { label: "在看 / 进行中", value: inProgressCount, color: "var(--color-accent)" },
    { label: "想看 / 计划", value: plannedCount, color: "#5ac8fa" },
    { label: "未开始", value: notStarted, color: "var(--color-surface-4)" },
  ];

  const completionRate = totalCollections > 0 ? completed.length / totalCollections : total > 0 ? completed.length / total : 0;
  const averageRate = rated.length ? rated.reduce((sum, subject) => sum + subject.bgmRate, 0) / rated.length : 0;
  const featured = completed[0] ?? [...rated].sort(prioritySorter)[0] ?? watching[0] ?? [...source].sort(prioritySorter)[0];
  const identity = profileIdentity({ completed, watching, rated, total, localPlayable });
  return {
    total,
    totalCollections,
    statusCounts,
    rated,
    completed,
    watching,
    localPlayable,
    scoreBuckets,
    tags,
    completionRate,
    averageRate,
    featured,
    identity,
    watchedEpisodes,
    finishedCount,
    inProgressCount,
    activitySegments,
    summary: profileSummary(identity, total, completed.length, rated.length, localPlayable.length),
  };
}

function profileIdentity(input: {
  completed: Subject[];
  watching: Subject[];
  rated: Subject[];
  total: number;
  localPlayable: Subject[];
}) {
  if (input.total === 0) return "等待建立画像";
  if (input.completed.length >= Math.max(3, input.total * 0.55)) return "看过型账号";
  if (input.rated.length >= Math.max(3, input.total * 0.35)) return "评分型账号";
  if (input.watching.length >= Math.max(2, input.total * 0.25)) return "追番型账号";
  if (input.localPlayable.length >= Math.max(3, input.total * 0.5)) return "本地媒体型";
  return "条目探索型";
}

function profileSummary(identity: string, total: number, completed: number, rated: number, local: number) {
  if (total === 0) {
    return "同步 Bangumi 或扫描本地媒体后，这里会生成个人观看画像。";
  }
  if (identity === "看过型账号") {
    return `看过记录是你的主要数据源：${completed} 部完成条目会用于回顾、标签和推荐。`;
  }
  if (identity === "评分型账号") {
    return `评分数据足够丰富：${rated} 个评分会优先影响推荐和标签权重。`;
  }
  if (identity === "本地媒体型") {
    return `本地可播放资源较多：${local} 部条目可直接进入播放或资源管理。`;
  }
  return `${total} 个条目正在构成你的 Bangumi 和本地媒体画像。`;
}

function buildTags(subjects: Subject[]) {
  const weights = new Map<string, number>();
  for (const subject of subjects) {
    const weight = Math.max(1, subject.bgmRate || Math.round(subject.rating) || 1);
    for (const tag of subject.tags.slice(0, 6)) {
      weights.set(tag, (weights.get(tag) ?? 0) + weight);
    }
  }
  return [...weights.entries()].sort((a, b) => b[1] - a[1]);
}

function statusLabel(subject: Subject) {
  if (subject.bgmCollectionLabel) return subject.bgmCollectionLabel;
  if (subject.local || subject.files > 0) return "本地可播";
  return "Bangumi";
}

function progressSorter(left: Subject, right: Subject) {
  return (right.progress - left.progress) || prioritySorter(left, right);
}

function prioritySorter(left: Subject, right: Subject) {
  return (right.bgmRate - left.bgmRate)
    || (right.rating - left.rating)
    || ((left.rank || Number.MAX_SAFE_INTEGER) - (right.rank || Number.MAX_SAFE_INTEGER))
    || left.title.localeCompare(right.title);
}
