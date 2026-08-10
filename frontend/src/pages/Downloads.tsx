import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { motion } from "framer-motion";
import { AlertCircle, ArrowRight, CheckCircle2, DownloadCloud, Gauge, Loader2, Pause, PauseCircle, Play, RefreshCw, Search, ShieldCheck, Trash2, X } from "lucide-react";
import { controlDownloadTask, downloadTasks, type DownloadTask } from "../backend";
import { friendlyDownloadError } from "../downloadErrors";
import { appleSpringBouncy, appleSpringSoft } from "../motion";

export function DownloadsPage({
  onSnack,
}: {
  onSnack: (text: string, tone?: "neutral" | "success" | "danger") => void;
}) {
  const [tasks, setTasks] = useState<DownloadTask[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [actingTaskId, setActingTaskId] = useState<number | null>(null);
  const refreshGenerationRef = useRef(0);

  const refresh = useCallback(async (quiet = false) => {
    const generation = ++refreshGenerationRef.current;
    if (!quiet) setLoading(true);
    try {
      const response = await downloadTasks();
      if (generation !== refreshGenerationRef.current) return;
      setTasks(response.tasks);
      setError(null);
    } catch (caught) {
      const message = friendlyDownloadError(caught);
      setError(message);
      if (!quiet) onSnack(`读取下载状态失败：${message}`, "danger");
    } finally {
      if (!quiet) setLoading(false);
    }
  }, [onSnack]);

  const controlTask = useCallback(async (
    task: DownloadTask,
    action: "pause" | "resume" | "cancel" | "remove",
    deleteFiles = false,
  ) => {
    if (action === "cancel" && !window.confirm(deleteFiles ? "确定取消任务并删除已经下载的文件？此操作不可撤销。" : "确定取消任务？已下载文件会保留。")) return;
    if (action === "remove" && !window.confirm("确定只清除这条下载记录？本地文件不会删除。")) return;
    setActingTaskId(task.id);
    try {
      const response = await controlDownloadTask({ taskId: task.id, action, deleteFiles });
      setTasks(response.tasks);
      const labels = { pause: "已暂停", resume: "已继续", cancel: "已取消", remove: "已清除" };
      onSnack(`${labels[action]}下载任务。`, "success");
    } catch (caught) {
      const message = friendlyDownloadError(caught);
      onSnack(`操作下载任务失败：${message}`, "danger");
    } finally {
      setActingTaskId(null);
    }
  }, [onSnack]);

  useEffect(() => {
    void refresh();
    const calibrate = () => {
      if (document.visibilityState === "visible") void refresh(true);
    };
    const timer = window.setInterval(() => {
      calibrate();
    }, 15_000);
    document.addEventListener("visibilitychange", calibrate);
    const unsubscribe = window.nexplay?.onBackendEvent((event) => {
      if (event.type === "downloadCompleted") void refresh(true);
    });
    return () => {
      window.clearInterval(timer);
      document.removeEventListener("visibilitychange", calibrate);
      unsubscribe?.();
    };
  }, [refresh]);

  const counts = useMemo(() => ({
    active: tasks.filter((task) => ["pending", "queued", "downloading", "paused"].includes(task.status)).length,
    completed: tasks.filter((task) => task.status === "completed").length,
    failed: tasks.filter((task) => task.status === "failed").length,
  }), [tasks]);
  const totalSpeed = tasks.reduce((sum, task) => sum + Math.max(0, task.dlspeed), 0);

  return (
    <div className="h-full overflow-y-auto overflow-x-hidden">
      <motion.div
        className="page-shell"
        initial={{ opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        exit={{ opacity: 0, y: -8 }}
        transition={appleSpringSoft}
      >
        <header className="page-header">
          <div>
            <div className="nx-eyebrow"><DownloadCloud size={14} /> Transfer route</div>
            <h1 className="nx-page-title">下载</h1>
            <p className="nx-page-subtitle">从搜索、队列到可播放状态，一眼看清内容正走到哪一站。</p>
          </div>
          <motion.button
            type="button"
            onClick={() => void refresh()}
            disabled={loading}
            className="flex h-10 items-center justify-center gap-2 rounded-[var(--radius-pill)] bg-[var(--color-primary)] px-4 text-[13px] font-semibold text-white disabled:opacity-60"
            whileTap={{ scale: 0.96 }}
            transition={appleSpringBouncy}
          >
            {loading ? <Loader2 size={15} className="animate-spin" /> : <RefreshCw size={15} />}
            刷新
          </motion.button>
        </header>

        <section className="nx-mosaic mt-7">
          <div className="nx-plane col-span-9 flex min-h-[138px] items-center gap-3 px-6 max-[1100px]:col-span-6">
            <PipelineStep icon={<Search size={18} />} label="找到资源" value={tasks.length} active={tasks.length > 0} />
            <ArrowRight size={18} className="text-[var(--nx-ink-3)]" />
            <PipelineStep icon={<DownloadCloud size={18} />} label="下载中" value={counts.active} active={counts.active > 0} />
            <ArrowRight size={18} className="text-[var(--nx-ink-3)]" />
            <PipelineStep icon={<ShieldCheck size={18} />} label="已就绪" value={counts.completed} active={counts.completed > 0} />
          </div>
          <div className="nx-plane nx-plane-blue col-span-3 flex min-h-[138px] flex-col justify-between p-5 max-[1100px]:col-span-6"><Gauge size={24} /><div><strong className="text-[27px]">{formatBytes(totalSpeed)}/s</strong><div className="mt-1 text-[11px] opacity-70">当前总速度</div></div></div>
        </section>

        {error && (
          <div className="mt-5 rounded-[var(--radius-card)] bg-rose-500/8 px-4 py-3 text-[13px] font-medium text-rose-600">
            {error}
          </div>
        )}

        <div className="mt-6 grid gap-2.5 pb-10">
          {tasks.map((task) => (
            <DownloadRow
              key={task.id}
              task={task}
              acting={actingTaskId === task.id}
              onAction={(action, deleteFiles) => void controlTask(task, action, deleteFiles)}
            />
          ))}
          {!loading && tasks.length === 0 && (
            <div className="flex min-h-[300px] items-center justify-center text-[13px] font-medium text-[var(--color-text-tertiary)]">
              还没有下载任务
            </div>
          )}
        </div>
      </motion.div>
    </div>
  );
}

function PipelineStep({ icon, label, value, active }: { icon: React.ReactNode; label: string; value: number; active: boolean }) {
  return <div className="flex min-w-0 flex-1 items-center gap-3"><span className={`grid size-11 shrink-0 place-items-center rounded-[14px] ${active ? "bg-[var(--nx-blue)] text-white" : "bg-[var(--nx-plane-2)] text-[var(--nx-ink-3)]"}`}>{icon}</span><span><strong className="block text-[22px] font-bold">{value}</strong><small className="text-[11px] text-[var(--nx-ink-3)]">{label}</small></span></div>;
}

function DownloadRow({
  task,
  acting,
  onAction,
}: {
  task: DownloadTask;
  acting: boolean;
  onAction: (action: "pause" | "resume" | "cancel" | "remove", deleteFiles?: boolean) => void;
}) {
  const progress = Math.round(Math.min(1, Math.max(0, task.progress)) * 100);
  const status = statusMeta(task.status, task.stale);
  const canPause = ["pending", "queued", "downloading"].includes(task.status) && !task.stale;
  const canResume = task.status === "paused" && !task.stale;
  const canCancel = ["pending", "queued", "downloading", "paused"].includes(task.status) && !task.stale;
  return (
    <motion.div
      className="download-row"
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={appleSpringSoft}
    >
      <div className="flex min-w-0 flex-1 flex-col gap-2">
        <div className="flex min-w-0 items-center gap-2">
          <span className={`download-status-icon ${status.className}`}>{status.icon}</span>
          <div className="min-w-0 flex-1 truncate text-[13.5px] font-semibold text-[var(--color-text-primary)]">
            {task.title}
          </div>
          <span className="text-[12px] font-semibold tabular-nums text-[var(--color-text-secondary)]">{progress}%</span>
        </div>
        <div className="download-progress">
          <div style={{ width: `${progress}%` }} />
        </div>
        <div className="flex flex-wrap gap-x-3 gap-y-1 text-[11.5px] text-[var(--color-text-tertiary)]">
          <span>{status.label}</span>
          {task.episodeNumber && <span>第 {task.episodeNumber} 话</span>}
          {task.size > 0 && <span>{formatBytes(task.downloaded)} / {formatBytes(task.size)}</span>}
          {task.dlspeed > 0 && <span>{formatBytes(task.dlspeed)}/s</span>}
          {task.eta > 0 && <span>剩余 {formatEta(task.eta)}</span>}
          {task.savePath && <span className="max-w-full truncate">{task.savePath}</span>}
          {task.qbittorrentHash && <span className="font-mono">{task.qbittorrentHash.slice(0, 12)}</span>}
        </div>
        {task.error && (
          <div className="text-[12px] font-medium text-rose-600">{friendlyDownloadError(task.error)}</div>
        )}
      </div>
      <div className="download-row-actions">
        {acting ? (
          <button type="button" disabled className="download-action-button">
            <Loader2 size={14} className="animate-spin" />
          </button>
        ) : (
          <>
            {canPause && (
              <button type="button" title="暂停" aria-label="暂停" className="download-action-button" onClick={() => onAction("pause")}>
                <Pause size={14} />
              </button>
            )}
            {canResume && (
              <button type="button" title="继续" aria-label="继续" className="download-action-button" onClick={() => onAction("resume")}>
                <Play size={14} />
              </button>
            )}
            {canCancel && (
              <button type="button" title="取消" aria-label="取消" className="download-action-button danger" onClick={() => onAction("cancel")}>
                <X size={14} />
              </button>
            )}
            {canCancel && (
              <button type="button" title="取消并删除文件" aria-label="取消并删除文件" className="download-action-button danger" onClick={() => onAction("cancel", true)}>
                <Trash2 size={14} />
              </button>
            )}
            {(task.stale || task.status === "failed" || task.status === "completed" || task.status === "missing") && (
              <button type="button" title="清除记录" aria-label="清除记录" className="download-action-button" onClick={() => onAction("remove")}>
                <Trash2 size={14} />
              </button>
            )}
          </>
        )}
      </div>
    </motion.div>
  );
}

function statusMeta(status: string, stale: boolean) {
  if (status === "missing") {
    return { label: "qB 中不存在", className: "text-amber-600 bg-amber-500/8", icon: <AlertCircle size={16} /> };
  }
  if (stale) {
    return {
      label: "BT 状态未同步",
      className: "text-amber-600 bg-amber-500/8",
      icon: <AlertCircle size={16} />,
    };
  }
  switch (status) {
    case "completed":
      return { label: "已完成", className: "text-emerald-600 bg-emerald-500/8", icon: <CheckCircle2 size={16} /> };
    case "failed":
      return { label: "失败", className: "text-rose-600 bg-rose-500/8", icon: <AlertCircle size={16} /> };
    case "paused":
      return { label: "已暂停", className: "text-amber-600 bg-amber-500/8", icon: <PauseCircle size={16} /> };
    case "downloading":
      return { label: "下载中", className: "text-blue-600 bg-blue-500/8", icon: <DownloadCloud size={16} /> };
    case "queued":
      return { label: "队列中", className: "text-[var(--color-accent)] bg-[var(--color-accent)]/8", icon: <Loader2 size={16} /> };
    default:
      return { label: "等待中", className: "text-[var(--color-text-secondary)] bg-black/[0.035]", icon: <DownloadCloud size={16} /> };
  }
}

function formatBytes(value: number) {
  if (value >= 1024 ** 3) return `${(value / 1024 ** 3).toFixed(1)} GB`;
  if (value >= 1024 ** 2) return `${(value / 1024 ** 2).toFixed(1)} MB`;
  if (value >= 1024) return `${(value / 1024).toFixed(1)} KB`;
  return `${value} B`;
}

function formatEta(seconds: number) {
  if (seconds >= 3600) return `${Math.round(seconds / 3600)} 小时`;
  if (seconds >= 60) return `${Math.round(seconds / 60)} 分钟`;
  return `${seconds} 秒`;
}
