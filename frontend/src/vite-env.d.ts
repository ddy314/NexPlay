/// <reference types="vite/client" />

interface Window {
  nexplay?: {
    appName: string;
    getSnapshot: () => Promise<import("./backend").BackendSnapshot>;
    scanLibrary: () => Promise<import("./backend").ScanResponse>;
    getSettings: () => Promise<import("./backend").EditableSettings>;
    saveSettings: (settings: import("./backend").EditableSettings) => Promise<import("./backend").EditableSettings>;
    openMedia: (mediaId: number) => Promise<{ opened: boolean }>;
    onBackendEvent: (callback: (event: import("./backend").BackendEvent) => void) => () => void;
    resolveAssetUrl: (value: string) => string;
  };
}
