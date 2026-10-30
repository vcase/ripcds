import type { AppInfo, BookForm, FreacInfo } from "../types";
import { FreacControls } from "./FreacControls";

interface NewBookFormProps {
  form: BookForm;
  onChange: (patch: Partial<BookForm>) => void;
  audioRoot: string;
  onAudioRootChange: (value: string) => void;
  drive: string;
  onDriveChange: (value: string) => void;
  onRefreshDrives: () => void;
  onLookupDisc: () => void;
  metadataBusy: boolean;
  freacPath: string;
  onFreacPathChange: (value: string) => void;
  freacVersion: string;
  onFreacVersionChange: (value: string) => void;
  onInstallFreac: () => void;
  installingFreac: boolean;
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
  onRefreshDrives,
  onLookupDisc,
  metadataBusy,
  freacPath,
  onFreacPathChange,
  freacVersion,
  onFreacVersionChange,
  onInstallFreac,
  installingFreac,
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

        <div className="field field--wide metadata-row">
          <button
            type="button"
            className="button"
            onClick={onLookupDisc}
            disabled={busy || metadataBusy}
          >
            {metadataBusy ? "Reading disc…" : "Read disc metadata"}
          </button>
          <p className="form-hint">
            Audio CDs do not store the total number of discs in a set, so set the
            last disc up front (you can add discs later). If the inserted disc has
            a MusicBrainz match, this fills in author, title, and the disc count.
          </p>
        </div>

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
          <div className="input-row">
            <select
              value={drive}
              onChange={(event) => onDriveChange(event.target.value)}
            >
              {info && info.drives.length > 0 ? (
                info.drives.map((entry) => (
                  <option key={entry.letter} value={entry.letter}>
                    Drive {entry.letter} {entry.name}
                  </option>
                ))
              ) : (
                <option value={drive}>{drive === "0" ? "No drive detected" : `${drive} (not detected)`}</option>
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
            {info && info.drives.length > 0
              ? `${info.drives.length} optical drive${
                  info.drives.length === 1 ? "" : "s"
                } detected. Refresh after adding a drive.`
              : "No optical drive detected yet. Add or power on a drive, then press Refresh."}
          </small>
        </label>

        <label className="field field--wide">
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
