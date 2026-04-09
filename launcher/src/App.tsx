import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useCallback, useEffect, useRef } from "react";
import { commands } from "./bindings";
import Navbar from "./components/Navbar";
import Titlebar from "./components/Titlebar";
import AlertDialog from "./components/dialogs/AlertDialog.tsx";
import { ConfirmDialog } from "./components/dialogs/ConfirmDialog.tsx";
import { InstallationDialog } from "./components/dialogs/InstallationDialog.tsx";
import { useAppStateContext } from "./lib/state";
import { AuthAccount, DownloadProgress, GameVersion, PatchNote } from "./lib/types";
import FriendsPage from "./pages/Friends";
import Homepage from "./pages/Home";
import InstallationsPage from "./pages/Installations";
import ModsPage from "./pages/Mods";
import NewsPage from "./pages/News";
import ServersPage from "./pages/Servers";
import SettingsPage from "./pages/Settings";

function App() {
  const {
    account,
    accountDropdown,
    page,
    setPage,
    accounts,
    setAccounts,
    setActiveIndex,
    server,
    setVersions,
    downloadedVersions,
    setLaunchingStatus,
    setAuthLoading,
    setStatus,
    setNews,
    setSkinUrl,
    setSelectedNote,
    setDownloadProgress,
    openedDialog,
    setOpenedDialog,
    launcherSettings,
    activeInstall,
    setActiveInstall,
    setInstallations,
    setDownloadedVersions,
  } = useAppStateContext();

  const { setIsOpen: setAccountDropdownOpen } = accountDropdown;

  const openPatchNote = useCallback(
    async (note: PatchNote) => {
      try {
        const body = await invoke<string>("get_patch_content", {
          contentPath: note.content_path,
        });
        setSelectedNote({
          title: note.title,
          body,
          image_url: note.image_url,
        });
        setPage("news");
      } catch (e) {
        console.error("Failed to fetch content:", e);
      }
    },
    [setPage, setSelectedNote],
  );

  const loadSkin = useCallback(
    (uuid: string) => {
      invoke<string>("get_skin_url", { uuid })
        .then(setSkinUrl)
        .catch(() => setSkinUrl(null));
    },
    [setSkinUrl],
  );

  useEffect(() => {
    invoke<AuthAccount[]>("get_all_accounts").then((accs) => {
      if (accs.length > 0) {
        setAccounts(accs);
        setActiveIndex(0);
        loadSkin(accs[0].uuid);
      }
    });
    invoke<PatchNote[]>("get_patch_notes", { count: 6 })
      .then(setNews)
      .catch((e) => console.error("Failed to fetch news:", e));
    invoke<GameVersion[]>("get_versions", { showSnapshots: false })
      .then(setVersions)
      .catch((e) => console.error("Failed to fetch versions:", e));
  }, [loadSkin, setAccounts, setActiveIndex, setNews, setVersions]);

  useEffect(() => {
    requestAnimationFrame(() => getCurrentWindow().show());
  }, []);

  useEffect(() => {
    const unlisten = listen<DownloadProgress>("download-progress", (event) => {
      setDownloadProgress(event.payload);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [setDownloadProgress]);

  const startAddAccount = useCallback(async () => {
    setAccountDropdownOpen(false);
    setAuthLoading(true);
    setStatus("Signing in via Microsoft...");
    try {
      const acc = await invoke<AuthAccount>("add_account");
      setAccounts((prev) => {
        const filtered = prev.filter((a) => a.uuid !== acc.uuid);
        return [...filtered, acc];
      });
      setActiveIndex(accounts.filter((a) => a.uuid !== acc.uuid).length);
      loadSkin(acc.uuid);
      setStatus(`Signed in as ${acc.username}`);
    } catch (e) {
      setStatus(`Auth failed: ${e}`);
    }
    setAuthLoading(false);
  }, [
    accounts,
    loadSkin,
    setAccountDropdownOpen,
    setAccounts,
    setActiveIndex,
    setAuthLoading,
    setStatus,
  ]);

  const switchAccount = useCallback(
    (index: number) => {
      setActiveIndex(index);
      setAccountDropdownOpen(false);
      if (accounts[index]) {
        loadSkin(accounts[index].uuid);
      }
    },
    [accounts, loadSkin, setAccountDropdownOpen, setActiveIndex],
  );

  const removeAccount = useCallback(
    (uuid: string) => {
      invoke("remove_account", { uuid }).catch((e) =>
        console.error("Failed to remove account:", e),
      );
      setAccounts((prev) => prev.filter((a) => a.uuid !== uuid));
      setActiveIndex(0);
      setAccountDropdownOpen(false);
      setSkinUrl(null);
    },
    [setAccountDropdownOpen, setAccounts, setActiveIndex, setSkinUrl],
  );

  const ensureAssets = useCallback(
    async (version: string) => {
      setStatus("Checking assets...");
      try {
        if (downloadedVersions.has(version)) {
          setLaunchingStatus("checking_assets");
        } else {
          setLaunchingStatus("installing");
        }
        await invoke("ensure_assets", { version });
        setDownloadedVersions((prev) => new Set([...prev, version]));
        return true;
      } catch (e) {
        setStatus(`${e}`);
        return false;
      } finally {
        setStatus("");
        setDownloadProgress(null);
        setLaunchingStatus(null);
      }
    },
    [downloadedVersions, setDownloadedVersions, setLaunchingStatus, setStatus, setDownloadProgress],
  );

  const handleLaunch = useCallback(async () => {
    if (!activeInstall) {
      setStatus("No installation selected");
      setTimeout(() => setStatus(""), 3000);
      return;
    }

    if (!(await ensureAssets(activeInstall.version))) {
      return;
    }

    const unlisten = await listen<{
      code: number | null;
      signal: number | null;
      last_line: string | null;
    }>("game_exited", (event) => {
      const { code, signal, last_line } = event.payload;
      const SIGNAL_NAMES: Record<number, string> = {
        4: "SIGILL",
        6: "SIGABRT",
        7: "SIGBUS",
        8: "SIGFPE",
        11: "SIGSEGV",
        16: "SIGSTKFLT",
      };
      const reason =
        signal !== null ? `signal ${SIGNAL_NAMES[signal] ?? signal}` : `code ${code ?? "unknown"}`;
      setOpenedDialog({
        name: "alert_dialog",
        props: {
          title: `Game exited (${reason})`,
          message: last_line ?? "The game exited unexpectedly.",
        },
      });
      unlisten();
    });

    try {
      setLaunchingStatus("launching");
      setStatus("Launching Pomme...");
      const result = await invoke<string>("launch_game", {
        uuid: account?.uuid || null,
        server: server || null,
        debugEnabled: launcherSettings.launchWithConsole || null,
        version: activeInstall.version,
        install_id: activeInstall.id,
      });
      setStatus(result);
    } catch (e) {
      setStatus(`${e}`);
    } finally {
      setDownloadProgress(null);
      setLaunchingStatus(null);
      setTimeout(() => {
        setStatus("");
      }, 3000);
    }
  }, [
    setOpenedDialog,
    ensureAssets,
    activeInstall,
    setLaunchingStatus,
    setStatus,
    setDownloadProgress,
    account?.uuid,
    server,
    launcherSettings.launchWithConsole,
  ]);

  const dialogDragStartedInside = useRef(false);

  useEffect(() => {
    commands.loadInstallations().then((res) => {
      if (res.ok) {
        setInstallations(res.value);
        setActiveInstall((prev) => prev ?? res.value[0]);
      } else {
        setStatus("Failed to load installations: " + res.error);
      }
    });
  }, [setInstallations, setActiveInstall, setStatus]);

  useEffect(() => {
    commands.getDownloadedVersions().then((versions) => {
      setDownloadedVersions((prev) => new Set([...prev, ...versions]));
    });
  }, [setDownloadedVersions, setStatus]);

  return (
    <div className="app">
      <Titlebar />

      <div className="layout">
        <Navbar
          startAddAccount={startAddAccount}
          switchAccount={switchAccount}
          removeAccount={removeAccount}
        />

        <main className="content">
          {page === "home" && (
            <Homepage handleLaunch={handleLaunch} openPatchNote={openPatchNote} />
          )}

          {page === "installations" && (
            <InstallationsPage handleLaunch={handleLaunch} ensureAssets={ensureAssets} />
          )}

          {page === "news" && <NewsPage openPatchNote={openPatchNote} />}

          {page === "servers" && <ServersPage handleLaunch={handleLaunch} />}

          {page === "friends" && <FriendsPage />}

          {page === "mods" && <ModsPage />}

          {page === "settings" && <SettingsPage />}
        </main>
      </div>

      {openedDialog !== null && (
        <div
          className="dialog-overlay"
          onMouseDown={(e) => {
            dialogDragStartedInside.current = e.target !== e.currentTarget;
          }}
          onClick={(e) => {
            if (e.target === e.currentTarget && !dialogDragStartedInside.current) {
              setOpenedDialog(null);
            }
          }}
        >
          {openedDialog.name === "installation" && <InstallationDialog {...openedDialog.props} />}
          {openedDialog.name === "confirm_dialog" && <ConfirmDialog {...openedDialog.props} />}
          {openedDialog.name === "alert_dialog" && <AlertDialog {...openedDialog.props} />}
        </div>
      )}
    </div>
  );
}

export default App;
