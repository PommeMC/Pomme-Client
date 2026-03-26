import { invoke } from "@tauri-apps/api/core";
import { open as openNativeDialog } from "@tauri-apps/plugin-dialog";
import { useState } from "react";
import { HiChevronDown, HiFolder } from "react-icons/hi2";
import { normalizeDirectoryName } from "../../lib/helpers.ts";
import { useDropdown } from "../../lib/hooks.ts";
import { useAppStateContext } from "../../lib/state.ts";
import { GameVersion, Installation } from "../../lib/types.ts";

export type InstallationDialogProps =
  | { editing: false }
  | { editing: true; installation: Installation };

export function InstallationDialog(dialogProps: InstallationDialogProps) {
  const {
    versions,
    invokeCreateInstallation,
    installations,
    activeInstall,
    setActiveInstall,
    setPage,
    setInstallations,
    setVersions,
    setStatus,
    setDownloadProgress,
    setOpenedDialog,
  } = useAppStateContext();

  function createEmptyInstallation(): Installation {
    return {
      id: "",
      icon: null,
      name: "",
      version: versions[0]?.id || "",
      lastPlayed: null,
      directory: "",
      width: 854,
      height: 480,
      canDelete: false,
      createdAt: 0,
    };
  }

  const handleCreateInstallation = async (payload: Installation): Promise<Installation | null> => {
    try {
      const inst = await invokeCreateInstallation(payload);
      setInstallations((prev) => [...prev, inst]);
      if (!activeInstall) setActiveInstall(inst);
      return inst;
    } catch (e) {
      console.log(e);
      setStatus(`Failed to create installation: ${e}`);
      return null;
    }
  };

  const editing = dialogProps.editing;

  const { ref: versionDropdownRef, ...versionDropdown } = useDropdown();
  const [directoryTouched, setDirectoryTouched] = useState(false);
  const [showSnapshots, setShowSnapshots] = useState(false);
  const [editingInstall, setEditingInstall] = useState<Installation>(() =>
    dialogProps.editing ? { ...dialogProps.installation } : createEmptyInstallation(),
  );

  return (
    <div className="dialog">
      <h2 className="dialog-title">{editing ? "Edit Installation" : "New Installation"}</h2>

      <div className="dialog-fields">
        <div className="dialog-field">
          <label>NAME</label>
          <input
            value={editingInstall.name}
            onChange={(e) => {
              const name = e.target.value;
              setEditingInstall((prev) => {
                if (!prev) return prev;
                return {
                  ...prev,
                  name,
                  directory: directoryTouched ? prev.directory : normalizeDirectoryName(name),
                };
              });
            }}
            placeholder="My Installation"
            autoFocus
          />
        </div>
        <div className="dialog-field">
          <label>VERSION</label>
          <div className="custom-select-wrapper" ref={versionDropdownRef}>
            <button className="custom-select" onClick={versionDropdown.toggle} type="button">
              <span>{editingInstall.version}</span>
              <HiChevronDown
                className={`custom-select-arrow ${versionDropdown.isOpen ? "open" : ""}`}
              />
            </button>
            {versionDropdown.isOpen && (
              <div className="custom-select-dropdown">
                <label className="custom-select-toggle">
                  <input
                    type="checkbox"
                    checked={showSnapshots}
                    onChange={(e) => {
                      setShowSnapshots(e.target.checked);
                      invoke<GameVersion[]>("get_versions", {
                        showSnapshots: e.target.checked,
                      }).then(setVersions);
                    }}
                  />
                  <span>Show snapshots</span>
                </label>
                <div className="custom-select-list">
                  {versions.map((v) => (
                    <button
                      key={v.id}
                      className={`custom-select-item ${v.id === editingInstall.version ? "active" : ""}`}
                      onClick={() => {
                        setEditingInstall((prev) => ({ ...prev, version: v.id }));
                        versionDropdown.close();
                      }}
                    >
                      <span>{v.id}</span>
                      {v.version_type !== "release" && (
                        <span className="custom-select-tag">{v.version_type}</span>
                      )}
                    </button>
                  ))}
                </div>
              </div>
            )}
          </div>
        </div>
        <div className="dialog-field">
          <label>GAME DIRECTORY</label>
          <div className="dialog-browse">
            <input
              value={editingInstall.directory}
              onChange={(e) => {
                const dirname = e.target.value;
                setDirectoryTouched(dirname !== "");
                setEditingInstall((prev) => ({
                  ...prev,
                  directory: dirname,
                }));
              }}
              placeholder="default"
            />
            <button
              className="dialog-browse-btn"
              onClick={async () => {
                const path = await openNativeDialog({ directory: true });
                if (path) {
                  setEditingInstall((prev) => ({
                    ...prev,
                    directory: path as string,
                  }));
                }
              }}
            >
              <HiFolder />
            </button>
          </div>
          <span
            className="dialog-field-info"
            hidden={editingInstall.directory === normalizeDirectoryName(editingInstall.directory)}
          >
            Will be created as: /{normalizeDirectoryName(editingInstall.directory)}
          </span>
        </div>
        <div className="dialog-field">
          <label>RESOLUTION</label>
          <div className="dialog-resolution">
            <input
              type="number"
              value={editingInstall.width}
              onChange={(e) =>
                setEditingInstall((prev) => ({
                  ...prev,
                  width: parseInt(e.target.value) || 854,
                }))
              }
              placeholder="854"
            />
            <span className="dialog-resolution-x">×</span>
            <input
              type="number"
              value={editingInstall.height}
              onChange={(e) =>
                setEditingInstall((prev) => ({
                  ...prev,
                  height: parseInt(e.target.value) || 480,
                }))
              }
              placeholder="480"
            />
          </div>
        </div>
      </div>

      <div className="dialog-actions">
        <button className="dialog-cancel" onClick={() => setOpenedDialog(null)}>
          Cancel
        </button>
        <button
          className="dialog-save"
          onClick={async () => {
            const editedInstall: Installation = {
              ...editingInstall,
              name: editingInstall.name || "My Installation",
              icon: editingInstall.icon || null,
              version: editingInstall.version || versions[0]?.id || "",
              width: editingInstall.width || 854,
              height: editingInstall.height || 480,
            };
            editedInstall.directory = normalizeDirectoryName(
              editingInstall.directory || editedInstall.name,
            );

            if (editingInstall.version === "") {
              console.error("Invalid version");
              return;
            }

            // TODO: edit

            if (!editing) {
              const install = await handleCreateInstallation(editedInstall);
              if (!install) return;

              setOpenedDialog(null);
              setPage("home");
              setDownloadProgress({
                downloaded: 0,
                total: 1,
                status: "Starting install...",
              });

              try {
                await invoke("ensure_assets", { version: install.version });
                setStatus(`${install.name} ready`);
              } catch (e) {
                setStatus(`Install failed: ${e}`);
              }

              setDownloadProgress(null);
              if (installations.length === 0) {
                setActiveInstall(install);
              }
              setTimeout(() => setStatus(""), 3000);
            }
          }}
        >
          {!editing ? "Install" : "Save"}
        </button>
      </div>
    </div>
  );
}
