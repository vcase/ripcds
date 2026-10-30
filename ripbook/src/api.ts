import { invoke } from "@tauri-apps/api/core";

import type {
  AppInfo,
  BookManifest,
  BookSummary,
  CreateBookRequest,
  DiscInfo,
  DriveInfo,
  FreacInfo,
  FreacInstallResult,
  RipRequest,
} from "./types";

export const api = {
  getAppInfo: () => invoke<AppInfo>("get_app_info"),
  listBooks: (root: string) => invoke<BookSummary[]>("list_books", { root }),
  createBook: (request: CreateBookRequest) =>
    invoke<BookManifest>("create_book", { request }),
  loadBook: (bookDir: string) =>
    invoke<BookManifest>("load_book", { bookDir }),
  setLastDisc: (bookDir: string, lastDisc: number) =>
    invoke<BookManifest>("set_last_disc", { bookDir, lastDisc }),
  listDrives: () => invoke<DriveInfo[]>("list_drives"),
  lookupDiscMetadata: (drive: string) =>
    invoke<DiscInfo>("lookup_disc_metadata", { drive }),
  probeFreaccmd: (path: string) =>
    invoke<FreacInfo>("probe_freaccmd", { path }),
  installFreac: (version: string | null) =>
    invoke<FreacInstallResult>("install_freac", { version }),
  ripDisc: (request: RipRequest) =>
    invoke<BookManifest>("rip_disc", { request }),
  cancelRip: () => invoke<void>("cancel_rip"),
  ejectDrive: (drive: string) => invoke<void>("eject_drive", { drive }),
  openFolder: (path: string) => invoke<void>("open_folder", { path }),
};
