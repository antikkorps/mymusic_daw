import { motion } from "framer-motion";
import { useState, memo } from "react";
import { cn } from "~/lib/utils";
import type { MidiNote } from "~/types/midi";
import { MIN_NOTE_DURATION } from "~/types/midi";

interface DraggableMidiNoteProps {
  note: MidiNote;
  isSelected: boolean;
  beatWidth: number;
  noteHeight: number;
  maxNote: number;
  onSelect: (noteId: string) => void;
  onMove: (noteId: string, newStartTime: number, newPitch: number) => void;
  onResize: (noteId: string, newDuration: number) => void;
  onDelete: (noteId: string) => void;
  snapToGrid: (time: number) => number;
}

export const DraggableMidiNote = memo(function DraggableMidiNote({
  note,
  isSelected,
  beatWidth,
  noteHeight,
  maxNote,
  onSelect,
  onMove,
  onResize,
  onDelete,
  snapToGrid,
}: DraggableMidiNoteProps) {
  const [isDragging, setIsDragging] = useState(false);
  const [isResizing, setIsResizing] = useState(false);
  const [dragStartTime, setDragStartTime] = useState(0);
  const [dragStartPitch, setDragStartPitch] = useState(0);
  const [previewPos, setPreviewPos] = useState<{ x: number; y: number } | null>(null);

  const y = (maxNote - note.pitch) * noteHeight;
  const x = note.startTime * beatWidth;
  const width = note.duration * beatWidth;
  const velocity = note.velocity / 127;

  const handleDragStart = (event: MouseEvent | TouchEvent | PointerEvent) => {
    setIsDragging(true);
    setDragStartTime(note.startTime);
    setDragStartPitch(note.pitch);
    setPreviewPos(null);
    onSelect(note.id);
  };

  const handleDrag = (
    _event: MouseEvent | TouchEvent | PointerEvent,
    info: { offset: { x: number; y: number } }
  ) => {
    // Show live preview of snapped position
    const timeOffset = info.offset.x / beatWidth;
    const pitchOffset = -Math.round(info.offset.y / noteHeight);

    const newStartTime = Math.max(0, dragStartTime + timeOffset);
    const snappedTime = snapToGrid(newStartTime);
    const newPitch = Math.max(0, Math.min(127, dragStartPitch + pitchOffset));

    const previewX = snappedTime * beatWidth;
    const previewY = (maxNote - newPitch) * noteHeight;

    setPreviewPos({ x: previewX, y: previewY });
  };

  const handleDragEnd = (
    _event: MouseEvent | TouchEvent | PointerEvent,
    info: { offset: { x: number; y: number } }
  ) => {
    setIsDragging(false);
    setPreviewPos(null);

    // Calculate final position
    const timeOffset = info.offset.x / beatWidth;
    const pitchOffset = -Math.round(info.offset.y / noteHeight);

    const newStartTime = Math.max(0, dragStartTime + timeOffset);
    const snappedTime = snapToGrid(newStartTime);
    const newPitch = Math.max(0, Math.min(127, dragStartPitch + pitchOffset));

    // Only call onMove if position changed
    if (snappedTime !== note.startTime || newPitch !== note.pitch) {
      onMove(note.id, snappedTime, newPitch);
    }
  };

  const handleResizeStart = (e: React.MouseEvent) => {
    e.stopPropagation();
    setIsResizing(true);

    const startX = e.clientX;
    const startDuration = note.duration;

    const handleMouseMove = (moveEvent: MouseEvent) => {
      const deltaX = moveEvent.clientX - startX;
      const deltaTime = deltaX / beatWidth;
      const newDuration = Math.max(MIN_NOTE_DURATION, startDuration + deltaTime);
      const snappedDuration = snapToGrid(newDuration);

      onResize(note.id, snappedDuration);
    };

    const handleMouseUp = () => {
      setIsResizing(false);
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);
  };

  return (
    <>
      {/* Preview ghost note during drag */}
      {isDragging && previewPos && (
        <div
          className="absolute rounded border-2 border-cyan-400 border-dashed pointer-events-none z-40"
          style={{
            top: previewPos.y + 2,
            left: previewPos.x,
            width: width - 4,
            height: noteHeight - 4,
            backgroundColor: "rgba(6, 182, 212, 0.2)",
          }}
        />
      )}

      <motion.div
        drag={!isResizing}
        dragMomentum={false}
        dragElastic={0}
        onDragStart={handleDragStart}
        onDrag={handleDrag}
        onDragEnd={handleDragEnd}
        whileHover={{ scale: 1.05, zIndex: 10 }}
        className={cn(
          "absolute rounded cursor-move transition-all pointer-events-auto",
          "hover:brightness-110",
          isSelected && "ring-2 ring-white",
          (isDragging || isResizing) && "opacity-50 z-50"
        )}
        style={{
          top: y + 2,
          left: x,
          width: width - 4,
          height: noteHeight - 4,
          backgroundColor: `rgba(6, 182, 212, ${0.5 + velocity * 0.5})`,
        }}
        onClick={(e) => {
          e.stopPropagation();
          onSelect(note.id);
        }}
        onDoubleClick={(e) => {
          e.stopPropagation();
          onDelete(note.id);
        }}
      >
      <div className="h-full flex items-center px-2 relative">
        <span className="text-xs text-white/90 font-mono">{note.velocity}</span>

        {/* Resize handle */}
        <div
          className={cn(
            "absolute right-0 top-0 bottom-0 w-2 cursor-ew-resize hover:bg-white/30 transition-all pointer-events-auto z-10",
            isResizing && "bg-white/40"
          )}
          title="Resize"
          onMouseDown={handleResizeStart}
        />
      </div>
    </motion.div>
    </>
  );
});
