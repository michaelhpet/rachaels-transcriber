import { useState, useEffect, useCallback, useRef } from "react";
import { useApp } from "@/App";
import { startRecording, stopRecording, pickSaveFile, cancel, saveTextFile } from "@/lib/commands";
import {
  onRecordProgress,
  onTranscribeDone,
  onTranscribeError,
  type UnlistenFn,
} from "@/lib/events";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { ArrowLeft, Mic, Square, X, Save } from "lucide-react";

function LiveRecord() {
  const { setView, status, setStatus, outputText, setOutputText, modelChoice, setModelChoice } =
    useApp();
  const [recording, setRecording] = useState(false);
  const [elapsed, setElapsed] = useState(0);
  const timerRef = useRef<number | null>(null);

  useEffect(() => {
    const unlisteners: Promise<UnlistenFn>[] = [];

    unlisteners.push(
      onRecordProgress((data) => {
        setElapsed(data.elapsed);
        setStatus({ type: "recording", elapsed: data.elapsed, text: data.text });
      }),
    );

    unlisteners.push(
      onTranscribeDone((text) => {
        setRecording(false);
        if (timerRef.current) clearInterval(timerRef.current);
        setOutputText((prev) => prev + text + "\n");
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
      unlisteners.forEach((p) => p.then((u) => u()));
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

  const formatTime = (s: number) => {
    const m = Math.floor(s / 60);
    const sec = s % 60;
    return `${m}:${sec.toString().padStart(2, "0")}`;
  };

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center gap-4 p-4 border-b border-border">
        <Button variant="ghost" onClick={() => setView("landing")}>
          <ArrowLeft className="size-4" />
          Back
        </Button>
        <h1 className="text-xl font-bold">Live Recording</h1>
      </div>

      <div className="flex-1 flex flex-col items-center justify-center gap-6 p-8">
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

        {recording ? (
          <div className="flex flex-col items-center gap-4">
            <div className="flex flex-col items-center gap-2">
              <div className="size-20 rounded-full bg-destructive flex items-center justify-center animate-pulse">
                <span className="text-destructive-foreground text-lg font-bold">
                  {formatTime(elapsed)}
                </span>
              </div>
            </div>
            <div className="flex gap-4">
              <Button variant="destructive" size="lg" onClick={handleStop}>
                <Square className="size-4" />
                Stop
              </Button>
              <Button variant="outline" onClick={handleCancel}>
                <X className="size-4" />
                Cancel
              </Button>
            </div>
          </div>
        ) : (
          <Button
            variant="destructive"
            size="icon"
            className="size-20 rounded-full"
            onClick={handleStart}
          >
            <Mic className="size-8" />
          </Button>
        )}

        {status.type === "recording" && (status as any).text && (
          <p className="text-muted-foreground text-sm max-w-lg text-center italic">
            {(status as any).text}
          </p>
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
              Save Transcript
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

export default LiveRecord;
