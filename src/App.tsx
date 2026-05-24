import { useState, useEffect, createContext, useContext, useCallback } from "react";
import { checkModels, downloadModel, type MissingModels } from "@/lib/commands";
import { onDownloadProgress, onDownloadDone, type UnlistenFn } from "@/lib/events";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import Landing from "@/components/Landing";
import FileTranscribe from "@/components/FileTranscribe";
import LiveRecord from "@/components/LiveRecord";

export type View = "landing" | "file" | "record";
export type AppStatus =
  | { type: "idle" }
  | { type: "downloading"; label: string; progress: number }
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
  startDownload: (label: string) => void;
}

const AppContext = createContext<AppCtx | null>(null);

export function useApp() {
  const ctx = useContext(AppContext);
  if (!ctx) throw new Error("useApp outside AppContext");
  return ctx;
}

function App() {
  const [view, setView] = useState<View>("landing");
  const [models, setModels] = useState<MissingModels>({ accurate: false, fast: false });
  const [status, setStatus] = useState<AppStatus>({ type: "idle" });
  const [outputText, setOutputText] = useState("");
  const [modelChoice, setModelChoice] = useState("Accurate");

  useEffect(() => {
    checkModels().then(setModels);
  }, []);

  useEffect(() => {
    if (!models.accurate || !models.fast) {
      setView("landing");
    }
  }, [models]);

  useEffect(() => {
    const unlisteners: Promise<UnlistenFn>[] = [];

    unlisteners.push(
      onDownloadProgress((data) => {
        const pct = data.total > 0 ? data.downloaded / data.total : 0;
        setStatus({ type: "downloading", label: data.label, progress: pct });
      }),
    );

    unlisteners.push(
      onDownloadDone(async () => {
        setModels(await checkModels());
        setStatus({ type: "idle" });
      }),
    );

    return () => {
      unlisteners.forEach((p) => p.then((u) => u()));
    };
  }, []);

  const startDownload = useCallback((label: string) => {
    setStatus({ type: "downloading", label, progress: 0 });
    downloadModel(label);
  }, []);

  const missingCount = [models.accurate, models.fast].filter((x) => !x).length;

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
        startDownload,
      }}
    >
      <div className="h-screen w-screen flex flex-col bg-background text-foreground select-none">
        {missingCount > 0 && status.type !== "downloading" ? (
          <div className="flex flex-col items-center justify-center h-full gap-6">
            <h1 className="text-2xl font-bold">Welcome to Rachael's Transcriber</h1>
            <p className="text-muted-foreground">
              {missingCount} model{missingCount > 1 ? "s" : ""} need to be downloaded
            </p>
            <div className="flex gap-4">
              {!models.accurate && (
                <Button variant="default" onClick={() => startDownload("Accurate")}>
                  Download Accurate ({missingCount > 1 ? "465 MB" : "recommended"})
                </Button>
              )}
              {!models.fast && (
                <Button variant="secondary" onClick={() => startDownload("Fast")}>
                  Download Fast ({missingCount > 1 ? "141 MB" : "recommended"})
                </Button>
              )}
            </div>
          </div>
        ) : status.type === "downloading" ? (
          <div className="flex flex-col items-center justify-center h-full gap-4">
            <p className="text-lg">Downloading {status.label} model...</p>
            <div className="w-80">
              <Progress value={Math.round(status.progress * 100)} />
            </div>
            <p className="text-sm text-muted-foreground">
              {Math.round(status.progress * 100)}%
            </p>
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
