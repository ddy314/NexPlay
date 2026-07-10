import { motion } from "framer-motion";
import {
  BarChart3,
  Compass,
  DownloadCloud,
  Home,
  Library,
  Moon,
  PanelLeftClose,
  PanelLeftOpen,
  Search,
  Settings,
  Sun,
} from "lucide-react";
import type { ReactNode } from "react";
import { appleSpringSoft } from "./motion";
import { cn } from "./utils/cn";

export type Route = "home" | "discover" | "library" | "downloads" | "insights" | "settings";

const items: { id: Exclude<Route, "settings">; label: string; hint: string; icon: ReactNode }[] = [
  { id: "home", label: "首页", hint: "下一步", icon: <Home size={20} /> },
  { id: "discover", label: "发现", hint: "找新内容", icon: <Compass size={20} /> },
  { id: "library", label: "媒体库", hint: "你的片库", icon: <Library size={20} /> },
  { id: "downloads", label: "下载", hint: "获取进度", icon: <DownloadCloud size={20} /> },
  { id: "insights", label: "洞察", hint: "观看趋势", icon: <BarChart3 size={20} /> },
];

export function NavRail({
  route,
  onRoute,
  onSearch,
  theme,
  onToggleTheme,
  collapsed,
  onToggleCollapsed,
}: {
  route: Route;
  onRoute: (route: Route) => void;
  onSearch: () => void;
  theme: "light" | "dark";
  onToggleTheme: () => void;
  collapsed: boolean;
  onToggleCollapsed: () => void;
}) {
  return (
    <aside className={cn("nx-nav", collapsed && "is-collapsed")} aria-label="主导航">
      <div className="nx-brand">
        <span className="nx-brand-mark">N</span>
        <span className="nx-brand-name">NexPlay</span>
        <button className="nx-nav-collapse" onClick={onToggleCollapsed} aria-label={collapsed ? "展开导航" : "收起导航"}>
          {collapsed ? <PanelLeftOpen size={17} /> : <PanelLeftClose size={17} />}
        </button>
      </div>

      <button type="button" className="nx-search-entry" onClick={onSearch} aria-label="全局搜索">
        <Search size={19} />
        <span>搜索</span>
        <kbd>⌘ K</kbd>
      </button>

      <nav className="nx-nav-items">
        {items.map((item, index) => {
          const active = route === item.id;
          return (
            <motion.button
              type="button"
              key={item.id}
              className={cn("nx-nav-item", active && "is-active")}
              onClick={() => onRoute(item.id)}
              whileTap={{ scale: 0.98 }}
              transition={appleSpringSoft}
              title={collapsed ? item.label : undefined}
              aria-current={active ? "page" : undefined}
            >
              <span className="nx-nav-index">{String(index + 1).padStart(2, "0")}</span>
              <span className="nx-nav-icon">{item.icon}</span>
              <span className="nx-nav-copy">
                <strong>{item.label}</strong>
                <small>{item.hint}</small>
              </span>
              {active && <motion.span layoutId="nx-nav-active" className="nx-nav-active" />}
            </motion.button>
          );
        })}
      </nav>

      <div className="nx-nav-footer">
        <button type="button" className="nx-nav-utility" onClick={onToggleTheme} aria-label={theme === "light" ? "使用深色模式" : "使用浅色模式"}>
          {theme === "light" ? <Moon size={19} /> : <Sun size={19} />}
          <span>{theme === "light" ? "深色" : "浅色"}</span>
        </button>
        <button type="button" className={cn("nx-nav-utility", route === "settings" && "is-active")} onClick={() => onRoute("settings")}>
          <Settings size={19} />
          <span>设置</span>
        </button>
      </div>
    </aside>
  );
}
