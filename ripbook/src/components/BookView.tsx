import { useEffect, useMemo, useRef, useState } from "react";

import type {
  BookManifest,
  DiscState,
  DriveInfo,
  FreacInfo,
  StatePayload,
} from "../types";
import { DiscRow } from "./DiscRow";
import { FreacControls } from "./FreacControls";

const STATUS_LABELS: Record<DiscState["status"], string> = {
  pending: "Pending",
  ripping: "Ripping",
  done: "Complete",
  failed: "Failed",
};

interface BookViewProps {
  manifest: BookManifest;
  busy: boolean;
  busyDisc: number | null;
  ripState: StatePayload | null;
  logLines: string[];
  drive: string;
  drives: DriveInfo[];
  onDriveChange: (value: string) => void;
  onRefreshDrives: () => void;
  freacPath: string;
  onFreacPathChange: (value: string) => void;
  onBrowseFreac: () => void;
  freacVersion: string;
  onFreacVersionChange: (value: string) => void;
  onInstallFreac: () => void;
  installingFreac: boolean;
  probe: FreacInfo | null;
  onProbe: () => void;
  onRip: (disc: number) => void;
  onCancel: () => void;
  onEject: () => void;
  onOpenFolder: () => void;
  onSetLastDisc: (lastDisc: number) => void;
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
  onRefreshDrives,
  freacPath,
  onFreacPathChange,
  onBrowseFreac,
  freacVersion,
  onFreacVersionChange,
  onInstallFreac,
  installingFreac,
  probe,
  onProbe,
  onRip,
  onCancel,
  onEject,
  onOpenFolder,
  onSetLastDisc,
  onBack,
}: BookViewProps) {
  const logRef = useRef<HTMLPreElement>(null);
  const [targetDisc, setTargetDisc] = useState<number | null>(null);
  const [discCountDraft, setDiscCountDraft] = useState(manifest.discs.length);

  const doneCount = useMemo(
    () => manifest.discs.filter((disc) => disc.status === "done").length,
    [manifest.discs],
  );
  const totalCount = manifest.discs.length;
  const progress = totalCount === 0 ? 0 : Math.round((doneCount / totalCount) * 100);
  const nextDisc = firstActionableDisc(manifest.discs);
  const allDone = totalCount > 0 && doneCount === totalCount;

  const selectedDisc =
    manifest.discs.find((disc) => disc.index === targetDisc) ??
    nextDisc ??
    manifest.discs[manifest.discs.length - 1] ??
    null;

  const discPercent = ripState?.percent ?? null;
  const discCountValid =
    Number.isFinite(discCountDraft) &&
    discCountDraft >= 1 &&
    manifest.firstDisc + discCountDraft - 1 <= 999;
  const ripLabel = selectedDisc
    ? selectedDisc.status === "done"
      ? `Re-rip disc ${selectedDisc.index}`
      : selectedDisc.status === "failed"
        ? `Retry disc ${selectedDisc.index}`
        : `Rip disc ${selectedDisc.index}`
    : "Rip disc";

  useEffect(() => {
    const node = logRef.current;
    if (node) {
      node.scrollTop = node.scrollHeight;
    }
  }, [logLines]);

  useEffect(() => {
    if (nextDisc) {
      setTargetDisc(nextDisc.index);
    }
  }, [nextDisc?.index, manifest.bookDir]);

  useEffect(() => {
    setTargetDisc((current) =>
      current != null && manifest.discs.some((disc) => disc.index === current)
        ? current
        : (nextDisc?.index ?? manifest.discs[0]?.index ?? null),
    );
  }, [manifest.bookDir, manifest.discs, nextDisc?.index]);

  useEffect(() => {
    setDiscCountDraft(manifest.discs.length);
  }, [manifest.bookDir, manifest.discs.length]);

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

        {ripState && busy ? (
          <div className="disc-progress">
            <div className="disc-progress-topline">
              <span>
                <strong>Current disc {ripState.disc}</strong>
                {ripState.trackTotal
                  ? ` · ${ripState.track ?? 0} of ${ripState.trackTotal} tracks`
                  : ripState.track
                    ? ` · track ${ripState.track}`
                    : ""}
              </span>
              <span className="disc-progress-percent">
                {discPercent != null ? `${discPercent}%` : "working…"}
              </span>
            </div>
            <div
              className={`progress-track progress-track--disc ${
                discPercent == null ? "progress-track--indeterminate" : ""
              }`}
              role="progressbar"
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={discPercent ?? undefined}
            >
              <div
                className="progress-fill"
                style={
                  discPercent != null ? { width: `${discPercent}%` } : undefined
                }
              />
            </div>
            <p className={`rip-state rip-state--${ripState.stage}`}>
              {ripState.message}
            </p>
          </div>
        ) : null}

        <div className={`next-step ${allDone ? "next-step--done" : ""}`}>
          {allDone ? (
            <div>
              <strong>All discs in this book are complete.</strong>
              <p>
                If the audiobook has more discs, add them below. Audio CDs do not
                record how many discs are in a set.
              </p>
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
              <strong>Choose a disc to rip.</strong>
              <p>Use the target disc selector to pick any disc, including one already ripped.</p>
            </div>
          )}

          <div className="next-step-actions">
            {busy && ripState ? (
              <button type="button" className="button" onClick={onCancel}>
                Cancel rip
              </button>
            ) : selectedDisc ? (
              <>
                <select
                  className="target-select"
                  value={String(selectedDisc.index)}
                  onChange={(event) => setTargetDisc(Number(event.target.value))}
                  aria-label="Target disc"
                  disabled={busy}
                >
                  {manifest.discs.map((disc) => (
                    <option key={disc.index} value={String(disc.index)}>
                      Disc {disc.index} — {STATUS_LABELS[disc.status]}
                    </option>
                  ))}
                </select>
                <button
                  type="button"
                  className="button button--primary"
                  onClick={() => onRip(selectedDisc.index)}
                  disabled={busy}
                >
                  {ripLabel}
                </button>
              </>
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

        <div className="disc-count-editor">
          <label>
            <span>Discs in this book</span>
            <input
              type="number"
              min={1}
              max={999}
              step={1}
              value={Number.isFinite(discCountDraft) ? discCountDraft : ""}
              onChange={(event) => setDiscCountDraft(Number(event.target.value))}
              disabled={busy}
            />
          </label>
          <button
            type="button"
            className="button button--ghost button--small"
            onClick={() =>
              onSetLastDisc(manifest.firstDisc + discCountDraft - 1)
            }
            disabled={busy || !discCountValid || discCountDraft === totalCount}
          >
            Update disc count
          </button>
          {allDone ? (
            <button
              type="button"
              className="button button--small"
              onClick={() => onSetLastDisc(manifest.firstDisc + totalCount)}
              disabled={busy}
            >
              Add disc {manifest.firstDisc + totalCount}
            </button>
          ) : null}
        </div>
      </section>

      <section className="panel disc-panel">
        <div className="panel-heading panel-heading--compact">
          <div>
            <p className="eyebrow">Disc list</p>
            <h2>Transfer queue</h2>
          </div>
          <p className="panel-note">
            Select any disc to re-rip it, or use the target selector above.
            Completed discs can be replaced at any time.
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
          <div className="field">
            <span>CD/DVD drive</span>
            <div className="input-row">
              <select
                value={drive}
                onChange={(event) => onDriveChange(event.target.value)}
              >
                {drives.length > 0 ? (
                  drives.map((entry) => (
                    <option key={entry.letter} value={entry.letter}>
                      Drive {entry.letter} {entry.name}
                    </option>
                  ))
                ) : (
                  <option value={drive}>
                    {drive === "0" ? "No drive detected" : `${drive} (not detected)`}
                  </option>
                )}
              </select>
              <button
                type="button"
                className="button button--ghost"
                onClick={onRefreshDrives}
              >
                Refresh
              </button>
            </div>
            <small>
              {drives.length > 0
                ? `${drives.length} optical drive${
                    drives.length === 1 ? "" : "s"
                  } detected. Refresh after adding a drive.`
                : "No optical drive detected. Add or power on a drive, then press Refresh."}
            </small>
          </div>

          <div className="field field--wide">
            <span>fre:ac command line tool</span>
            <FreacControls
              path={freacPath}
              onPathChange={onFreacPathChange}
              probe={probe}
              onProbe={onProbe}
              onBrowse={onBrowseFreac}
              version={freacVersion}
              onVersionChange={onFreacVersionChange}
              onInstall={onInstallFreac}
              installing={installingFreac}
            />
          </div>
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
