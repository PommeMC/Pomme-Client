import {
  HiPlay,
  HiCube,
  HiDocumentDuplicate,
  HiFolder,
  HiPencil,
  HiPlus,
  HiTrash,
} from "react-icons/hi2";
import { useAppStateContext } from "../lib/state";

export default function InstallationsPage() {
  const {
    activeInstall,
    setActiveInstall,
    setEditingInstall,
    installations,
    setInstallations,
    setPage,
  } = useAppStateContext();

  return (
    <div className="page installs-page">
      <div className="installs-header">
        <h2 className="installs-heading">INSTALLATIONS</h2>
        <button
          className="installs-new-btn"
          onClick={() =>
            setEditingInstall({
              id: "",
              name: "",
              version: "1.21.11",
              lastPlayed: "Never",
              directory: "",
              width: 854,
              height: 480,
            })
          }
        >
          <HiPlus /> New Installation
        </button>
      </div>

      <div className="installs-list">
        {installations.map((inst) => (
          <div
            key={inst.id}
            className={`install-card ${inst.id === activeInstall ? "active" : ""}`}
          >
            <div className="install-card-icon">
              <HiCube />
            </div>
            <div className="install-card-info">
              <span className="install-card-name">{inst.name}</span>
              <span className="install-card-version">{inst.version}</span>
            </div>
            <span className="install-card-played">{inst.lastPlayed}</span>
            <button
              className="install-play-btn"
              onClick={() => {
                setActiveInstall(inst.id);
                setPage("home");
              }}
            >
              <HiPlay /> Play
            </button>
            <button
              className="install-folder-btn"
              onClick={() => console.log("Open:", inst.directory)}
            >
              <HiFolder />
            </button>
            <div className="install-card-actions">
              <button
                className="install-action-btn"
                onClick={() => setEditingInstall({ ...inst })}
                title="Edit"
              >
                <HiPencil />
              </button>
              <button
                className="install-action-btn"
                title="Duplicate"
                onClick={() => {
                  const dup = {
                    ...inst,
                    id: Date.now().toString(36),
                    name: `${inst.name} (copy)`,
                  };
                  setInstallations((prev) => [...prev, dup]);
                }}
              >
                <HiDocumentDuplicate />
              </button>
              {inst.id !== "default" && (
                <button
                  className="install-action-btn delete"
                  title="Delete"
                  onClick={() => {
                    setInstallations((prev) => prev.filter((i) => i.id !== inst.id));
                    if (activeInstall === inst.id) {
                      setActiveInstall("default");
                    }
                  }}
                >
                  <HiTrash />
                </button>
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
