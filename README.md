# ripcds

Tools for ripping multi-disc audiobooks on Windows.

- **`ripbook/`** — RipBook, a resumable Tauri desktop app. See
  [`ripbook/README.md`](ripbook/README.md) for development, Windows build, and
  installation details.
- **`ripBook.bat`** — the original interactive batch script. It remains as a
  legacy fallback and still expects fre:ac 1.0.x command-line flags.

RipBook stores each book's progress in a `.ripbook.json` manifest inside the
book folder and produces files named `cdNN_tNN.mp3`, so an interrupted transfer
can continue with the next pending disc.
