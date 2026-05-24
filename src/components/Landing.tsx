import { FileText, Mic } from "lucide-react";
import { useApp } from "@/App";

function Landing() {
	const { setView } = useApp();

	return (
		<div className="flex items-center justify-center h-full">
			<div className="flex gap-4">
				<button
					type="button"
					onClick={() => setView("file")}
					className="size-60 flex items-center justify-center shrink-0 border border-foreground/20 rounded-lg hover:bg-foreground/5"
				>
					<div>
						<p className="flex flex-col items-center gap-1">
							<FileText className="size-8 text-blue-500" />
							File Transcription
						</p>
					</div>
				</button>
				<button
					type="button"
					className="size-60 flex items-center justify-center shrink-0 border border-foreground/20 rounded-lg hover:bg-foreground/5"
					onClick={() => setView("record")}
				>
					<div>
						<p className="flex flex-col items-center gap-1">
							<Mic className="size-8 text-red-400" />
							Live Recording
						</p>
					</div>
				</button>
			</div>
		</div>
	);
}

export default Landing;
