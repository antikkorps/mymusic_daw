import { useState } from "react";
import { Layout } from "~/components/layout/Layout";
import { DraggableMidiNote } from "~/components/piano-roll/DraggableMidiNote";
import { mockMidiNotes } from "~/lib/mockData";
import { midiNoteToName } from "~/types/midi";
import { useToast } from "~/lib/toast";
import type { MidiNote, GridSnap } from "~/types/midi";
import { cn } from "~/lib/utils";

const GRID_SNAPS: GridSnap[] = ["1/4", "1/8", "1/16", "1/32"];
const NOTE_HEIGHT = 16;
const BEAT_WIDTH = 80;

// Grid snap denominators for converting snap to duration in beats
const SNAP_DENOMINATORS: Record<GridSnap, number> = {
  "1/1": 1,
  "1/2": 0.5,
  "1/4": 0.25,
  "1/8": 0.125,
  "1/16": 0.0625,
  "1/32": 0.03125,
  "1/64": 0.015625,
};

export default function PianoRollPage() {
  const [notes, setNotes] = useState<MidiNote[]>(mockMidiNotes);
  const [gridSnap, setGridSnap] = useState<GridSnap>("1/16");
  const [selectedNotes, setSelectedNotes] = useState<Set<string>>(new Set());
  const { showToast } = useToast();

  const minNote = 48; // C3
  const maxNote = 84; // C6
  const noteRange = maxNote - minNote + 1;
  const totalBeats = 16;

  const toggleNoteSelection = (noteId: string) => {
    setSelectedNotes((prev) => {
      const next = new Set(prev);
      if (next.has(noteId)) {
        next.delete(noteId);
      } else {
        next.add(noteId);
      }
      return next;
    });
  };

  const deleteNote = (noteId: string) => {
    setNotes((prev) => prev.filter((n) => n.id !== noteId));
    setSelectedNotes((prev) => {
      const next = new Set(prev);
      next.delete(noteId);
      return next;
    });
    showToast("Note deleted", "info", 1000);
  };

  const deleteSelectedNotes = () => {
    if (selectedNotes.size === 0) return;
    setNotes((prev) => prev.filter((n) => !selectedNotes.has(n.id)));
    setSelectedNotes(new Set());
    showToast(`Deleted ${selectedNotes.size} note(s)`, "success", 1500);
  };

  const handleNoteMove = (noteId: string, newStartTime: number, newPitch: number) => {
    setNotes((prev) =>
      prev.map((n) =>
        n.id === noteId ? { ...n, startTime: newStartTime, pitch: newPitch } : n
      )
    );
  };

  const handleNoteResize = (noteId: string, newDuration: number) => {
    setNotes((prev) =>
      prev.map((n) => (n.id === noteId ? { ...n, duration: newDuration } : n))
    );
  };

  // Convert grid snap to duration in beats
  const getSnapDuration = (): number => {
    return SNAP_DENOMINATORS[gridSnap];
  };

  const snapToGrid = (time: number): number => {
    const snapValue = getSnapDuration();
    return Math.round(time / snapValue) * snapValue;
  };

  const handleGridClick = (e: React.MouseEvent<HTMLDivElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    // Calculate time and pitch
    const time = snapToGrid(x / BEAT_WIDTH);
    const pitch = maxNote - Math.floor(y / NOTE_HEIGHT);

    // Check if clicking on existing note
    const clickedNote = notes.find(
      (note) =>
        note.pitch === pitch &&
        time >= note.startTime &&
        time < note.startTime + note.duration
    );

    if (clickedNote) {
      toggleNoteSelection(clickedNote.id);
      return;
    }

    // Add new note
    const newNote: MidiNote = {
      id: `note-${crypto.randomUUID()}`,
      pitch,
      velocity: 100,
      startTime: time,
      duration: getSnapDuration(),
      channel: 0,
    };

    setNotes((prev) => [...prev, newNote]);
    showToast(`Added ${midiNoteToName(pitch)}`, "success", 1000);
  };

  const isWhiteKey = (midiNote: number) => {
    const note = midiNote % 12;
    return [0, 2, 4, 5, 7, 9, 11].includes(note);
  };

  // Handle keyboard shortcuts
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Delete" || e.key === "Backspace") {
      deleteSelectedNotes();
    } else if (e.key === "a" && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      setSelectedNotes(() => new Set(notes.map((n) => n.id)));
      showToast("Selected all notes", "info", 1000);
    }
  };

  return (
    <Layout>
      <div className="h-full flex flex-col bg-zinc-900" onKeyDown={handleKeyDown} tabIndex={0}>
        {/* Piano Roll Header */}
        <div className="p-4 border-b border-zinc-800 flex items-center justify-between">
          <div>
            <h2 className="text-2xl font-bold text-white">Piano Roll</h2>
            <p className="text-zinc-400 text-sm mt-1">
              Click to add • Drag to move • Resize to change duration • Delete to remove
            </p>
          </div>

          {/* Grid Snap Controls */}
          <div className="flex items-center gap-2">
            <label className="text-sm text-zinc-400">Grid Snap</label>
            <div className="flex gap-1">
              {GRID_SNAPS.map((snap) => (
                <button
                  key={snap}
                  onClick={() => setGridSnap(snap)}
                  className={cn(
                    "px-3 py-1 rounded text-sm font-mono transition-colors",
                    gridSnap === snap
                      ? "bg-cyan-500 text-white"
                      : "bg-zinc-800 text-zinc-400 hover:bg-zinc-700"
                  )}
                >
                  {snap}
                </button>
              ))}
            </div>
          </div>
        </div>

        {/* Piano Roll Content */}
        <div className="flex-1 flex overflow-hidden">
          {/* Piano Keys */}
          <div className="w-20 bg-zinc-950 border-r border-zinc-800 overflow-y-auto">
            <div className="flex flex-col-reverse">
              {Array.from({ length: noteRange }).map((_, i) => {
                const midiNote = minNote + i;
                const isWhite = isWhiteKey(midiNote);

                return (
                  <div
                    key={midiNote}
                    className={cn(
                      "flex items-center justify-center border-b border-zinc-800 cursor-pointer hover:bg-cyan-900/20",
                      isWhite ? "bg-zinc-900" : "bg-zinc-950"
                    )}
                    style={{ height: NOTE_HEIGHT }}
                  >
                    <span className="text-xs font-mono text-zinc-400">
                      {midiNoteToName(midiNote)}
                    </span>
                  </div>
                );
              })}
            </div>
          </div>

          {/* Note Grid */}
          <div className="flex-1 overflow-auto">
            <div className="relative" style={{ width: totalBeats * BEAT_WIDTH }}>
              {/* Grid Background */}
              <div
                className="absolute inset-0 cursor-crosshair"
                style={{ height: noteRange * NOTE_HEIGHT }}
                onClick={handleGridClick}
              >
                {/* Horizontal lines (notes) */}
                <div className="flex flex-col-reverse">
                  {Array.from({ length: noteRange }).map((_, i) => {
                    const midiNote = minNote + i;
                    const isWhite = isWhiteKey(midiNote);

                    return (
                      <div
                        key={midiNote}
                        className={cn(
                          "border-b border-zinc-900",
                          isWhite ? "bg-zinc-950" : "bg-zinc-900"
                        )}
                        style={{ height: NOTE_HEIGHT }}
                      />
                    );
                  })}
                </div>

                {/* Vertical lines (beats) */}
                <div className="absolute inset-0 flex pointer-events-none">
                  {Array.from({ length: totalBeats * 4 }).map((_, i) => {
                    const isBeat = i % 4 === 0;
                    return (
                      <div
                        key={i}
                        className={cn(
                          "border-r",
                          isBeat ? "border-zinc-700" : "border-zinc-900"
                        )}
                        style={{ width: BEAT_WIDTH / 4 }}
                      />
                    );
                  })}
                </div>
              </div>

              {/* MIDI Notes */}
              <div
                className="absolute inset-0 pointer-events-none"
                style={{ height: noteRange * NOTE_HEIGHT }}
              >
                {notes.map((note) => (
                  <DraggableMidiNote
                    key={note.id}
                    note={note}
                    isSelected={selectedNotes.has(note.id)}
                    beatWidth={BEAT_WIDTH}
                    noteHeight={NOTE_HEIGHT}
                    maxNote={maxNote}
                    onSelect={toggleNoteSelection}
                    onMove={handleNoteMove}
                    onResize={handleNoteResize}
                    onDelete={deleteNote}
                    snapToGrid={snapToGrid}
                  />
                ))}
              </div>
            </div>
          </div>
        </div>

        {/* Info Bar */}
        <div className="h-8 bg-zinc-950 border-t border-zinc-800 flex items-center px-4 text-xs text-zinc-500">
          {selectedNotes.size > 0 ? (
            <span>
              {selectedNotes.size} note{selectedNotes.size > 1 ? "s" : ""} selected • Press Delete to remove
            </span>
          ) : (
            <span>
              Grid: {gridSnap} • {notes.length} notes • Click to add • Drag to move • Resize from right edge • Double-click to delete
            </span>
          )}
        </div>
      </div>
    </Layout>
  );
}
