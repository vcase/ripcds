export type DiscStatus = "pending" | "ripping" | "done" | "failed";

export interface DiscState {
  index: number;
  status: DiscStatus;
  trackCount?: number | null;
  files: string[];
  error?: string | null;
  completedAt?: number | null;
}

export interface BookManifest {
  schemaVersion: number;
  author: string;
  title: string;
  bookDir: string;
  audioRoot: string;
  firstDisc: number;
  lastDisc: number;
  drive: string;
  freaccmdPath: string;
  discs: DiscState[];
  createdAt: number;
  updatedAt: number;
}

export interface CreateBookRequest {
  author: string;
  title: string;
  firstDisc: number;
  lastDisc: number;
  audioRoot: string;
  drive: string;
  freaccmdPath: string;
}

export interface RipRequest {
  bookDir: string;
  discIndex: number;
  drive: string;
  freaccmdPath: string;
}

export interface BookSummary {
  bookDir: string;
  author: string;
  title: string;
  doneDiscs: number;
  totalDiscs: number;
  updatedAt: number;
}

export interface DriveInfo {
  index: number;
  letter: string;
  name: string;
}

export interface AppInfo {
  platform: string;
  defaultAudioRoot: string;
  defaultFreaccmdPath: string;
  defaultDrive: string;
  freaccmdFound: boolean;
  drives: DriveInfo[];
}

export interface FreacInfo {
  path: string;
  exists: boolean;
  version: string;
  supportsTimeout: boolean;
  message: string;
}

export interface LogPayload {
  disc: number;
  line: string;
}

export interface StatePayload {
  disc: number;
  stage: "starting" | "ripping" | "installing" | "done" | "failed" | string;
  message: string;
}

export interface BookForm {
  author: string;
  title: string;
  firstDisc: number;
  lastDisc: number;
}
