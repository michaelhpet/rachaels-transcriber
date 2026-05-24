import { createContext, useContext, useEffect, useState } from "react";
import FileTranscribe from "@/components/FileTranscribe";
import Landing from "@/components/Landing";
import LiveRecord from "@/components/LiveRecord";
import { Progress } from "@/components/ui/progress";
import {
	checkModels,
	downloadModels,
	type MissingModels,
} from "@/lib/commands";
import {
	onDownloadDone,
	onDownloadProgress,
	type UnlistenFn,
} from "@/lib/events";

export type View = "landing" | "file" | "record";
export type AppStatus =
	| { type: "idle" }
	| { type: "downloading"; progress: number }
	| { type: "transcribing"; progress: number; text: string }
	| { type: "recording"; elapsed: number; text: string }
	| { type: "done"; text: string }
	| { type: "error"; message: string };

interface AppCtx {
	view: View;
	setView: (v: View) => void;
	models: MissingModels;
	status: AppStatus;
	setStatus: (s: AppStatus) => void;
	outputText: string;
	setOutputText: (t: string | ((prev: string) => string)) => void;
	modelChoice: string;
	setModelChoice: (m: string) => void;
}

const AppContext = createContext<AppCtx | null>(null);

export function useApp() {
	const ctx = useContext(AppContext);
	if (!ctx) throw new Error("useApp outside AppContext");
	return ctx;
}

function App() {
	const [view, setView] = useState<View>("landing");
	const [models, setModels] = useState<MissingModels>({
		accurate: false,
		fast: false,
	});
	const [status, setStatus] = useState<AppStatus>({ type: "idle" });
	const [outputText, setOutputText] = useState("");
	const [modelChoice, setModelChoice] = useState("Accurate");

	const progress = status?.type === "downloading" ? status.progress : 0;
	const modelsReady = models.accurate && models.fast;

	useEffect(() => {
		checkModels().then((m) => {
			setModels(m);
			if (!m.accurate || !m.fast) {
				setStatus({ type: "downloading", progress: 0 });
				downloadModels();
			}
		});
	}, []);

	useEffect(() => {
		const unlisteners: Promise<UnlistenFn>[] = [];

		unlisteners.push(
			onDownloadProgress((data) => {
				setStatus({
					type: "downloading",
					progress: data.total > 0 ? data.downloaded / data.total : 0,
				});
			}),
		);

		unlisteners.push(
			onDownloadDone(async () => {
				setModels(await checkModels());
				setStatus({ type: "idle" });
			}),
		);

		return () => {
			unlisteners.forEach((p) => {
				p.then((u) => u());
			});
		};
	}, []);

	return (
		<AppContext.Provider
			value={{
				view,
				setView,
				models,
				status,
				setStatus,
				outputText,
				setOutputText,
				modelChoice,
				setModelChoice,
			}}
		>
			<div className="h-screen w-screen flex flex-col bg-background text-foreground select-none">
				{modelsReady ? (
					<div className="flex flex-col items-center justify-center h-full gap-4">
						<p className="text-lg">Preparing models...</p>
						<div className="w-80">
							<Progress value={Math.round(progress * 100)} />
						</div>
						{status.type === "downloading" && (
							<p className="text-sm text-muted-foreground">
								{Math.round(progress * 100)}%
							</p>
						)}
					</div>
				) : view === "landing" ? (
					<Landing />
				) : view === "file" ? (
					<FileTranscribe />
				) : (
					<LiveRecord />
				)}
			</div>
		</AppContext.Provider>
	);
}

export default App;
