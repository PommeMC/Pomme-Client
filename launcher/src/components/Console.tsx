import { invoke } from "@tauri-apps/api/core";
import "../styles.css";
import Titlebar from "./Titlebar";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState, useRef } from "react";
import { HiClipboardCopy } from "react-icons/hi";
import { HiXMark } from "react-icons/hi2";

const getLogs: () => Promise<string[]> = async () => invoke("get_client_logs");

interface ConsoleMessage {
  type: "message" | "reset";
  val?: string;
}

interface Filter {
  info_enabled: boolean;
  warn_enabled: boolean;
  debug_enabled: boolean;
  error_enabled: boolean;
  search?: string;
}

const Log = ({ log, filter }: { log: string; filter: Filter }) => {
  let splitIndex = log.indexOf("]");
  if (splitIndex === -1) {
    return <p className="console-log">{log}</p>;
  }
  let start_str = log.slice(0, splitIndex);
  let message = log.slice(splitIndex + 1);

  let type = "";

  for (const tag of ["INFO", "WARN", "DEBUG", "ERROR"]) {
    if (start_str.includes(tag)) {
      type = tag;
    }
  }

  let render = false;
  if (
    (type === "INFO" && filter.info_enabled) ||
    (type === "WARN" && filter.warn_enabled) ||
    (type === "DEBUG" && filter.debug_enabled) ||
    (type === "ERROR" && filter.error_enabled)
  )
    render = true;

  if (filter.search && !log.includes(filter.search)) render = false;

  if (!render) return null;

  if (type === "") {
    return <p className="console-log">{log}</p>;
  }

  return (
    <p className="console-log">
      <span className={`console-text-${type.toLowerCase()}`}>{start_str}]</span>
      {message}
    </p>
  );
};

const FILTER_LEVELS = [
  { key: "info_enabled" as const, label: "INFO", type: "info" },
  { key: "warn_enabled" as const, label: "WARN", type: "warning" },
  { key: "debug_enabled" as const, label: "DEBUG", type: "debug" },
  { key: "error_enabled" as const, label: "ERROR", type: "error" },
];

const FilterChip = ({
  label,
  type,
  active,
  onToggle,
}: {
  label: string;
  type: string;
  active: boolean;
  onToggle: () => void;
}) => (
  <button onClick={onToggle} className={`console-chip-button ${type} ${active ? "active" : ""}`}>
    {label}
  </button>
);

export default function Console() {
  const [logs, setLogs] = useState<string[]>([]);
  const [filter, setFilter] = useState<Filter>({
    info_enabled: true,
    warn_enabled: true,
    debug_enabled: true,
    error_enabled: true,
  });

  const bottomRef = useRef<HTMLDivElement | null>(null);
  const searchRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    let eventsRegistered = false;

    const initListener = async () => {
      const initialLogs = await getLogs();

      if (eventsRegistered) return;

      setLogs(initialLogs);

      const unlistenFn = await listen<ConsoleMessage>("console_message", (event) => {
        let recv = event.payload;
        switch (recv.type) {
          case "message":
            setLogs((prevLogs) => {
              const updatedLogs = [...prevLogs, recv.val as string];
              const maxLogs = 10_000;

              if (updatedLogs.length > maxLogs) {
                return updatedLogs.slice(1);
              }
              return updatedLogs;
            });

            break;

          case "reset":
            setLogs([]);
            break;
          default:
            console.error(`Received bad event type '${recv.type}'.`, recv);
        }
      });

      if (eventsRegistered) {
        unlistenFn();
        return;
      }

      unlisten = unlistenFn;
    };

    initListener();

    return () => {
      eventsRegistered = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs]);

  const copyLogs = () => {
    let copyText = logs.join("\n");
    navigator.clipboard.writeText(copyText);
  };

  return (
    <div className="app">
      <Titlebar name="POMC Debugger" />
      <div className="console-holder">
        <div className="console">
          <div className="console-scroll">
            {logs.map((object, key) => (
              <Log log={object} key={key} filter={filter} />
            ))}
            <div ref={bottomRef} />{" "}
          </div>
        </div>
        <div className="console-bottom-bar">
          <button className="console-copy-button" onClick={copyLogs}>
            <HiClipboardCopy className="clipboard-icon" />
          </button>
          {FILTER_LEVELS.map(({ key, label, type }) => (
            <FilterChip
              key={key}
              label={label}
              type={type}
              active={filter[key]}
              onToggle={() => setFilter((prev) => ({ ...prev, [key]: !prev[key] }))}
            />
          ))}
          <input
            placeholder="Search..."
            type="text"
            ref={searchRef}
            className="console-search"
            onInput={(e) => {
              const value = e.currentTarget.value;
              setFilter((prev) => ({ ...prev, search: value }));
            }}
          />
          <button
            className="console-clear-search-button"
            onClick={() => {
              if (searchRef.current) {
                searchRef.current.value = "";
              }
              setFilter((prev) => ({ ...prev, search: undefined }));
            }}
          >
            <HiXMark />
          </button>
        </div>
      </div>
    </div>
  );
}
