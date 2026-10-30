import type { FreacInfo } from "../types";

interface FreacControlsProps {
  path: string;
  onPathChange: (value: string) => void;
  probe: FreacInfo | null;
  onProbe: () => void;
  onBrowse?: () => void;
  version: string;
  onVersionChange: (value: string) => void;
  onInstall: () => void;
  installing: boolean;
}

export function FreacControls({
  path,
  onPathChange,
  probe,
  onProbe,
  onBrowse,
  version,
  onVersionChange,
  onInstall,
  installing,
}: FreacControlsProps) {
  return (
    <>
      <div className="input-row">
        <input
          value={path}
          onChange={(event) => onPathChange(event.target.value)}
          placeholder="Path to freaccmd.exe"
        />
        {onBrowse ? (
          <button type="button" className="button button--ghost" onClick={onBrowse}>
            Browse
          </button>
        ) : null}
        <button type="button" className="button button--ghost" onClick={onProbe}>
          Check
        </button>
      </div>

      {probe ? (
        <small
          className={
            probe.exists && probe.version !== "unknown" ? "hint-ok" : "hint-warn"
          }
        >
          {probe.message}
          {probe.exists ? ` (fre:ac ${probe.version})` : ""}
        </small>
      ) : (
        <small>
          RipBook uses fre:ac’s <code>freaccmd.exe</code>. Choose an installed
          copy or download one below.
        </small>
      )}

      <div className="input-row install-row">
        <input
          value={version}
          onChange={(event) => onVersionChange(event.target.value)}
          placeholder="Latest fre:ac (or e.g. 1.1.7)"
          aria-label="fre:ac version to download"
        />
        <button
          type="button"
          className="button"
          onClick={onInstall}
          disabled={installing}
        >
          {installing ? "Downloading…" : "Download fre:ac"}
        </button>
      </div>
      <small>
        Downloads the official Windows portable release into RipBook’s app-data
        folder. Leave the version blank for the latest release.
      </small>
    </>
  );
}
