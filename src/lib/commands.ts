import { invoke } from "@tauri-apps/api/core";

export interface MissingModels {
	accurate: boolean;
	fast: boolean;
}

export interface DownloadProgress {
	label: string;
	downloaded: number;
	total: number;
}

export interface TranscribeProgress {
	progress: number;
	text: string;
}

export interface RecordProgress {
	elapsed: number;
	text: string;
}

export function checkModels(): Promise<MissingModels> {
	return invoke("check_models");
}

export function downloadModels(): Promise<void> {
	return invoke("download_models");
}

export function getModelsDir(): Promise<string> {
	return invoke("get_models_dir");
}

export function pickAudioFile(): Promise<string | null> {
	return invoke("pick_audio_file");
}

export function pickSaveFile(): Promise<string | null> {
	return invoke("pick_save_file");
}

export function pickAudioSaveFile(): Promise<string | null> {
	return invoke("pick_audio_save_file");
}

export function transcribeFile(path: string, model: string): Promise<void> {
	return invoke("transcribe_file", { path, model });
}

export function startRecording(
	model: string,
	saveAudioPath?: string | null,
	saveTranscriptPath?: string | null,
	vadEnabled?: boolean,
): Promise<void> {
	return invoke("start_recording", {
		model,
		saveAudioPath,
		saveTranscriptPath,
		vadEnabled,
	});
}

export function stopRecording(): Promise<string> {
	return invoke("stop_recording");
}

export function cancel(): Promise<void> {
	return invoke("cancel");
}

export function saveTextFile(path: string, content: string): Promise<void> {
	return invoke("save_text_file", { path, content });
}
