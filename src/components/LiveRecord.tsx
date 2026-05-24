import { Mic, Save, Square, X } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useApp } from "@/App";
import { Layout } from "@/components/Layout";
import { Button } from "@/components/ui/button";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import {
	SidebarGroup,
	SidebarGroupContent,
	SidebarGroupLabel,
	SidebarMenu,
	SidebarMenuButton,
	SidebarMenuItem,
} from "@/components/ui/sidebar";
import { Textarea } from "@/components/ui/textarea";
import {
	cancel,
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
		await startRecording(modelChoice);
		timerRef.current = window.setInterval(() => {
			setElapsed((prev) => prev + 1);
		}, 1000);
	}, [modelChoice, setStatus]);

	const handleStop = useCallback(async () => {
		if (timerRef.current) clearInterval(timerRef.current);
		setRecording(false);
		await stopRecording();
	}, []);

	const handleCancel = useCallback(async () => {
		if (timerRef.current) clearInterval(timerRef.current);
		setRecording(false);
		setStatus({ type: "idle" });
		await cancel();
	}, [setStatus]);

	const handleSave = useCallback(async () => {
		if (!outputText) return;
		const path = await pickSaveFile();
		if (path) {
			await saveTextFile(path, outputText);
		}
	}, [outputText]);

	const recordingText = status.type === "recording" ? status.text : null;
	const errorMessage = status.type === "error" ? status.message : null;

	const formatTime = (s: number) => {
		const m = Math.floor(s / 60);
		const sec = s % 60;
		return `${m}:${sec.toString().padStart(2, "0")}`;
	};

	return (
		<Layout
			sidebar={
				<>
					<SidebarGroup>
						<SidebarGroupLabel>Model</SidebarGroupLabel>
						<SidebarGroupContent>
							<div className="px-2">
								<Select value={modelChoice} onValueChange={setModelChoice}>
									<SelectTrigger>
										<SelectValue />
									</SelectTrigger>
									<SelectContent>
										<SelectItem value="Accurate">
											Accurate (small.en)
										</SelectItem>
										<SelectItem value="Fast">Fast (base.en)</SelectItem>
									</SelectContent>
								</Select>
							</div>
						</SidebarGroupContent>
					</SidebarGroup>
					<SidebarGroup>
						<SidebarGroupLabel>Controls</SidebarGroupLabel>
						<SidebarGroupContent>
							<SidebarMenu>
								{recording ? (
									<div className="flex flex-col gap-2 px-2">
										<div className="flex items-center gap-2 text-destructive">
											<div className="size-2 rounded-full bg-destructive animate-pulse" />
											<span className="text-sm font-mono">
												{formatTime(elapsed)}
											</span>
										</div>
										<Button
											variant="destructive"
											size="sm"
											onClick={handleStop}
										>
											<Square className="size-4" />
											Stop
										</Button>
										<Button variant="outline" size="sm" onClick={handleCancel}>
											<X className="size-4" />
											Cancel
										</Button>
									</div>
								) : (
									<SidebarMenuItem>
										<SidebarMenuButton
											tooltip="Start recording"
											onClick={handleStart}
										>
											<Mic />
											<span>Record</span>
										</SidebarMenuButton>
									</SidebarMenuItem>
								)}
							</SidebarMenu>
						</SidebarGroupContent>
					</SidebarGroup>
				</>
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
