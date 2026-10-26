import { useEffect, useMemo, useRef } from "react";

import type {
  BookManifest,
  DiscState,
  DriveInfo,
  FreacInfo,
  StatePayload,
} from "../types";
import { DiscRow } from "./DiscRow";

interface BookViewProps {
  manifest: BookManifest;
  busy: boolean;
  busyDisc: number | null;
  ripState: StatePayload | null;
  logLines: string[];
  drive: string;
  drives: DriveInfo[];
  onDriveChange: (value: string) => void;
  freacPath: string;
  onFreacPathChange: (value: string) => void;
  probe: FreacInfo | null;
  onProbe: () => void;
  onRip: (disc: number) => void;
  onCancel: () => void;
  onEject: () => void;
  onOpenFolder: () => void;
  onBack: () => void;
}

function firstActionableDisc(discs: DiscState[]): DiscState | null {
  return (
    discs.find((disc) => disc.status === "ripping") ??
    discs.find((disc) => disc.status === "failed") ??
    discs.find((disc) => disc.status === "pending") ??
    null
  );
}

export function BookView({
  manifest,
  busy,
  busyDisc,
  ripState,
  logLines,
  drive,
  drives,
  onDriveChange,
  freacPath,
  onFreacPathChange,
  probe,
  onProbe,
  onRip,
  onCancel,
  onEject,
  onOpenFolder,
  onBack,
}: BookViewProps) {
  const logRef = useRef<HTMLPreElement>(null);

  const doneCount = useMemo(
    () => manifest.discs.filter((disc) => disc.status === "done").length,
    [manifest.discs],
  );
  const totalCount = manifest.discs.length;
  const progress = totalCount === 0 ? 0 : Math.round((doneCount / totalCount) * 100);
  const nextDisc = firstActionableDisc(manifest.discs);
  const allDone = totalCount > 0 && doneCount === totalCount;

  useEffect(() => {
    const node = logRef.current;
    if (node) {
      node.scrollTop = node.scrollHeight;
    }
  }, [logLines]);

  return (
    <div className="book-layout">
      <header className="book-header">
        <button
          type="button"
          className="button button--ghost"
          onClick={onBack}
          disabled={busy}
        >
          ← Library
        </button>
        <div className="book-heading">
          <p className="eyebrow">{manifest.author}</p>
          <h1>{manifest.title}</h1>
          <p className="book-path" title={manifest.bookDir}>
            {manifest.bookDir}
          </p>
        </div>
        <div className="book-header-actions">
          <button type="button" className="button button--ghost" onClick={onOpenFolder}>
            Open folder
          </button>
        </div>
      </header>

      <section className="progress-card">
        <div className="progress-topline">
          <div>
            <p className="eyebrow">Book progress</p>
            <strong>
              {doneCount} of {totalCount} discs complete
            </strong>
          </div>
          <span className="progress-percent">{progress}%</span>
        </div>
        <div
          className="progress-track"
          role="progressbar"
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={progress}
        >
          <div className="progress-fill" style={{ width: `${progress}%` }} />
        </div>

        <div className={`next-step ${allDone ? "next-step--done" : ""}`}>
          {allDone ? (
            <div>
              <strong>All discs are complete.</strong>
              <p>You can re-rip any disc or open the folder to review the files.</p>
            </div>
          ) : nextDisc ? (
            <div>
              <strong>
                Insert disc {nextDisc.index}
                {nextDisc.status === "failed" ? " and retry" : ""}
              </strong>
              <p>
                {ripState && busy
                  ? ripState.message
                  : nextDisc.status === "failed"
                    ? "The previous attempt failed. Check the log, then retry."
                    : "When the disc is ready, start the rip. The disc will be ejected when it finishes."}
              </p>
            </div>
          ) : (
            <div>
              <strong>Ready for the next disc.</strong>
              <p>All currently known discs are complete.</p>
            </div>
          )}

          <div className="next-step-actions">
            {busy ? (
              <button type="button" className="button" onClick={onCancel}>
                Cancel rip
              </button>
            ) : nextDisc ? (
              <button
                type="button"
                className="button button--primary"
                onClick={() => onRip(nextDisc.index)}
              >
                {nextDisc.status === "failed" ? "Retry" : "Rip"} disc {nextDisc.index}
              </button>
            ) : null}
            <button
              type="button"
              className="button button--ghost"
              onClick={onEject}
              disabled={busy}
            >
              Eject
            </button>
          </div>
        </div>

        {ripState && busy ? (
          <p className={`rip-state rip-state--${ripState.stage}`}>
            Disc {ripState.disc}: {ripState.message}
          </p>
        ) : null}
      </section>

      <section className="panel disc-panel">
        <div className="panel-heading panel-heading--compact">
          <div>
            <p className="eyebrow">Disc list</p>
            <h2>Transfer queue</h2>
          </div>
          <p className="panel-note">
            Completed discs can be re-ripped at any time. Track names are
            normalized to <code>cdNN_tNN</code>.
          </p>
        </div>

        <div className="disc-list">
          {manifest.discs.map((disc) => (
            <DiscRow
              key={disc.index}
              disc={disc}
              busy={busy}
              isBusyDisc={busyDisc === disc.index}
              onRip={onRip}
            />
          ))}
        </div>
      </section>

      <details className="panel settings-panel">
        <summary>
          <span>Ripping settings</span>
          <span className="summary-hint">drive and fre:ac path</span>
        </summary>
        <div className="settings-grid">
          <label className="field">
            <span>CD/DVD drive</span>
            <select
              value={drive}
              onChange={(event) => onDriveChange(event.target.value)}
            >
              {drives.length > 0 ? (
                drives.map((entry) => (
                  <option key={entry.index} value={String(entry.index)}>
                    Drive {entry.index}: {entry.letter} {entry.name}
                  </option>
                ))
              ) : (
                <option value="0">Drive 0 (default)</option>
              )}
            </select>
          </label>

          <label className="field field--wide">
            <span>freaccmd.exe</span>
            <div className="input-row">
              <input
                value={freacPath}
                onChange={(event) => onFreacPathChange(event.target.value)}
                placeholder="Path to freaccmd.exe"
              />
              <button
                type="button"
                className="button button--ghost"
                onClick={onProbe}
              >
                Check
              </button>
            </div>
            {probe ? (
              <small
                className={
                  probe.exists && probe.version !== "unknown"
                    ? "hint-ok"
                    : "hint-warn"
                }
              >
                {probe.message}
                {probe.exists ? ` (fre:ac ${probe.version})` : ""}
              </small>
            ) : null}
          </label>
        </div>
      </details>

      <section className="panel log-panel">
        <div className="panel-heading panel-heading--compact">
          <div>
            <p className="eyebrow">Log</p>
            <h2>fre:ac output</h2>
          </div>
          <p className="panel-note">
            The full log is also saved in the book folder as{" "}
            <code>rip_cdNN.log</code>.
          </p>
        </div>
        <pre ref={logRef} className="log-output">
          {logLines.length > 0
            ? logLines.join("\n")
            : "The rip log will appear here."}
        </pre>
      </section>
    </div>
  );
}
