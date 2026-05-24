import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type { UnlistenFn };
import type { DownloadProgress, TranscribeProgress, RecordProgress } from "./commands";

export function onDownloadProgress(
  cb: (data: DownloadProgress) => void,
): Promise<UnlistenFn> {
  return listen<DownloadProgress>("download-progress", (e) => cb(e.payload));
}

export function onDownloadDone(cb: () => void): Promise<UnlistenFn> {
  return listen("download-done", () => cb());
}

export function onTranscribeProgress(
  cb: (data: TranscribeProgress) => void,
): Promise<UnlistenFn> {
  return listen<TranscribeProgress>("transcribe-progress", (e) => cb(e.payload));
}

export function onTranscribeDone(cb: (text: string) => void): Promise<UnlistenFn> {
  return listen<string>("transcribe-done", (e) => cb(e.payload));
}

export function onTranscribeError(cb: (msg: string) => void): Promise<UnlistenFn> {
  return listen<string>("transcribe-error", (e) => cb(e.payload));
}

export function onRecordProgress(
  cb: (data: RecordProgress) => void,
): Promise<UnlistenFn> {
  return listen<RecordProgress>("record-progress", (e) => cb(e.payload));
}
