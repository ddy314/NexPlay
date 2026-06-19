import { memo, useState } from "react";
import { motion } from "framer-motion";
import { Play } from "lucide-react";
import { appleSpring, appleSpringBouncy } from "./motion";
import { cn } from "./utils/cn";
import { Badge } from "./ui";
import { STATUS_COLOR, STATUS_LABEL, type Subject } from "./data";

type ImageLoading = "eager" | "lazy";
type ImageFetchPriority = "auto" | "high" | "low";

export const Poster = memo(function Poster({
  src,
  alt,
  className,
  loading = "lazy",
  fetchPriority = "auto",
}: {
  src?: string;
  alt: string;
  className?: string;
  loading?: ImageLoading;
  fetchPriority?: ImageFetchPriority;
}) {
  const [loaded, setLoaded] = useState(false);
  const [failed, setFailed] = useState(false);
  const resolvedSrc = src ? window.nexplay?.resolveAssetUrl(src) ?? src : src;
  if (!src || failed) {
    return (
      <div
        className={cn(
          "relative grid place-items-center bg-gradient-to-br from-[var(--color-surface-3)] to-[var(--color-surface-1)] text-[var(--color-on-surface-faint)]",
          className
        )}
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.4} className="size-10 opacity-70">
          <rect x="3" y="4" width="18" height="16" rx="2" />
          <circle cx="9" cy="10" r="1.5" />
          <path d="M3 16l5-5 5 5 3-3 5 5" />
        </svg>
        <span className="text-[10px] mt-2 tracking-wide uppercase">No Poster</span>
      </div>
    );
  }
  return (
    <div className={cn("relative overflow-hidden", className)}>
      {!loaded && <div className="absolute inset-0 skeleton" />}
      <img
        src={resolvedSrc}
        alt={alt}
        loading={loading}
        decoding="async"
        fetchPriority={fetchPriority}
        onLoad={() => setLoaded(true)}
        onError={() => setFailed(true)}
        className={cn(
          "size-full object-cover transition-all duration-500",
          loaded ? "opacity-100" : "opacity-0"
        )}
      />
    </div>
  );
});

export const MediaCard = memo(function MediaCard({
  subject,
  onClick,
  selected,
  index = 0,
  imageLoading = "lazy",
  imageFetchPriority = "auto",
}: {
  subject: Subject;
  onClick?: () => void;
  selected?: boolean;
  index?: number;
  imageLoading?: ImageLoading;
  imageFetchPriority?: ImageFetchPriority;
}) {
  const progressText = `${subject.watchedEpisodes} / ${subject.episodes || subject.files || "?"} 话`;
  const resolvedPoster = subject.poster
    ? window.nexplay?.resolveAssetUrl(subject.poster) ?? subject.poster
    : "";

  return (
    <motion.button
      type="button"
      onClick={onClick}
      className={cn(
        "group relative min-w-0 cursor-pointer text-left focus:outline-none",
        selected && "ring-2 ring-[var(--color-primary)] ring-offset-2 ring-offset-[var(--color-bg)]"
      )}
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      transition={{ duration: 0.18, delay: Math.min(index, 12) * 0.012 }}
    >
      <div className="relative mb-2.5">
        {resolvedPoster && (
          <div
            className="absolute inset-[8%] -z-10 translate-y-4 scale-[0.88] rounded-[var(--radius-card)] bg-cover bg-center opacity-0 blur-[24px] saturate-[1.25] transition-opacity duration-300 group-hover:opacity-[0.16]"
            style={{ backgroundImage: `url(${resolvedPoster})` }}
          />
        )}
        <motion.div
          className="relative aspect-[3/4] overflow-hidden rounded-[var(--radius-card)] bg-[var(--color-surface-elevated)]"
          style={{
            boxShadow:
              "0 1px 2px rgba(0,0,0,0.08), 0 10px 22px rgba(0,0,0,0.10)",
          }}
          whileHover={{
            scale: 1.01,
            boxShadow:
              "0 2px 8px rgba(0,0,0,0.10), 0 14px 28px rgba(0,0,0,0.14)",
          }}
          whileTap={{ scale: 0.96 }}
          transition={appleSpringBouncy}
        >
          <Poster
            src={subject.poster}
            alt={subject.title}
            className="absolute inset-0"
            loading={imageLoading}
            fetchPriority={imageFetchPriority}
          />

          <div className="absolute inset-x-0 top-0 h-16 bg-gradient-to-b from-black/25 to-transparent opacity-0 transition-opacity duration-300 group-hover:opacity-100" />
          <div className="absolute inset-x-0 bottom-0 h-24 bg-gradient-to-t from-black/40 to-transparent opacity-0 transition-opacity duration-300 group-hover:opacity-100" />

          <div className="absolute right-2.5 top-2.5 translate-y-1 opacity-0 transition-all duration-300 group-hover:translate-y-0 group-hover:opacity-100">
            {subject.rating > 0 ? (
              <div className="glass-dark flex items-center gap-1 rounded-[var(--radius-control)] px-2 py-[3px] text-[10.5px] font-semibold tracking-wide text-white">
                <span className="text-amber-400">★</span> {subject.rating.toFixed(1)}
              </div>
            ) : subject.status !== "matched" ? (
              <span
                className={cn(
                  "inline-flex h-6 items-center rounded-full px-2 text-[11px] font-medium ring-1 ring-inset",
                  STATUS_COLOR[subject.status]
                )}
              >
                {STATUS_LABEL[subject.status]}
              </span>
            ) : null}
          </div>

          {subject.newEpisode && (
            <div className="absolute left-2.5 top-2.5">
              <Badge tone="primary">NEW</Badge>
            </div>
          )}

          <div className="absolute inset-0 flex scale-75 items-center justify-center opacity-0 transition-all duration-300 group-hover:scale-100 group-hover:opacity-100">
            <motion.div
              className="relative flex size-12 items-center justify-center rounded-full bg-white/92"
              style={{
                boxShadow:
                  "0 4px 18px rgba(0,0,0,0.18), 0 1px 3px rgba(0,0,0,0.1)",
              }}
              whileHover={{ scale: 1.04 }}
              transition={appleSpringBouncy}
            >
              <Play size={18} className="ml-0.5 text-[var(--color-text-primary)]" fill="currentColor" />
            </motion.div>
          </div>

          {subject.progress > 0 && (
            <div className="absolute inset-x-0 bottom-0 h-[3px] bg-black/20">
              <motion.div
                className="h-full rounded-r-full"
                style={{ background: "var(--color-primary)" }}
                initial={{ width: 0 }}
                animate={{ width: `${Math.min(100, Math.max(0, subject.progress * 100))}%` }}
                transition={appleSpring}
              />
            </div>
          )}

          <div className="pointer-events-none absolute inset-0 rounded-[var(--radius-card)] ring-1 ring-inset ring-black/[0.05]" />
        </motion.div>
      </div>

      <div className="px-0.5">
        <h3 className="truncate text-[15px] font-semibold leading-tight text-[var(--color-text-primary)] transition-colors duration-200 group-hover:text-[var(--color-accent)]">
          {subject.title}
        </h3>
        <p className="mt-1.5 truncate text-[13px] font-medium text-[var(--color-text-tertiary)]">
          {subject.titleCn} · {subject.year}
        </p>
        {subject.progress > 0 && subject.progress < 1 && (
          <p className="mt-1.5 text-[13px] font-medium tabular-nums text-[var(--color-accent)]/80">
            {progressText}
          </p>
        )}
        {subject.progress >= 1 && (
          <p className="mt-1.5 text-[13px] font-medium text-green-600/80">
            已完成 · {subject.episodes || subject.files || "?"} 话
          </p>
        )}
      </div>
    </motion.button>
  );
});
