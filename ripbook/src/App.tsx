import { useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { confirm, open } from "@tauri-apps/plugin-dialog";

import { api } from "./api";
import { BookView } from "./components/BookView";
import { NewBookForm } from "./components/NewBookForm";
import type {
  AppInfo,
  BookForm,
  BookManifest,
  BookSummary,
  DriveInfo,
  FreacInfo,
  LogPayload,
  StatePayload,
} from "./types";
import "./App.css";

function pickDrive(drives: DriveInfo[], preferred: string): string {
  const value = preferred.trim();
  const match =
    drives.find(
      (drive) =>
        drive.letter.toUpperCase() === value.toUpperCase() ||
        drive.letter.replace(/:$/, "").toUpperCase() ===
          value.replace(/:$/, "").toUpperCase() ||
        String(drive.index) === value,
    ) ?? drives[0];

  return match?.letter ?? (value || "0");
}

function formatError(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  try {
    return JSON.stringify(error);
  } catch {
    return "Something went wrong.";
  }
}

function formatDate(epochMillis: number): string {
  if (!epochMillis) return "";
  return new Date(epochMillis).toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

const emptyForm: BookForm = {
  author: "",
  title: "",
  firstDisc: 1,
  lastDisc: 1,
};

export default function App() {
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [books, setBooks] = useState<BookSummary[]>([]);
  const [manifest, setManifest] = useState<BookManifest | null>(null);
  const [form, setForm] = useState<BookForm>(emptyForm);
  const [audioRoot, setAudioRoot] = useState("");
  const [drive, setDrive] = useState("0");
  const [freacPath, setFreacPath] = useState("");
  const [freacVersion, setFreacVersion] = useState("");
  const [probe, setProbe] = useState<FreacInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [busyDisc, setBusyDisc] = useState<number | null>(null);
  const [metadataBusy, setMetadataBusy] = useState(false);
  const [installingFreac, setInstallingFreac] = useState(false);
  const [logLines, setLogLines] = useState<string[]>([]);
  const [ripState, setRipState] = useState<StatePayload | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const refreshBooks = async (root = audioRoot) => {
    try {
      setBooks(await api.listBooks(root));
    } catch (caught) {
      setError(formatError(caught));
    }
  };

  const refreshDrives = async (showNotice = false) => {
    try {
      const drives = await api.listDrives();
      setInfo((current) => (current ? { ...current, drives } : current));
      setDrive((current) => pickDrive(drives, current));

      if (showNotice) {
        setNotice(
          drives.length === 0
            ? "No optical drive detected yet. Add or power on a drive, then refresh again."
            : `${drives.length} optical drive${
                drives.length === 1 ? "" : "s"
              } detected.`,
        );
      }
    } catch (caught) {
      setError(formatError(caught));
    }
  };

  const applyProbe = (result: FreacInfo) => {
    setProbe(result);
    setInfo((current) =>
      current
        ? {
            ...current,
            freaccmdFound: result.exists,
            defaultFreaccmdPath: result.path,
          }
        : current,
    );
  };

  useEffect(() => {
    let cancelled = false;

    (async () => {
      try {
        const appInfo = await api.getAppInfo();
        if (cancelled) return;

        setInfo(appInfo);
        setAudioRoot(appInfo.defaultAudioRoot);
        setFreacPath(appInfo.defaultFreaccmdPath);
        setDrive(pickDrive(appInfo.drives, appInfo.defaultDrive));

        const [library, tool] = await Promise.all([
          api.listBooks(appInfo.defaultAudioRoot),
          api.probeFreaccmd(appInfo.defaultFreaccmdPath),
        ]);
        if (cancelled) return;
        setBooks(library);
        setProbe(tool);
      } catch (caught) {
        if (!cancelled) setError(formatError(caught));
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    const handleFocus = () => {
      void refreshDrives();
    };

    window.addEventListener("focus", handleFocus);
    return () => window.removeEventListener("focus", handleFocus);
  }, []);

  useEffect(() => {
    let disposed = false;
    const unlisteners: Array<() => void> = [];

    (async () => {
      const unlistenLog = await listen<LogPayload>("rip-log", (event) => {
        setLogLines((lines) => [
          ...lines.slice(-799),
          `[CD ${event.payload.disc}] ${event.payload.line}`,
        ]);
      });
      const unlistenBook = await listen<BookManifest>(
        "book-updated",
        (event) => setManifest(event.payload),
      );
      const unlistenState = await listen<StatePayload>("rip-state", (event) => {
        setRipState(event.payload);
      });

      if (disposed) {
        unlistenLog();
        unlistenBook();
        unlistenState();
      } else {
        unlisteners.push(unlistenLog, unlistenBook, unlistenState);
      }
    })().catch((caught) => setError(formatError(caught)));

    return () => {
      disposed = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    if (!manifest) return;
    setDrive(info ? pickDrive(info.drives, manifest.drive) : manifest.drive);
    setFreacPath(manifest.freaccmdPath);
  }, [manifest?.bookDir]);

  const handleCreate = async () => {
    if (busy) return;
    if (!form.author.trim()) {
      setError("Author is required.");
      return;
    }
    if (!form.title.trim()) {
      setError("Book title is required.");
      return;
    }
    if (form.firstDisc < 1) {
      setError("The first disc must be 1 or greater.");
      return;
    }
    if (form.lastDisc < form.firstDisc) {
      setError("The last disc must be greater than or equal to the first disc.");
      return;
    }

    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const created = await api.createBook({
        author: form.author.trim(),
        title: form.title.trim(),
        firstDisc: form.firstDisc,
        lastDisc: form.lastDisc,
        audioRoot,
        drive,
        freaccmdPath: freacPath,
      });
      setManifest(created);
      setLogLines([]);
      setRipState(null);
      setNotice(
        created.discs.some((disc) => disc.status === "done")
          ? "Book loaded. Existing files were imported where possible."
          : `Book ready. Insert disc ${created.firstDisc} when you are ready.`,
      );
      await refreshBooks(created.audioRoot);
    } catch (caught) {
      setError(formatError(caught));
    } finally {
      setBusy(false);
    }
  };

  const handleOpen = async (bookDir: string) => {
    if (busy) return;
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const loaded = await api.loadBook(bookDir);
      setManifest(loaded);
      setLogLines([]);
      setRipState(null);
      applyProbe(await api.probeFreaccmd(loaded.freaccmdPath));
      setNotice(
        loaded.discs.every((disc) => disc.status === "done")
          ? "Book loaded. All discs in this book are complete; add more discs if the set has more."
          : "Book loaded. RipBook will continue with the next pending disc.",
      );
    } catch (caught) {
      setError(formatError(caught));
    } finally {
      setBusy(false);
    }
  };

  const handleLookupDisc = async () => {
    if (metadataBusy) return;

    if (!info || info.drives.length === 0) {
      setError(
        "No optical drive detected. Connect a drive and insert a disc, then refresh the drive list.",
      );
      return;
    }

    setMetadataBusy(true);
    setError(null);
    setNotice(null);
    try {
      const disc = await api.lookupDiscMetadata(drive);
      const patch: Partial<BookForm> = {};

      if (disc.artist) patch.author = disc.artist;
      if (disc.title) patch.title = disc.title;
      if (disc.discCount && disc.discCount > 0) {
        patch.lastDisc = form.firstDisc + disc.discCount - 1;
      }

      if (Object.keys(patch).length > 0) {
        setForm((current) => ({ ...current, ...patch }));
      }

      const tracks = `${disc.trackCount} audio track${
        disc.trackCount === 1 ? "" : "s"
      }`;
      setNotice(`Found ${tracks}. ${disc.message}`);
    } catch (caught) {
      setError(formatError(caught));
    } finally {
      setMetadataBusy(false);
    }
  };

  const handleInstallFreac = async () => {
    if (installingFreac) return;

    setInstallingFreac(true);
    setError(null);
    setNotice(null);
    try {
      const result = await api.installFreac(freacVersion.trim() || null);
      setFreacPath(result.path);
      applyProbe(await api.probeFreaccmd(result.path));
      setNotice(result.message);
    } catch (caught) {
      setError(formatError(caught));
    } finally {
      setInstallingFreac(false);
    }
  };

  const handleSetLastDisc = async (lastDisc: number) => {
    if (!manifest || busy) return;

    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const updated = await api.setLastDisc(manifest.bookDir, lastDisc);
      setManifest(updated);
      setNotice(
        `Book now contains ${updated.discs.length} disc${
          updated.discs.length === 1 ? "" : "s"
        }.`,
      );
      await refreshBooks(updated.audioRoot);
    } catch (caught) {
      setError(formatError(caught));
    } finally {
      setBusy(false);
    }
  };

  const handleRip = async (discIndex: number) => {
    if (!manifest || busy) return;

    const disc = manifest.discs.find((entry) => entry.index === discIndex);
    if (disc?.status === "done") {
      try {
        const confirmed = await confirm(
          `Re-rip disc ${discIndex}? Insert disc ${discIndex}; its existing files will be replaced.`,
          { title: "Re-rip disc", kind: "warning" },
        );
        if (!confirmed) return;
      } catch (caught) {
        setError(formatError(caught));
        return;
      }
    }

    setBusy(true);
    setBusyDisc(discIndex);
    setError(null);
    setNotice(null);
    setLogLines([]);
    setRipState({
      disc: discIndex,
      stage: "starting",
      message: "Preparing disc...",
    });

    try {
      const updated = await api.ripDisc({
        bookDir: manifest.bookDir,
        discIndex,
        drive,
        freaccmdPath: freacPath,
      });
      setManifest(updated);
      const nextPending = updated.discs.find(
        (entry) => entry.status === "pending" || entry.status === "failed",
      );
      setNotice(
        nextPending
          ? `Disc ${discIndex} complete. Insert disc ${nextPending.index} next.`
          : `Disc ${discIndex} complete. All discs in this book are complete. If the set has more discs, add them.`,
      );
      await refreshBooks(updated.audioRoot);
    } catch (caught) {
      const message = formatError(caught);
      if (message === "Rip cancelled.") {
        setNotice("Rip cancelled. You can retry this disc when you are ready.");
      } else {
        setError(message);
      }
      try {
        setManifest(await api.loadBook(manifest.bookDir));
      } catch {
        // Keep the last known manifest if reloading also fails.
      }
    } finally {
      setBusy(false);
      setBusyDisc(null);
      setRipState(null);
    }
  };

  const handleCancel = async () => {
    try {
      await api.cancelRip();
      setNotice("Cancelling the current rip...");
    } catch (caught) {
      setError(formatError(caught));
    }
  };

  const handleEject = async () => {
    try {
      await api.ejectDrive(drive);
      setNotice("Eject command sent.");
    } catch (caught) {
      setError(formatError(caught));
    }
  };

  const handleProbe = async () => {
    try {
      applyProbe(await api.probeFreaccmd(freacPath));
    } catch (caught) {
      setError(formatError(caught));
    }
  };

  const handleBrowseAudioRoot = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        defaultPath: audioRoot || undefined,
        title: "Choose the audiobook library folder",
      });
      if (typeof selected === "string") {
        setAudioRoot(selected);
      }
    } catch (caught) {
      setError(formatError(caught));
    }
  };

  const handleBrowseFreac = async () => {
    try {
      const selected = await open({
        directory: false,
        multiple: false,
        defaultPath: freacPath || undefined,
        title: "Choose freaccmd.exe",
      });
      if (typeof selected === "string") {
        setFreacPath(selected);
        applyProbe(await api.probeFreaccmd(selected));
      }
    } catch (caught) {
      setError(formatError(caught));
    }
  };

  const handleOpenFolder = async () => {
    if (!manifest) return;
    try {
      await api.openFolder(manifest.bookDir);
    } catch (caught) {
      setError(formatError(caught));
    }
  };

  const handleBack = () => {
    setManifest(null);
    setRipState(null);
    setLogLines([]);
    setError(null);
    setNotice(null);
    void refreshBooks(audioRoot);
  };

  const headerStatus = useMemo(() => {
    if (!info) return "Starting…";
    const platform = info.platform === "windows" ? "Windows" : info.platform;
    const tool = info.freaccmdFound ? "fre:ac found" : "fre:ac not found";
    const drives =
      info.drives.length === 0
        ? "no optical drive"
        : `${info.drives.length} optical drive${
            info.drives.length === 1 ? "" : "s"
          }`;
    return `${platform} · ${tool} · ${drives}`;
  }, [info]);

  if (loading) {
    return (
      <div className="splash">
        <div className="brand-mark brand-mark--large">RB</div>
        <h1>RipBook</h1>
        <p>Checking your library and fre:ac installation…</p>
      </div>
    );
  }

  return (
    <div className="app-shell">
      <header className="app-header">
        <div className="brand">
          <div className="brand-mark">RB</div>
          <div>
            <strong>RipBook</strong>
            <span>Resumable audiobook CD ripper</span>
          </div>
        </div>
        <div className="app-status">
          <span
            className={`status-dot ${
              info?.freaccmdFound ? "status-dot--ok" : "status-dot--warn"
            }`}
          />
          {headerStatus}
          <button
            type="button"
            className="button button--ghost button--small"
            onClick={() => void refreshDrives(true)}
          >
            Refresh drives
          </button>
        </div>
      </header>

      {error ? (
        <div className="banner banner--error" role="alert">
          <span>{error}</span>
          <button type="button" onClick={() => setError(null)}>
            Dismiss
          </button>
        </div>
      ) : null}

      {notice ? (
        <div className="banner banner--notice" role="status">
          <span>{notice}</span>
          <button type="button" onClick={() => setNotice(null)}>
            Dismiss
          </button>
        </div>
      ) : null}

      {info && info.platform !== "windows" ? (
        <div className="banner banner--warning">
          This is a development build for {info.platform}. CD ripping and drive
          ejection require Windows with fre:ac installed.
        </div>
      ) : null}

      <main className={manifest ? "main main--book" : "main"}>
        {manifest ? (
          <BookView
            manifest={manifest}
            busy={busy}
            busyDisc={busyDisc}
            ripState={ripState}
            logLines={logLines}
            drive={drive}
            drives={info?.drives ?? []}
            onDriveChange={setDrive}
            onRefreshDrives={() => void refreshDrives(true)}
            freacPath={freacPath}
            onFreacPathChange={setFreacPath}
            onBrowseFreac={handleBrowseFreac}
            freacVersion={freacVersion}
            onFreacVersionChange={setFreacVersion}
            onInstallFreac={handleInstallFreac}
            installingFreac={installingFreac}
            probe={probe}
            onProbe={handleProbe}
            onRip={handleRip}
            onCancel={handleCancel}
            onEject={handleEject}
            onOpenFolder={handleOpenFolder}
            onSetLastDisc={handleSetLastDisc}
            onBack={handleBack}
          />
        ) : (
          <div className="library-layout">
            <NewBookForm
              form={form}
              onChange={(patch) => setForm((current) => ({ ...current, ...patch }))}
              audioRoot={audioRoot}
              onAudioRootChange={setAudioRoot}
              drive={drive}
              onDriveChange={setDrive}
              onRefreshDrives={() => void refreshDrives(true)}
              onLookupDisc={handleLookupDisc}
              metadataBusy={metadataBusy}
              freacPath={freacPath}
              onFreacPathChange={setFreacPath}
              freacVersion={freacVersion}
              onFreacVersionChange={setFreacVersion}
              onInstallFreac={handleInstallFreac}
              installingFreac={installingFreac}
              probe={probe}
              info={info}
              busy={busy}
              onBrowseAudioRoot={handleBrowseAudioRoot}
              onBrowseFreac={handleBrowseFreac}
              onProbe={handleProbe}
              onCreate={handleCreate}
            />

            <section className="panel library-panel">
              <div className="panel-heading">
                <div>
                  <p className="eyebrow">Library</p>
                  <h2>Continue a book</h2>
                </div>
                <button
                  type="button"
                  className="button button--ghost button--small"
                  onClick={() => void refreshBooks(audioRoot)}
                >
                  Refresh
                </button>
              </div>

              {books.length === 0 ? (
                <div className="empty-state">
                  <strong>No books here yet.</strong>
                  <p>
                    Create a book on the left. Its progress is saved in the book
                    folder, so closing RipBook never loses your place.
                  </p>
                </div>
              ) : (
                <div className="book-list">
                  {books.map((book) => {
                    const percent =
                      book.totalDiscs === 0
                        ? 0
                        : Math.round((book.doneDiscs / book.totalDiscs) * 100);
                    return (
                      <button
                        type="button"
                        className="book-card"
                        key={book.bookDir}
                        disabled={busy}
                        onClick={() => void handleOpen(book.bookDir)}
                      >
                        <div className="book-card-main">
                          <span className="book-author">{book.author}</span>
                          <strong>{book.title}</strong>
                          <span className="book-meta">
                            {book.doneDiscs} of {book.totalDiscs} discs · updated{" "}
                            {formatDate(book.updatedAt)}
                          </span>
                        </div>
                        <div className="book-card-progress">
                          <span>{percent}%</span>
                          <div className="mini-track">
                            <div
                              className="mini-fill"
                              style={{ width: `${percent}%` }}
                            />
                          </div>
                        </div>
                      </button>
                    );
                  })}
                </div>
              )}
            </section>
          </div>
        )}
      </main>

      <footer className="app-footer">
        <span>
          Files are stored under <code>AudioBooks/Author/Book</code> as{" "}
          <code>cdNN_tNN.mp3</code>.
        </span>
        <span>Resume state lives in each book folder.</span>
      </footer>
    </div>
  );
}
