import { invoke } from "@tauri-apps/api/core";
import { listen, once } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useCallback, useEffect, useRef } from "react";
import Navbar from "./components/Navbar";
import Titlebar from "./components/Titlebar";
import AlertDialog from "./components/dialogs/AlertDialog.tsx";
import { ConfirmDialog } from "./components/dialogs/ConfirmDialog.tsx";
import { InstallationDialog } from "./components/dialogs/InstallationDialog.tsx";
import { ServerDialog } from "./components/dialogs/ServerDialog.tsx";
import { useAppStateContext } from "./lib/state";
import {
  AuthAccount,
  DownloadProgress,
  GameVersion,
  handleLaunchType,
  Installation,
  InstallationError,
  PatchNote,
} from "./lib/types";
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
    invokeCreateInstallation,
    invokeDeleteInstallation,
    invokeDuplicateInstallation,
    invokeEditInstallation,
    activeInstall,
    setActiveInstall,
    installations,
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
    async (version: string): Promise<Error | null> => {
      setStatus("Checking assets...");
      try {
        if (downloadedVersions.has(version)) {
          setLaunchingStatus("checking_assets");
        } else {
          setLaunchingStatus("installing");
        }

        await invoke("ensure_assets", { version });

        setDownloadedVersions((prev) => new Set([...prev, version]));
        return null;
      } catch (e) {
        return e instanceof Error ? e : new Error(String(e));
      } finally {
        setStatus("");
        setDownloadProgress(null);
        setLaunchingStatus(null);
      }
    },
    [downloadedVersions, setDownloadedVersions, setLaunchingStatus, setStatus, setDownloadProgress],
  );

  const handleLaunch: handleLaunchType = useCallback(
    async (serverIp?: string, serverVersion?: string) => {
      let currentInstall = activeInstall;

      if (serverVersion && serverIp) {
        const candidate =
          installations.find((i) => i.id === "latest-release" || i.id === "latest-snapshot") ??
          null;

        if (candidate) {
          currentInstall = { ...candidate, version: serverVersion };
        }
      }

      if (!currentInstall) {
        setStatus("No installation selected");
        setTimeout(() => setStatus(""), 3000);
        return;
      }

      const err = await ensureAssets(currentInstall.version);
      if (err instanceof Error) {
        setOpenedDialog({
          name: "alert_dialog",
          props: {
            title: "Failed to download assets",
            message: `Failed to download assets for ${currentInstall.version}:\n${err.message}`,
          },
        });
        return;
      }

      await once<{
        code: number | null;
        signal: number | null;
        last_lines: string[] | null;
      }>("game_exited", (event) => {
        const { code, signal, last_lines } = event.payload;
        if (code === 0) {
          return;
        }
        const SIGNAL_NAMES: Record<number, string> = {
          4: "SIGILL",
          6: "SIGABRT",
          7: "SIGBUS",
          8: "SIGFPE",
          11: "SIGSEGV",
          16: "SIGSTKFLT",
        };
        const reason =
          signal !== null
            ? `signal ${SIGNAL_NAMES[signal] ?? signal}`
            : `code ${code ?? "unknown"}`;
        const message =
          code === 1 && last_lines && last_lines.length > 0
            ? last_lines.map((line, i) => `${i + 1}: ${line}`).join("\n")
            : "The game exited unexpectedly.";

        setOpenedDialog({
          name: "alert_dialog",
          props: {
            title: `Game exited (${reason})`,
            message,
          },
        });
      });

      try {
        setLaunchingStatus("launching");
        setStatus("Launching Pomme...");
        const result = await invoke<string>("launch_game", {
          installId: currentInstall.id,
          uuid: account?.uuid || null,
          serverIp: serverIp || null,
          overrideVersion: serverVersion || null,
          debugEnabled: launcherSettings.launchWithConsole || null,
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
    },
    [
      installations,
      ensureAssets,
      activeInstall,
      setLaunchingStatus,
      setStatus,
      setDownloadProgress,
      setOpenedDialog,
      account?.uuid,
      launcherSettings.launchWithConsole,
    ],
  );

  const dialogDragStartedInside = useRef(false);

  const createInstallation = async (
    payload: Installation,
  ): Promise<[Installation, null] | [null, InstallationError]> => {
    try {
      const inst = await invokeCreateInstallation(payload);
      setInstallations((prev) => [...prev, inst]);
      return [inst, null];
    } catch (e) {
      console.error("Failed to create installation", e);
      return [null, e as InstallationError];
    }
  };

  const deleteInstallation = async (install_id: string): Promise<null | InstallationError> => {
    try {
      await invokeDeleteInstallation(install_id);
      setInstallations((prev) => prev.filter((inst) => inst.id !== install_id));
      return null;
    } catch (e) {
      console.error("Failed to delete installation", e);
      return e as InstallationError;
    }
  };

  const duplicateInstallation = async (
    install_id: string,
    new_payload: Installation,
  ): Promise<[Installation, null] | [null, InstallationError]> => {
    try {
      const inst = await invokeDuplicateInstallation(install_id, new_payload);
      setInstallations((prev) => [...prev, inst]);
      return [inst, null];
    } catch (e) {
      console.error("Failed to duplicate installation", e);
      return [null, e as InstallationError];
    }
  };

  const editInstallation = async (
    install_id: string,
    new_payload: Installation,
  ): Promise<null | InstallationError> => {
    try {
      const inst = await invokeEditInstallation(install_id, new_payload);
      setInstallations((prev) => prev.map((i) => (i.id === install_id ? inst : i)));
      return null;
    } catch (e) {
      console.error("Failed to edit installation", e);
      return e as InstallationError;
    }
  };

  useEffect(() => {
    invoke<Installation[]>("load_installations")
      .then((installs) => {
        setInstallations(installs);
        setActiveInstall((prev) => prev ?? installs[0]);
      })
      .catch((e) => setStatus("Failed to load installations: " + e));
  }, [setInstallations, setActiveInstall, setStatus]);

  useEffect(() => {
    invoke<string[]>("get_downloaded_versions")
      .then((versions) => {
        setDownloadedVersions((prev) => new Set([...prev, ...versions]));
      })
      .catch((e) => console.error("Failed to load downloaded versions: " + e));
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
            <InstallationsPage
              deleteInstallation={deleteInstallation}
              handleLaunch={handleLaunch}
              ensureAssets={ensureAssets}
            />
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
          {openedDialog.name === "installation_dialog" && (
            <InstallationDialog
              {...openedDialog.props}
              createInstallation={createInstallation}
              duplicateInstallation={duplicateInstallation}
              editInstallation={editInstallation}
            />
          )}
          {openedDialog.name === "server_dialog" && <ServerDialog {...openedDialog.props} />}
          {openedDialog.name === "confirm_dialog" && <ConfirmDialog {...openedDialog.props} />}
          {openedDialog.name === "alert_dialog" && <AlertDialog {...openedDialog.props} />}
        </div>
      )}
    </div>
  );
}

export default App;
