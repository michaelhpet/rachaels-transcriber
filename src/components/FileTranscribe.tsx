import { FolderOpen, Play, Save, X } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useApp } from "@/App";
import { Layout } from "@/components/Layout";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
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
	pickAudioFile,
	pickSaveFile,
	saveTextFile,
	transcribeFile,
} from "@/lib/commands";
import {
	onTranscribeDone,
	onTranscribeError,
	onTranscribeProgress,
	type UnlistenFn,
} from "@/lib/events";

function FileTranscribe() {
	const {
		status,
		setStatus,
		outputText,
		setOutputText,
		modelChoice,
		setModelChoice,
	} = useApp();
	const [filePath, setFilePath] = useState<string | null>(null);
	const [transcribing, setTranscribing] = useState(false);

	useEffect(() => {
		const unlisteners: Promise<UnlistenFn>[] = [];

		unlisteners.push(
			onTranscribeProgress((data) => {
				setStatus({
					type: "transcribing",
					progress: data.progress,
					text: data.text,
				});
			}),
		);

		unlisteners.push(
			onTranscribeDone((text) => {
				setOutputText(text);
				setStatus({ type: "done", text });
				setTranscribing(false);
			}),
		);

		unlisteners.push(
			onTranscribeError((msg) => {
				setStatus({ type: "error", message: msg });
				setTranscribing(false);
			}),
		);

		return () => {
			unlisteners.forEach((p) => {
				p.then((u) => u());
			});
		};
	}, [setOutputText, setStatus]);

	const handlePick = useCallback(async () => {
		const path = await pickAudioFile();
		if (path) setFilePath(path);
	}, []);

	const handleTranscribe = useCallback(async () => {
		if (!filePath) return;
		setTranscribing(true);
		setStatus({ type: "transcribing", progress: 0, text: "" });
		await transcribeFile(filePath, modelChoice);
	}, [filePath, modelChoice, setStatus]);

	const handleCancel = useCallback(async () => {
		await cancel();
		setTranscribing(false);
		setStatus({ type: "idle" });
	}, [setStatus]);

	const handleSave = useCallback(async () => {
		if (!outputText) return;
		const path = await pickSaveFile();
		if (path) {
			await saveTextFile(path, outputText);
		}
	}, [outputText]);

	const transcribePct =
		status.type === "transcribing" ? status.progress * 100 : 0;
	const errorMessage = status.type === "error" ? status.message : null;
	const fileName =
		filePath && (filePath.split("/").pop() || filePath.split("\\").pop());

	return (
		<Layout
			sidebar={
				<>
					<SidebarGroup>
						<SidebarGroupLabel>File</SidebarGroupLabel>
						<SidebarGroupContent>
							<SidebarMenu>
								<SidebarMenuItem>
									<SidebarMenuButton
										tooltip="Select Audio File"
										onClick={handlePick}
									>
										<FolderOpen />
										<span>{fileName || "Select Audio File"}</span>
									</SidebarMenuButton>
								</SidebarMenuItem>
							</SidebarMenu>
						</SidebarGroupContent>
					</SidebarGroup>
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
						<SidebarGroupContent>
							<SidebarMenu>
								{!transcribing && filePath && (
									<SidebarMenuItem>
										<SidebarMenuButton
											tooltip="Start transcription"
											onClick={handleTranscribe}
										>
											<Play />
											<span>Transcribe</span>
										</SidebarMenuButton>
									</SidebarMenuItem>
								)}
								{transcribing && (
									<div className="flex flex-col gap-2 px-2">
										<Progress value={Math.round(transcribePct)} />
										<Button
											variant="destructive"
											size="sm"
											onClick={handleCancel}
										>
											<X className="size-4" />
											Cancel
										</Button>
									</div>
								)}
							</SidebarMenu>
						</SidebarGroupContent>
					</SidebarGroup>
				</>
			}
			errorMessage={errorMessage}
		>
			<div className="flex flex-col h-full p-6 gap-4">
				{transcribing && (
					<div>
						<p className="text-sm text-muted-foreground">
							Transcribing... {Math.round(transcribePct)}%
						</p>
					</div>
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
							Save to File
						</Button>
					</div>
				) : (
					<div className="flex-1 flex items-center justify-center text-muted-foreground">
						Select an audio file and start transcription
					</div>
				)}
			</div>
		</Layout>
	);
}

export default FileTranscribe;
