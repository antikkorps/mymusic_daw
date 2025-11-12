import { useState, useCallback } from "react";
import { Sidebar } from "./Sidebar";
import { Toolbar } from "./Toolbar";
import { StatusBar } from "./StatusBar";
import { PageTransition } from "./PageTransition";
import { useKeyboardShortcuts, type ShortcutAction } from "~/hooks/useKeyboardShortcuts";
import { useToast } from "~/lib/toast";
import type { TransportState } from "~/types/audio";
import { mockTransportState } from "~/lib/mockData";

interface LayoutProps {
  children: React.ReactNode;
}

export function Layout({ children }: LayoutProps) {
  const [transport, setTransport] = useState<TransportState>(mockTransportState);
  const { showToast } = useToast();

  const updateTransport = (updates: Partial<TransportState>) => {
    setTransport((prev) => ({ ...prev, ...updates }));
  };

  const handleShortcutAction = useCallback((action: ShortcutAction) => {
    switch (action) {
      case "play":
        setTransport((prev) => {
          const newPlaying = !prev.playing;
          showToast(newPlaying ? "Playing" : "Paused", "info", 1000);
          return { ...prev, playing: newPlaying };
        });
        break;
      case "stop":
        setTransport((prev) => {
          showToast("Stopped and reset", "info", 1000);
          return { ...prev, playing: false, recording: false, position: 0 };
        });
        break;
      case "record":
        setTransport((prev) => {
          const newRecording = !prev.recording;
          showToast(newRecording ? "Recording" : "Recording stopped", newRecording ? "error" : "info", 1000);
          return { ...prev, recording: newRecording };
        });
        break;
      case "save":
        showToast("Project saved (mock)", "success");
        break;
      case "undo":
        showToast("Undo (not implemented)", "warning", 1500);
        break;
      case "redo":
        showToast("Redo (not implemented)", "warning", 1500);
        break;
      default:
        break;
    }
  }, [showToast]);

  useKeyboardShortcuts(handleShortcutAction);

  return (
    <div className="flex h-screen overflow-hidden bg-zinc-900">
      <Sidebar />
      <div className="flex-1 flex flex-col overflow-hidden">
        <Toolbar transport={transport} onTransportChange={updateTransport} />
        <main className="flex-1 overflow-auto">
          <PageTransition>{children}</PageTransition>
        </main>
        <StatusBar />
      </div>
    </div>
  );
}
