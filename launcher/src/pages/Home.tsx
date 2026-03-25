import { useState } from "react";
import { HiPlay, HiChevronDown, HiCube } from "react-icons/hi2";
import { PatchNote } from "../lib/types";
import { useAppStateContext } from "../lib/state";
import SkinRunner from "../components/SkinRunner";

interface HomepageProps {
  handleLaunch: () => Promise<void>;
  openPatchNote: (item: PatchNote) => Promise<void>;
}

export default function Homepage({ handleLaunch, openPatchNote }: HomepageProps) {
  const [versionDropdownOpen, setVersionDropdownOpen] = useState(false);

  const {
    launching,
    installations,
    selectedVersion,
    activeInstall,
    setActiveInstall,
    news,
    status,
    downloadProgress,
    skinUrl,
  } = useAppStateContext();

  return (
    <div className="page home-page">
      <div className="hero-banner">
        <div className="hero-overlay" />
        <div className="hero-content">
          <h1 className="hero-title">POMC</h1>
          <p className="hero-subtitle">RUST-NATIVE MINECRAFT CLIENT</p>
        </div>
      </div>

      <div className="launch-bar">
        <button
          className={`play-button ${launching ? "launching" : ""}`}
          onClick={handleLaunch}
          disabled={launching}
        >
          <HiPlay className="play-icon" />
          <span className="play-text">{launching ? "LAUNCHING..." : "PLAY"}</span>
        </button>
      </div>

      <div className="version-badge-wrapper">
        {versionDropdownOpen && (
          <div className="click-away" onClick={() => setVersionDropdownOpen(false)} />
        )}
        <button
          className="version-badge"
          onClick={() => setVersionDropdownOpen(!versionDropdownOpen)}
        >
          <HiCube className="version-badge-icon" />
          <span>{selectedVersion}</span>
          <HiChevronDown className={`version-badge-arrow ${versionDropdownOpen ? "open" : ""}`} />
        </button>
        {versionDropdownOpen && (
          <div className="version-dropdown">
            <div className="version-list">
              {installations.map((inst) => (
                <button
                  key={inst.id}
                  className={`version-item ${inst.id === activeInstall ? "active" : ""}`}
                  onClick={() => {
                    setActiveInstall(inst.id);
                    setVersionDropdownOpen(false);
                  }}
                >
                  <span className="version-item-id">{inst.name}</span>
                  <span className="version-item-type">{inst.version}</span>
                </button>
              ))}
            </div>
          </div>
        )}
      </div>

      {downloadProgress && (
        <div className="download-progress">
          <div className="download-progress-text">{downloadProgress.status}</div>
          <div className="download-progress-bar">
            <SkinRunner
              skinUrl={skinUrl}
              progress={
                downloadProgress.total > 0
                  ? downloadProgress.downloaded / downloadProgress.total
                  : 0
              }
            />
            <div
              className="download-progress-fill"
              style={{
                width:
                  downloadProgress.total > 0
                    ? `${(downloadProgress.downloaded / downloadProgress.total) * 100}%`
                    : "0%",
              }}
            />
          </div>
        </div>
      )}
      {!downloadProgress && status && <div className="status-toast">{status}</div>}

      <div className="news-section">
        <h2 className="news-heading">LATEST NEWS</h2>
        <div className="news-grid">
          {news.slice(0, 3).map((item) => (
            <div className="news-card" key={item.version} onClick={() => openPatchNote(item)}>
              <div className="news-card-img" style={{ backgroundImage: `url(${item.image_url})` }}>
                <span className="news-type-badge">{item.entry_type}</span>
              </div>
              <div className="news-card-body">
                <span className="news-date">{item.date.replace(/-/g, ".")}</span>
                <h3 className="news-title">{item.title}</h3>
                <p className="news-desc">{item.summary}</p>
              </div>
            </div>
          ))}
          {news.length === 0 && <p className="news-loading">Loading patch notes...</p>}
        </div>
      </div>
    </div>
  );
}
