import type { AppInfo, BookForm, FreacInfo } from "../types";

interface NewBookFormProps {
  form: BookForm;
  onChange: (patch: Partial<BookForm>) => void;
  audioRoot: string;
  onAudioRootChange: (value: string) => void;
  drive: string;
  onDriveChange: (value: string) => void;
  freacPath: string;
  onFreacPathChange: (value: string) => void;
  probe: FreacInfo | null;
  info: AppInfo | null;
  busy: boolean;
  onBrowseAudioRoot: () => void;
  onBrowseFreac: () => void;
  onProbe: () => void;
  onCreate: () => void;
}

export function NewBookForm({
  form,
  onChange,
  audioRoot,
  onAudioRootChange,
  drive,
  onDriveChange,
  freacPath,
  onFreacPathChange,
  probe,
  info,
  busy,
  onBrowseAudioRoot,
  onBrowseFreac,
  onProbe,
  onCreate,
}: NewBookFormProps) {
  return (
    <section className="panel new-book">
      <div className="panel-heading">
        <div>
          <p className="eyebrow">New book</p>
          <h2>Set up a book</h2>
        </div>
        <p className="panel-note">
          Rip each disc in order. If you stop, open the book again and continue
          with the next pending disc.
        </p>
      </div>

      <form
        className="form-grid"
        onSubmit={(event) => {
          event.preventDefault();
          onCreate();
        }}
      >
        <label className="field">
          <span>Author</span>
          <input
            value={form.author}
            onChange={(event) => onChange({ author: event.target.value })}
            placeholder="e.g. Ursula K. Le Guin"
            autoFocus
          />
        </label>

        <label className="field">
          <span>Book title</span>
          <input
            value={form.title}
            onChange={(event) => onChange({ title: event.target.value })}
            placeholder="e.g. The Left Hand of Darkness"
          />
        </label>

        <label className="field field--small">
          <span>First disc</span>
          <input
            type="number"
            min={1}
            max={999}
            step={1}
            value={form.firstDisc}
            onChange={(event) =>
              onChange({ firstDisc: Number(event.target.value) })
            }
          />
        </label>

        <label className="field field--small">
          <span>Last disc</span>
          <input
            type="number"
            min={1}
            max={999}
            step={1}
            value={form.lastDisc}
            onChange={(event) =>
              onChange({ lastDisc: Number(event.target.value) })
            }
          />
        </label>

        <label className="field field--wide">
          <span>Audiobook library folder</span>
          <div className="input-row">
            <input
              value={audioRoot}
              onChange={(event) => onAudioRootChange(event.target.value)}
              placeholder="Choose the folder that holds all audiobooks"
            />
            <button
              type="button"
              className="button button--ghost"
              onClick={onBrowseAudioRoot}
            >
              Browse
            </button>
          </div>
        </label>

        <label className="field">
          <span>CD/DVD drive</span>
          <select
            value={drive}
            onChange={(event) => onDriveChange(event.target.value)}
          >
            {info && info.drives.length > 0 ? (
              info.drives.map((entry) => (
                <option key={entry.index} value={String(entry.index)}>
                  Drive {entry.index}: {entry.letter} {entry.name}
                </option>
              ))
            ) : (
              <option value="0">Drive 0 (default)</option>
            )}
          </select>
          <small>
            {info && info.drives.length > 0
              ? `${info.drives.length} optical drive${
                  info.drives.length === 1 ? "" : "s"
                } detected`
              : "No optical drive detected. You can still configure the book."}
          </small>
        </label>

        <label className="field field--wide">
          <span>fre:ac command line tool</span>
          <div className="input-row">
            <input
              value={freacPath}
              onChange={(event) => onFreacPathChange(event.target.value)}
              placeholder="Path to freaccmd.exe"
            />
            <button
              type="button"
              className="button button--ghost"
              onClick={onBrowseFreac}
            >
              Browse
            </button>
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
          ) : (
            <small>
              RipBook uses fre:ac’s <code>freaccmd.exe</code>. Install fre:ac or
              bundle a portable copy with the app.
            </small>
          )}
        </label>

        <div className="form-actions field--wide">
          <button type="submit" className="button button--primary" disabled={busy}>
            {busy ? "Preparing…" : "Create / open book"}
          </button>
          <span className="form-hint">
            Existing books with the same author and title open in resume mode.
          </span>
        </div>
      </form>
    </section>
  );
}
