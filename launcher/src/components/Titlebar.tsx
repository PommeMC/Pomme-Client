import { getCurrentWindow } from "@tauri-apps/api/window";
import { HiCube, HiMinus, HiSquare2Stack, HiXMark } from "react-icons/hi2";

export default function Titlebar({ name }: { name?: string } = { name: "POMC Launcher" }) {
  const appWindow = getCurrentWindow();

  const minimize = () => {
    appWindow.minimize();
  };
  const toggleMaximize = () => {
    appWindow.toggleMaximize();
  };
  const close = () => {
    appWindow.close();
  };

  return (
    <div className="titlebar" data-tauri-drag-region>
      <div className="titlebar-left" data-tauri-drag-region>
        <span className="titlebar-icon">
          <HiCube />
        </span>
      </div>
      <span className="titlebar-title" data-tauri-drag-region>
        {name}
      </span>
      <div className="titlebar-controls">
        <button className="tb-btn" onClick={minimize}>
          <HiMinus />
        </button>
        <button className="tb-btn" onClick={toggleMaximize}>
          <HiSquare2Stack />
        </button>
        <button className="tb-btn tb-close" onClick={close}>
          <HiXMark />
        </button>
      </div>
    </div>
  );
}
