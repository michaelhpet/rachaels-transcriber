import { useState, useEffect, useCallback } from "react";
import { useApp } from "@/App";
import {
  pickAudioFile,
  pickSaveFile,
  transcribeFile,
  cancel,
  saveTextFile,
} from "@/lib/commands";
import {
  onTranscribeProgress,
  onTranscribeDone,
  onTranscribeError,
  type UnlistenFn,
} from "@/lib/events";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { Textarea } from "@/components/ui/textarea";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { ArrowLeft, FolderOpen, Play, X, Save } from "lucide-react";

function FileTranscribe() {
  const { setView, status, setStatus, outputText, setOutputText, modelChoice, setModelChoice } =
    useApp();
  const [filePath, setFilePath] = useState<string | null>(null);
  const [transcribing, setTranscribing] = useState(false);

  useEffect(() => {
    const unlisteners: Promise<UnlistenFn>[] = [];

    unlisteners.push(
      onTranscribeProgress((data) => {
        setStatus({ type: "transcribing", progress: data.progress, text: data.text });
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
      unlisteners.forEach((p) => p.then((u) => u()));
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

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center gap-4 p-4 border-b border-border">
        <Button variant="ghost" onClick={() => setView("landing")}>
          <ArrowLeft className="size-4" />
          Back
        </Button>
        <h1 className="text-xl font-bold">File Transcription</h1>
      </div>

      <div className="flex-1 flex flex-col items-center justify-center gap-6 p-8">
        <div className="flex gap-4 items-center">
          <Button variant="default" onClick={handlePick}>
            <FolderOpen className="size-4" />
            {filePath ? "Change File" : "Select Audio File"}
          </Button>
          {filePath && (
            <span className="text-sm text-muted-foreground truncate max-w-64">
              {filePath.split("/").pop() || filePath.split("\\").pop()}
            </span>
          )}
        </div>

        <div className="flex gap-4 items-center">
          <label className="text-sm text-muted-foreground">Model:</label>
          <Select value={modelChoice} onValueChange={setModelChoice}>
            <SelectTrigger className="w-48">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="Accurate">Accurate (small.en)</SelectItem>
              <SelectItem value="Fast">Fast (base.en)</SelectItem>
            </SelectContent>
          </Select>
        </div>

        {filePath && !transcribing && (
          <Button variant="default" size="lg" onClick={handleTranscribe}>
            <Play className="size-4" />
            Transcribe
          </Button>
        )}

        {transcribing && (
          <div className="flex flex-col items-center gap-4 w-full max-w-lg">
            <Progress
              value={Math.round(status.type === "transcribing" ? (status as any).progress * 100 : 0)}
              className="w-full"
            />
            <Button variant="destructive" onClick={handleCancel}>
              <X className="size-4" />
              Cancel
            </Button>
          </div>
        )}

        {outputText && (
          <div className="w-full max-w-2xl flex flex-col gap-4">
            <Textarea
              className="w-full h-48 font-mono resize-none"
              value={outputText}
              readOnly
            />
            <Button variant="outline" className="self-end" onClick={handleSave}>
              <Save className="size-4" />
              Save to File
            </Button>
          </div>
        )}

        {status.type === "error" && (
          <p className="text-destructive">{(status as any).message}</p>
        )}
      </div>
    </div>
  );
}

export default FileTranscribe;
