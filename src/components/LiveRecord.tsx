import { FolderOpen, Mic, Save, Square } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useApp } from "@/App";
import { Layout } from "@/components/Layout";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import { Separator } from "@/components/ui/separator";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import {
	pickSaveFile,
	saveTextFile,
	startRecording,
	stopRecording,
} from "@/lib/commands";
import {
	onRecordProgress,
	onTranscribeDone,
	onTranscribeError,
	type UnlistenFn,
} from "@/lib/events";

function LiveRecord() {
	const {
		status,
		setStatus,
		outputText,
		setOutputText,
		modelChoice,
		setModelChoice,
	} = useApp();
	const [recording, setRecording] = useState(false);
	const [elapsed, setElapsed] = useState(0);
	const [saveAudioPath, setSaveAudioPath] = useState<string | null>(null);
	const [saveTranscriptPath, setSaveTranscriptPath] = useState<string | null>(
		null,
	);
	const [vadEnabled, setVadEnabled] = useState(true);
	const timerRef = useRef<number | null>(null);

	useEffect(() => {
		const unlisteners: Promise<UnlistenFn>[] = [];

		unlisteners.push(
			onRecordProgress((data) => {
				setElapsed(data.elapsed);
				setStatus({
					type: "recording",
					elapsed: data.elapsed,
					text: data.text,
				});
			}),
		);

		unlisteners.push(
			onTranscribeDone((text) => {
				setRecording(false);
				if (timerRef.current) clearInterval(timerRef.current);
				setOutputText((prev) => `${prev}${text}\n`);
				setStatus({ type: "done", text });
			}),
		);

		unlisteners.push(
			onTranscribeError((msg) => {
				setRecording(false);
				if (timerRef.current) clearInterval(timerRef.current);
				setStatus({ type: "error", message: msg });
			}),
		);

		return () => {
			unlisteners.forEach((p) => {
				p.then((u) => u());
			});
			if (timerRef.current) clearInterval(timerRef.current);
		};
	}, [setOutputText, setStatus]);

	const handleStart = useCallback(async () => {
		setRecording(true);
		setElapsed(0);
		setStatus({ type: "recording", elapsed: 0, text: "" });
		await startRecording(
			modelChoice,
			saveAudioPath,
			saveTranscriptPath,
			vadEnabled,
		);
		timerRef.current = window.setInterval(() => {
			setElapsed((prev) => prev + 1);
		}, 1000);
	}, [modelChoice, saveAudioPath, saveTranscriptPath, vadEnabled, setStatus]);

	const handleStop = useCallback(async () => {
		if (timerRef.current) clearInterval(timerRef.current);
		setRecording(false);
		await stopRecording();
	}, []);

	const handleSave = useCallback(async () => {
		if (!outputText) return;
		const path = await pickSaveFile();
		if (path) {
			await saveTextFile(path, outputText);
		}
	}, [outputText]);

	const recordingText = status.type === "recording" ? status.text : null;
	const errorMessage = status.type === "error" ? status.message : null;

	return (
		<Layout
			sidebar={
				<div className="flex flex-col gap-2 px-2">
					<div className="flex flex-col gap-1">
						<Label className="text-xs text-muted-foreground">Model</Label>
						<Select value={modelChoice} onValueChange={setModelChoice}>
							<SelectTrigger className="w-full">
								<SelectValue />
							</SelectTrigger>
							<SelectContent>
								<SelectItem value="Accurate">Accurate (small.en)</SelectItem>
								<SelectItem value="Fast">Fast (base.en)</SelectItem>
							</SelectContent>
						</Select>
					</div>

					<div className="flex flex-col gap-1">
						<Label className="text-xs text-muted-foreground">
							Save audio as
						</Label>
						<Button
							variant="outline"
							className="text-muted-foreground"
							onClick={async () => {
								const p = await pickSaveFile();
								if (p) setSaveAudioPath(p);
							}}
						>
							<FolderOpen className="size-4 shrink-0" />
							<span className="truncate">
								{saveAudioPath
									? saveAudioPath.split("/").pop() ||
										saveAudioPath.split("\\").pop()
									: "Choose"}
							</span>
						</Button>
					</div>

					<div className="flex flex-col gap-1">
						<Label className="text-xs text-muted-foreground">
							Save transcript as
						</Label>
						<Button
							variant="outline"
							className="text-muted-foreground"
							onClick={async () => {
								const p = await pickSaveFile();
								if (p) setSaveTranscriptPath(p);
							}}
						>
							<FolderOpen className="size-4 shrink-0" />
							<span className="truncate">
								{saveTranscriptPath
									? saveTranscriptPath.split("/").pop() ||
										saveTranscriptPath.split("\\").pop()
									: "Choose"}
							</span>
						</Button>
					</div>

					<div className="flex flex-col gap-1">
						<Label className="text-xs text-muted-foreground">
							VAD (Skip silence)
						</Label>
						<Switch checked={vadEnabled} onCheckedChange={setVadEnabled} />
					</div>

					<Separator />

					<div className="flex flex-col items-stretch gap-1">
						{recording ? (
							<>
								<Button size="lg" variant="destructive" onClick={handleStop}>
									<Square className="size-4" />
									Stop
								</Button>
								<p className="text-center text-xs text-destructive tabular-nums">
									Recording: {Math.round(elapsed)}s
								</p>
							</>
						) : (
							<Button size="lg" variant="default" onClick={handleStart}>
								<Mic className="size-4" />
								Start
							</Button>
						)}
					</div>
				</div>
			}
			errorMessage={errorMessage}
		>
			<div className="flex flex-col h-full p-6 gap-4">
				{recordingText && (
					<p className="text-sm text-muted-foreground italic">
						{recordingText}
					</p>
				)}

				{outputText ? (
					<div className="flex-1 flex flex-col gap-3 min-h-0">
						<Textarea
							className="flex-1 font-mono resize-none"
							value={outputText}
							readOnly
						/>
						<Button variant="outline" className="self-end" onClick={handleSave}>
							<Save className="size-4" />
							Save Transcript
						</Button>
					</div>
				) : (
					<div className="flex-1 flex items-center justify-center text-muted-foreground">
						{recording
							? "Listening..."
							: "Press Record to start live transcription"}
					</div>
				)}
			</div>
		</Layout>
	);
}

export default LiveRecord;
