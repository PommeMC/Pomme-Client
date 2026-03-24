import {createElement, createContext, useContext, useState, ReactNode, useEffect} from "react";
import { Page, AuthAccount, Installation, GameVersion, PatchNote, LauncherSettings } from "./types";
import {invoke} from "@tauri-apps/api/core";

const useLauncherSettings = () => {
  const [launcherSettings, setLauncherSettings] = useState<LauncherSettings>({
    language: "English",
    keepLauncherOpen: true,
    launchWithConsole: false,
  });

  useEffect(() => {
    invoke<LauncherSettings>("load_launcher_settings")
      .then((settings) => setLauncherSettings(settings))
      .catch(console.error);
  }, []);

  return launcherSettings;
}
const useAppState = () => {
  const [page, setPage] = useState<Page>("home");
  const [accounts, setAccounts] = useState<AuthAccount[]>([]);
  const [activeIndex, setActiveIndex] = useState(0);
  const [accountDropdownOpen, setAccountDropdownOpen] = useState(false);

  const [server] = useState("");
  const [keepOpen, setKeepOpen] = useState(true);
  const [useConsole, setUseConsole] = useState(false);

  const [modView, setModView] = useState<"list" | "grid">("list");
  const [modSearch, setModSearch] = useState("");
  const [modFilter, setModFilter] = useState("all");
  const [installations, setInstallations] = useState<Installation[]>([
    {
      id: "default",
      name: "Latest Release",
      version: "1.21.11",
      lastPlayed: "Today",
      directory: "default",
      width: 854,
      height: 480,
    },
  ]);
  const [activeInstall, setActiveInstall] = useState("default");
  const [editingInstall, setEditingInstall] = useState<Installation | null>(null);
  const [dialogVersionOpen, setDialogVersionOpen] = useState(false);
  const selectedVersion =
    installations.find((i) => i.id === activeInstall)?.version || "1.21.11";
  const [versions, setVersions] = useState<GameVersion[]>([]);
  const [showSnapshots, setShowSnapshots] = useState(false);
  const [launching, setLaunching] = useState(false);
  const [authLoading, setAuthLoading] = useState(false);
  const [status, setStatus] = useState("");
  const [news, setNews] = useState<PatchNote[]>([]);
  const [skinUrl, setSkinUrl] = useState<string | null>(null);

  const account = accounts[activeIndex] || null;
  const username = account?.username || "Steve";
  const [selectedNote, setSelectedNote] = useState<{
    title: string;
    body: string;
  } | null>(null);

  const launcherSettings= useLauncherSettings();
  console.log(launcherSettings);

  return {
    account,
    page,
    setPage,
    accounts,
    setAccounts,
    activeIndex,
    setActiveIndex,
    accountDropdownOpen,
    setAccountDropdownOpen,
    server,
    keepOpen,
    setKeepOpen,
    useConsole,
    setUseConsole,
    modView,
    setModView,
    modSearch,
    setModSearch,
    modFilter,
    setModFilter,
    installations,
    setInstallations,
    activeInstall,
    setActiveInstall,
    editingInstall,
    setEditingInstall,
    dialogVersionOpen,
    setDialogVersionOpen,
    selectedVersion,
    versions,
    setVersions,
    showSnapshots,
    setShowSnapshots,
    launching,
    setLaunching,
    authLoading,
    setAuthLoading,
    status,
    setStatus,
    news,
    setNews,
    skinUrl,
    setSkinUrl,
    selectedNote,
    setSelectedNote,
    username,

    launcherSettings,
  };
};

type AppState = ReturnType<typeof useAppState>;

const AppStateContext = createContext<AppState | null>(null);

export function AppStateProvider({children}: { children: ReactNode }) {
  const state = useAppState();
  return createElement(AppStateContext.Provider, {value: state}, children);
}

export function useAppStateContext(): AppState {
  const ctx = useContext(AppStateContext);
  if (!ctx) {
    throw new Error("useAppStateContext must be used within an AppStateProvider");
  }
  return ctx;
}
