import { useEffect } from "react";

export type ShortcutAction =
  | "play"
  | "stop"
  | "record"
  | "save"
  | "undo"
  | "redo"
  | "delete"
  | "duplicate"
  | "selectAll"
  | "copy"
  | "paste";

export interface KeyboardShortcut {
  key: string;
  ctrl?: boolean;
  shift?: boolean;
  alt?: boolean;
  action: ShortcutAction;
}

const defaultShortcuts: KeyboardShortcut[] = [
  { key: " ", action: "play" }, // Spacebar
  { key: "Escape", action: "stop" },
  { key: "r", action: "record" },
  { key: "s", ctrl: true, action: "save" },
  { key: "z", ctrl: true, action: "undo" },
  { key: "z", ctrl: true, shift: true, action: "redo" },
  { key: "Delete", action: "delete" },
  { key: "Backspace", action: "delete" },
  { key: "d", ctrl: true, action: "duplicate" },
  { key: "a", ctrl: true, action: "selectAll" },
  { key: "c", ctrl: true, action: "copy" },
  { key: "v", ctrl: true, action: "paste" },
];

export function useKeyboardShortcuts(
  onAction: (action: ShortcutAction) => void,
  shortcuts: KeyboardShortcut[] = defaultShortcuts
) {
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Don't trigger shortcuts if user is typing in an input
      const target = e.target as HTMLElement;
      if (
        target.tagName === "INPUT" ||
        target.tagName === "TEXTAREA" ||
        target.isContentEditable
      ) {
        return;
      }

      for (const shortcut of shortcuts) {
        const keyMatches = e.key === shortcut.key;
        const ctrlMatches = shortcut.ctrl === undefined || e.ctrlKey === shortcut.ctrl || e.metaKey === shortcut.ctrl;
        const shiftMatches = shortcut.shift === undefined || e.shiftKey === shortcut.shift;
        const altMatches = shortcut.alt === undefined || e.altKey === shortcut.alt;

        if (keyMatches && ctrlMatches && shiftMatches && altMatches) {
          e.preventDefault();
          onAction(shortcut.action);
          break;
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onAction, shortcuts]);
}
