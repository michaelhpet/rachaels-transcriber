import { useApp } from "@/App";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";
import { FileText, Mic } from "lucide-react";

function Landing() {
  const { setView } = useApp();

  return (
    <div className="flex items-center justify-center h-full">
      <div className="flex gap-4">
        <Card
          size="default"
          className="w-60 h-60 cursor-pointer hover:bg-muted/50 transition-colors"
          onClick={() => setView("file")}
        >
          <CardHeader>
            <CardTitle className="flex flex-col items-center gap-3 text-lg font-semibold">
              <FileText className="size-8 text-blue-500" />
              File Transcription
            </CardTitle>
          </CardHeader>
          <CardContent className="text-sm text-muted-foreground text-center">
            Transcribe audio files
          </CardContent>
        </Card>
        <Card
          size="default"
          className="w-60 h-60 cursor-pointer hover:bg-muted/50 transition-colors"
          onClick={() => setView("record")}
        >
          <CardHeader>
            <CardTitle className="flex flex-col items-center gap-3 text-lg font-semibold">
              <Mic className="size-8 text-red-400" />
              Live Recording
            </CardTitle>
          </CardHeader>
          <CardContent className="text-sm text-muted-foreground text-center">
            Record and transcribe live
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

export default Landing;
