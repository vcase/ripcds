import type { DiscState } from "../types";

const STATUS_LABELS: Record<DiscState["status"], string> = {
  pending: "Pending",
  ripping: "Ripping",
  done: "Complete",
  failed: "Failed",
};

interface DiscRowProps {
  disc: DiscState;
  busy: boolean;
  isBusyDisc: boolean;
  onRip: (disc: number) => void;
}

export function DiscRow({ disc, busy, isBusyDisc, onRip }: DiscRowProps) {
  const actionLabel =
    disc.status === "done"
      ? "Re-rip"
      : disc.status === "failed"
        ? "Retry"
        : disc.status === "ripping" || isBusyDisc
          ? "Ripping..."
          : "Rip disc";

  return (
    <div className={`disc-row disc-row--${disc.status}`}>
      <div className="disc-number" aria-hidden="true">
        {String(disc.index).padStart(2, "0")}
      </div>

      <div className="disc-main">
        <div className="disc-title-line">
          <strong>Disc {disc.index}</strong>
          <span className={`badge badge--${disc.status}`}>
            {STATUS_LABELS[disc.status]}
          </span>
          {disc.trackCount != null && disc.status === "done" ? (
            <span className="disc-meta">{disc.trackCount} tracks</span>
          ) : null}
        </div>
        {disc.error ? (
          <p
            className={
              disc.error.startsWith("Imported") ? "disc-warning" : "disc-error"
            }
          >
            {disc.error}
          </p>
        ) : null}
        {disc.status === "done" && disc.files.length > 0 ? (
          <p className="disc-files">
            {disc.files.length === 1
              ? disc.files[0]
              : `${disc.files[0]} … ${disc.files[disc.files.length - 1]}`}
          </p>
        ) : null}
      </div>

      <button
        type="button"
        className="button button--small"
        disabled={busy}
        onClick={() => onRip(disc.index)}
      >
        {actionLabel}
      </button>
    </div>
  );
}
