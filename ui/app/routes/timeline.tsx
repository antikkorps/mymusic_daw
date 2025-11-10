import { useState, useEffect, useCallback } from "react";
import { Layout } from "~/components/layout/Layout";
import { DraggableClip } from "~/components/timeline/DraggableClip";
import { mockTracks, mockClips } from "~/lib/mockData";
import { useToast } from "~/lib/toast";
import type { Track, Clip } from "~/types/audio";
import { cn } from "~/lib/utils";

export default function TimelinePage() {
  const [tracks] = useState<Track[]>(mockTracks);
  const [clips, setClips] = useState<Clip[]>(mockClips);
  const [zoom, setZoom] = useState(50); // pixels per second
  const [selectedClips, setSelectedClips] = useState<Set<string>>(new Set());
  const { showToast } = useToast();

  const maxTime = Math.max(...clips.map((c) => c.startTime + c.duration), 16);
  const timelineWidth = maxTime * zoom;

  const handleClipSelection = (clipId: string, shiftKey: boolean) => {
    setSelectedClips((prev) => {
      const next = new Set(prev);

      if (shiftKey) {
        // Shift+Click: toggle this clip in the selection
        if (next.has(clipId)) {
          next.delete(clipId);
        } else {
          next.add(clipId);
        }
      } else {
        // Normal click: select only this clip
        next.clear();
        next.add(clipId);
      }

      return next;
    });
  };

  const handleClipMove = (clipId: string, newStartTime: number, silent = false) => {
    const clip = clips.find((c) => c.id === clipId);

    setClips((prev) =>
      prev.map((c) =>
        c.id === clipId ? { ...c, startTime: newStartTime } : c
      )
    );

    // Show toast notification when drag is complete (unless silent)
    if (clip && !silent) {
      showToast(`Moved "${clip.name}" to ${newStartTime.toFixed(2)}s`, "success", 1000);
    }
  };

  const handleClipResize = (clipId: string, newDuration: number) => {
    setClips((prev) =>
      prev.map((clip) =>
        clip.id === clipId ? { ...clip, duration: newDuration } : clip
      )
    );
  };

  const handleClipDuplicate = (clipId: string) => {
    setClips((prev) => {
      const clip = prev.find((c) => c.id === clipId);
      if (!clip) return prev;

      const newClip: Clip = {
        ...clip,
        id: `clip-${Date.now()}`,
        startTime: clip.startTime + clip.duration + 0.5, // Place right after with gap
      };

      showToast(`Duplicated "${clip.name}"`, "success", 1500);
      return [...prev, newClip];
    });
  };

  const handleClipDelete = (clipId: string) => {
    setClips((prev) => {
      const clip = prev.find((c) => c.id === clipId);
      
      if (clip) {
        showToast(`Deleted "${clip.name}"`, "info", 1500);
      }
      
      return prev.filter((c) => c.id !== clipId);
    });
    
    setSelectedClips((prev) => {
      const next = new Set(prev);
      next.delete(clipId);
      return next;
    });
  };

  const handleClipSplit = (clipId: string) => {
    setClips((prev) => {
      const clip = prev.find((c) => c.id === clipId);
      if (!clip) return prev;

      // Split at middle for demo
      const splitPoint = clip.duration / 2;

      const firstHalf: Clip = {
        ...clip,
        id: `${clip.id}-1`,
        duration: splitPoint,
      };

      const secondHalf: Clip = {
        ...clip,
        id: `${clip.id}-2`,
        startTime: clip.startTime + splitPoint,
        duration: clip.duration - splitPoint,
      };

      showToast(`Split "${clip.name}" at ${splitPoint.toFixed(2)}s`, "success", 1500);
      
      return [
        ...prev.filter((c) => c.id !== clipId),
        firstHalf,
        secondHalf,
      ];
    });
  };

  // Group operations
  const deleteSelectedClips = useCallback(() => {
    if (selectedClips.size === 0) return;

    const count = selectedClips.size;
    setClips((prev) => prev.filter((c) => !selectedClips.has(c.id)));
    setSelectedClips(new Set());
    showToast(`Deleted ${count} clip${count > 1 ? "s" : ""}`, "info", 1500);
  }, [selectedClips, showToast]);

  const duplicateSelectedClips = useCallback(() => {
    if (selectedClips.size === 0) return;

    const newClips: Clip[] = [];
    clips.forEach((clip) => {
      if (selectedClips.has(clip.id)) {
        newClips.push({
          ...clip,
          id: `clip-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
          startTime: clip.startTime + clip.duration + 0.5,
        });
      }
    });

    setClips((prev) => [...prev, ...newClips]);
    showToast(
      `Duplicated ${newClips.length} clip${newClips.length > 1 ? "s" : ""}`,
      "success",
      1500
    );
  }, [clips, selectedClips, showToast]);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Don't trigger if typing in input
      const target = e.target as HTMLElement;
      if (target.tagName === "INPUT" || target.tagName === "TEXTAREA") return;

      if ((e.key === "Delete" || e.key === "Backspace") && selectedClips.size > 0) {
        e.preventDefault();
        deleteSelectedClips();
      }

      if (e.ctrlKey && e.key === "d" && selectedClips.size > 0) {
        e.preventDefault();
        duplicateSelectedClips();
      }

      if (e.ctrlKey && e.key === "a") {
        e.preventDefault();
        setSelectedClips(new Set(clips.map((c) => c.id)));
        showToast(`Selected all ${clips.length} clips`, "info", 1000);
      }

      if (e.key === "Escape" && selectedClips.size > 0) {
        e.preventDefault();
        setSelectedClips(new Set());
        showToast("Deselected all clips", "info", 1000);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [selectedClips, clips, deleteSelectedClips, duplicateSelectedClips, showToast]);

  return (
    <Layout>
      <div className="h-full flex flex-col bg-zinc-900">
        {/* Timeline Header */}
        <div className="p-4 border-b border-zinc-800 flex items-center justify-between">
          <div>
            <h2 className="text-2xl font-bold text-white">Timeline</h2>
            <p className="text-zinc-400 text-sm mt-1">
              Drag clips to arrange • Snaps to 0.25s grid
            </p>
          </div>

          {/* Zoom Controls */}
          <div className="flex items-center gap-4">
            <label className="text-sm text-zinc-400">Zoom</label>
            <input
              type="range"
              min={20}
              max={100}
              value={zoom}
              onChange={(e) => setZoom(parseInt(e.target.value))}
              className="w-32"
            />
            <span className="text-sm text-zinc-500 font-mono">{zoom}px/s</span>
          </div>
        </div>

        {/* Timeline Content */}
        <div className="flex-1 flex overflow-hidden">
          {/* Track Names */}
          <div className="w-48 bg-zinc-950 border-r border-zinc-800 overflow-y-auto">
            <div className="h-12 border-b border-zinc-800 flex items-center px-4 bg-zinc-900">
              <span className="text-xs text-zinc-500 font-semibold uppercase">Tracks</span>
            </div>
            {tracks.map((track) => (
              <div
                key={track.id}
                className="h-20 border-b border-zinc-800 flex items-center px-4 gap-3"
              >
                <div
                  className="w-3 h-3 rounded-full flex-shrink-0"
                  style={{ backgroundColor: track.color }}
                />
                <div className="flex-1 min-w-0">
                  <div className="text-sm font-medium truncate">{track.name}</div>
                  <div className="text-xs text-zinc-500">{track.type}</div>
                </div>
              </div>
            ))}
          </div>

          {/* Timeline Grid and Clips */}
          <div className="flex-1 overflow-auto">
            <div className="relative" style={{ width: timelineWidth + 100 }}>
              {/* Time Ruler */}
              <div className="sticky top-0 z-10 h-12 bg-zinc-900 border-b border-zinc-800 flex items-center">
                {Array.from({ length: Math.ceil(maxTime) + 1 }).map((_, i) => (
                  <div
                    key={i}
                    className="absolute flex flex-col items-start"
                    style={{ left: i * zoom }}
                  >
                    <span className="text-xs text-zinc-500 font-mono">{i}s</span>
                    <div className="w-px h-2 bg-zinc-700" />
                  </div>
                ))}
              </div>

              {/* Track Lanes */}
              {tracks.map((track) => (
                <div
                  key={track.id}
                  className="relative h-20 border-b border-zinc-800 bg-zinc-950"
                >
                  {/* Grid lines */}
                  <div className="absolute inset-0 flex">
                    {Array.from({ length: Math.ceil(maxTime) + 1 }).map((_, i) => (
                      <div
                        key={i}
                        className="border-r border-zinc-900"
                        style={{ width: zoom }}
                      />
                    ))}
                  </div>

                  {/* Clips for this track */}
                  {clips
                    .filter((clip) => clip.trackId === track.id)
                    .map((clip) => (
                      <DraggableClip
                        key={clip.id}
                        clip={clip}
                        zoom={zoom}
                        isSelected={selectedClips.has(clip.id)}
                        onSelect={handleClipSelection}
                        onMove={handleClipMove}
                        onResize={handleClipResize}
                        onDuplicate={handleClipDuplicate}
                        onDelete={handleClipDelete}
                        onSplit={handleClipSplit}
                      />
                    ))}
                </div>
              ))}
            </div>
          </div>
        </div>

        {/* Info Bar */}
        <div className="h-8 bg-zinc-950 border-t border-zinc-800 flex items-center px-4 text-xs text-zinc-500">
          {selectedClips.size > 0 ? (
            <span>
              {selectedClips.size} clip{selectedClips.size > 1 ? "s" : ""} selected •{" "}
              <span className="text-zinc-400">Del</span> to delete •{" "}
              <span className="text-zinc-400">Ctrl+D</span> to duplicate •{" "}
              <span className="text-zinc-400">Esc</span> to deselect
            </span>
          ) : (
            <span>
              Click to select • Hold <span className="text-zinc-400">Shift</span> for multi-select •{" "}
              <span className="text-zinc-400">Ctrl+A</span> to select all
            </span>
          )}
        </div>
      </div>
    </Layout>
  );
}
