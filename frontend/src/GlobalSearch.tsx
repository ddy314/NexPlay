import { useDeferredValue, useEffect, useState } from "react";
import { ArrowRight, Search, X } from "lucide-react";
import { AnimatePresence, motion } from "framer-motion";
import { searchCatalog } from "./backend";
import type { Subject } from "./data";
import { Poster } from "./MediaCard";
import { appleEase } from "./motion";

export function GlobalSearch({ open, localSubjects, onClose, onOpen }: { open: boolean; localSubjects: Subject[]; onClose: () => void; onOpen: (subject: Subject) => void }) {
  const [query, setQuery] = useState("");
  const [online, setOnline] = useState<Subject[]>([]);
  const [active, setActive] = useState(0);
  const deferred = useDeferredValue(query.trim());
  const local = deferred ? localSubjects.filter((subject) => `${subject.title} ${subject.titleCn} ${subject.tags.join(" ")}`.toLowerCase().includes(deferred.toLowerCase())).slice(0, 6) : localSubjects.slice(0, 4);
  const results = [...local, ...online.filter((subject) => !local.some((item) => item.canonicalKey === subject.canonicalKey))].slice(0, 14);

  useEffect(() => {
    if (!open) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
      if (event.key === "ArrowDown") { event.preventDefault(); setActive((value) => Math.min(results.length - 1, value + 1)); }
      if (event.key === "ArrowUp") { event.preventDefault(); setActive((value) => Math.max(0, value - 1)); }
      if (event.key === "Enter" && results[active]) { event.preventDefault(); onOpen(results[active]); onClose(); }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [active, onClose, onOpen, open, results]);

  useEffect(() => {
    setActive(0);
    if (!open || deferred.length < 2) { setOnline([]); return; }
    let cancelled = false;
    const timer = window.setTimeout(() => {
      searchCatalog(deferred, 12).then((response) => { if (!cancelled) setOnline(response.subjects); }).catch(() => { if (!cancelled) setOnline([]); });
    }, 180);
    return () => { cancelled = true; window.clearTimeout(timer); };
  }, [deferred, open]);

  return <AnimatePresence>{open && <motion.div className="nx-global-search" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={appleEase} onMouseDown={(event) => { if (event.target === event.currentTarget) onClose(); }}><motion.div className="nx-search-panel" initial={{ y: 18, scale: .985 }} animate={{ y: 0, scale: 1 }} exit={{ y: 12, scale: .99 }} transition={appleEase}><div className="nx-search-input"><Search size={22} className="text-[var(--nx-blue)]" /><input autoFocus value={query} onChange={(event) => setQuery(event.target.value)} placeholder="搜索本地、收藏与 Bangumi…" /><kbd>ESC</kbd><button type="button" onClick={onClose} aria-label="关闭搜索"><X size={18} /></button></div><div className="nx-search-results">{results.length ? results.map((subject, index) => <button key={subject.canonicalKey} type="button" className={`nx-search-row${active === index ? " is-active" : ""}`} onMouseEnter={() => setActive(index)} onClick={() => { onOpen(subject); onClose(); }}><Poster src={subject.poster} alt={subject.title} className="!h-[54px] !w-[44px]" /><span><strong>{subject.titleCn || subject.title}</strong><small>{availabilityLabel(subject.availability)}{subject.tags.length ? ` · ${subject.tags.slice(0, 3).join(" / ")}` : ""}</small></span><ArrowRight size={16} className="text-[var(--nx-ink-3)]" /></button>) : <div className="nx-empty !min-h-[220px] !border-0 !shadow-none"><div><div className="nx-empty-icon"><Search size={25} /></div><h2>{deferred ? "没有找到匹配内容" : "直接输入标题、译名或标签"}</h2><p>使用 ↑ ↓ 选择，Enter 打开。</p></div></div>}</div></motion.div></motion.div>}</AnimatePresence>;
}

function availabilityLabel(value: string) {
  if (value === "localPlayable") return "本地可播放";
  if (value === "cloudCollection") return "云端收藏";
  return "仅在线";
}
