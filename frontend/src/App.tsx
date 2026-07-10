import { useCallback, useEffect, useMemo, useState, type MouseEvent } from "react";
import { AnimatePresence, MotionConfig, motion } from "framer-motion";
import { GlobalSearch } from "./GlobalSearch";
import { NavRail, type Route } from "./NavRail";
import { useBackendSnapshot } from "./backend";
import type { PlaybackEpisode, Subject } from "./data";
import { appleEase } from "./motion";
import { DetailPage } from "./pages/Detail";
import { DownloadsPage } from "./pages/Downloads";
import { ExplorePage } from "./pages/Explore";
import { HomePage } from "./pages/Home";
import { InsightsPage } from "./pages/Insights";
import { LibraryPage } from "./pages/Library";
import { PlayerPage } from "./pages/Player";
import { ResourcesPage, type ResourceSearchPrefill } from "./pages/Resources";
import { SettingsPage } from "./pages/Settings";
import { BootSplash, Snackbar, useSnackbar } from "./ui";

type PlaybackState = { subject: Subject; episode: PlaybackEpisode };
type AppView =
  | { kind: "route"; route: Route }
  | { kind: "detail"; subject: Subject }
  | { kind: "playback"; playback: PlaybackState }
  | { kind: "resources"; prefill: ResourceSearchPrefill | null };

export default function App() {
  const [viewStack, setViewStack] = useState<AppView[]>([{ kind: "route", route: "home" }]);
  const [theme, setTheme] = useState<"light" | "dark">(readStoredTheme);
  const [navCollapsed, setNavCollapsed] = useState(() => readStoredBoolean("nexplay.navCollapsed", false));
  const [searchOpen, setSearchOpen] = useState(false);
  const [bootDone, setBootDone] = useState(false);
  const [bootLeaving, setBootLeaving] = useState(false);
  const [minElapsed, setMinElapsed] = useState(false);
  const backend = useBackendSnapshot();
  const snack = useSnackbar();

  const collectionSubjects = useMemo(() => {
    const localBgm = backend.subjects.filter((subject) => subject.bgmCollectionType);
    const seen = new Set<string>();
    return [...localBgm, ...backend.bangumiCollections].filter((subject) => {
      const key = `${subject.provider}:${subject.providerSubjectId || subject.id}`;
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });
  }, [backend.bangumiCollections, backend.subjects]);
  const allSubjects = useMemo(() => [...backend.subjects, ...collectionSubjects], [backend.subjects, collectionSubjects]);
  const currentView = viewStack.at(-1) ?? { kind: "route", route: "home" as Route };
  const route = currentView.kind === "route"
    ? currentView.route
    : [...viewStack].reverse().find((view): view is { kind: "route"; route: Route } => view.kind === "route")?.route ?? "home";
  const playerActive = currentView.kind === "playback";

  useEffect(() => { const timer = window.setTimeout(() => setMinElapsed(true), 500); return () => window.clearTimeout(timer); }, []);
  useEffect(() => { if (!bootDone && !bootLeaving && !backend.loading && minElapsed) setBootLeaving(true); }, [backend.loading, bootDone, bootLeaving, minElapsed]);
  useEffect(() => { if (!bootLeaving) return; const timer = window.setTimeout(() => setBootDone(true), 360); return () => window.clearTimeout(timer); }, [bootLeaving]);
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    document.documentElement.style.colorScheme = theme;
    window.localStorage.setItem("nexplay.theme", theme);
  }, [theme]);
  useEffect(() => { window.localStorage.setItem("nexplay.navCollapsed", String(navCollapsed)); }, [navCollapsed]);
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") { event.preventDefault(); setSearchOpen(true); }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const handleRoute = useCallback((next: Route) => {
    setSearchOpen(false);
    setViewStack([{ kind: "route", route: next }]);
  }, []);
  const openDetail = useCallback((subject: Subject) => {
    setViewStack((current) => [...current, { kind: "detail", subject }]);
  }, []);
  const goBack = useCallback(() => setViewStack((current) => current.length > 1 ? current.slice(0, -1) : current), []);
  const openResourceSearch = useCallback((subject: Subject) => setViewStack((current) => [...current, { kind: "resources", prefill: { subject } }]), []);
  const sameSubject = useCallback((left: Subject, right: Subject) => left.provider === right.provider && left.providerSubjectId === right.providerSubjectId, []);
  const refreshSubjectInStack = useCallback(async (base: Subject) => {
    const next = await backend.refresh();
    const updated = [...next.subjects, ...next.bangumiCollections].find((candidate) => sameSubject(candidate, base));
    if (!updated) return;
    setViewStack((current) => current.map((view) => {
      if (view.kind === "detail" && sameSubject(view.subject, updated)) return { ...view, subject: { ...view.subject, ...updated } };
      if (view.kind === "playback" && sameSubject(view.playback.subject, updated)) return { ...view, playback: { ...view.playback, subject: { ...view.playback.subject, ...updated } } };
      return view;
    }));
  }, [backend, sameSubject]);

  return (
    <MotionConfig reducedMotion="user">
      <div data-theme={theme} data-nav-collapsed={navCollapsed ? "true" : "false"} className="app-shell relative h-screen w-screen overflow-hidden" onMouseDownCapture={suppressNonTextControlFocus} onPointerUpCapture={blurActiveNonTextControl}>
        {!playerActive && <NavRail route={route} onRoute={handleRoute} onSearch={() => setSearchOpen(true)} theme={theme} onToggleTheme={() => setTheme((value) => value === "light" ? "dark" : "light")} collapsed={navCollapsed} onToggleCollapsed={() => setNavCollapsed((value) => !value)} />}
        <main className="app-main absolute inset-y-0 right-0 z-10 min-w-0 overflow-hidden transition-[left] duration-200" style={{ left: playerActive ? 0 : "var(--nav-width)" }}>
          <AnimatePresence initial={false} mode="sync">
            <motion.div key={viewKey(currentView)} className="absolute inset-0" initial={{ opacity: 0, x: currentView.kind === "route" ? 0 : 18 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: currentView.kind === "route" ? 0 : -12 }} transition={appleEase}>
              {currentView.kind === "playback" ? <PlayerPage subject={currentView.playback.subject} initialEpisode={currentView.playback.episode} onBack={goBack} onSubjectUpdated={refreshSubjectInStack} onSnack={snack.show} />
                : currentView.kind === "detail" ? <DetailPage subject={currentView.subject} onBack={goBack} onPlay={(subject, episode) => setViewStack((current) => [...current, { kind: "playback", playback: { subject, episode } }])} onFindResources={openResourceSearch} onSubjectUpdated={() => refreshSubjectInStack(currentView.subject)} onSnack={snack.show} />
                  : currentView.kind === "resources" ? <ResourcesPage prefill={currentView.prefill} onBackToDetail={goBack} onSnack={snack.show} />
                    : currentView.route === "home" ? <HomePage subjects={allSubjects} onOpen={openDetail} onNavigate={handleRoute} />
                      : currentView.route === "discover" ? <ExplorePage onOpen={openDetail} />
                        : currentView.route === "insights" ? <InsightsPage />
                          : currentView.route === "downloads" ? <DownloadsPage onSnack={snack.show} />
                            : currentView.route === "settings" ? <SettingsPage onSnack={snack.show} />
                              : <LibraryPage route="library" subjects={backend.subjects} cloudSubjects={collectionSubjects} searchQuery="" onSearchQueryChange={() => undefined} scanStatus={backend.scanStatus} logs={backend.logs} loading={backend.loading} error={backend.error} onOpen={openDetail} onSnack={snack.show} onScan={async () => { try { const result = await backend.scanLibrary(); if (!result) return; snack.show(`扫描完成：新增 ${result.summary.added}，修改 ${result.summary.modified}`, "success"); } catch (error) { snack.show(`扫描失败：${error instanceof Error ? error.message : String(error)}`, "danger"); } }} />}
            </motion.div>
          </AnimatePresence>
        </main>
        <GlobalSearch open={searchOpen} localSubjects={allSubjects} onClose={() => setSearchOpen(false)} onOpen={openDetail} />
        <Snackbar msg={snack.msg} onDismiss={snack.dismiss} />
        {!bootDone && <BootSplash leaving={bootLeaving} />}
      </div>
    </MotionConfig>
  );
}

function viewKey(view: AppView) {
  if (view.kind === "route") return `route-${view.route}`;
  if (view.kind === "detail") return `detail-${view.subject.id}`;
  if (view.kind === "playback") return `player-${view.playback.subject.id}-${view.playback.episode.key}`;
  return `resources-${view.prefill?.subject.id ?? "manual"}`;
}

function readStoredTheme(): "light" | "dark" {
  const value = window.localStorage.getItem("nexplay.theme");
  if (value === "dark" || value === "light") return value;
  return window.matchMedia?.("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}
function readStoredBoolean(key: string, fallback: boolean) { const value = window.localStorage.getItem(key); return value === "true" ? true : value === "false" ? false : fallback; }
function suppressNonTextControlFocus(event: MouseEvent<HTMLElement>) { const target = event.target; if (!(target instanceof HTMLElement) || isTextEntryTarget(target)) return; const control = target.closest("button, [role='button'], [tabindex]"); if (control instanceof HTMLElement && !control.closest("[data-allow-focus='true']")) event.preventDefault(); }
function blurActiveNonTextControl() { const active = document.activeElement; if (active instanceof HTMLElement && !isTextEntryTarget(active) && active.matches("button, [role='button'], input[type='range'], [tabindex]")) active.blur(); }
function isTextEntryTarget(target: HTMLElement) { if (target.isContentEditable) return true; if (target instanceof HTMLTextAreaElement || target instanceof HTMLSelectElement) return true; return target instanceof HTMLInputElement && target.type !== "range"; }
