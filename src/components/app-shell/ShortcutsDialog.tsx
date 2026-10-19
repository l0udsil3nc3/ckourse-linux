import { useState, useEffect, useCallback } from "react";
import { KeyboardIcon as Keyboard, XIcon as X } from "@phosphor-icons/react";
import { cn } from "@/lib/utils";

interface ShortcutGroup {
  title: string;
  items: { keys: string[]; action: string }[];
}

const GROUPS: ShortcutGroup[] = [
  {
    title: "General",
    items: [
      { keys: ["/"], action: "Open search" },
      { keys: ["Ctrl", "F"], action: "Toggle search" },
      { keys: ["Ctrl", "Space"], action: "Toggle search" },
      { keys: ["D"], action: "Go to dashboard" },
      { keys: ["Ctrl", "N"], action: "Start a new note" },
      { keys: ["Esc"], action: "Close dialogs" },
    ],
  },
  {
    title: "Dashboard",
    items: [
      { keys: ["type"], action: "Start searching directly" },
      { keys: ["↑", "↓", "Enter"], action: "Select and open a course" },
      { keys: ["Space"], action: "Resume last-played video" },
    ],
  },
  {
    title: "Search",
    items: [
      { keys: ["↑", "↓"], action: "Move through results" },
      { keys: ["Enter"], action: "Open the highlighted result" },
      { keys: ["Esc"], action: "Close search" },
    ],
  },
  {
    title: "Player",
    items: [
      { keys: ["Space", "K"], action: "Play / pause" },
      { keys: ["J", "L"], action: "Back / forward 10 seconds" },
      { keys: ["0", "-", "9"], action: "Jump to 0%–90% of the video" },
      { keys: ["←", "→"], action: "Seek" },
      { keys: ["↑", "↓"], action: "Volume" },
      { keys: ["N", "P"], action: "Next / previous lesson" },
      { keys: ["F"], action: "Fullscreen" },
      { keys: ["M"], action: "Mute" },
      { keys: ["C"], action: "Subtitles" },
    ],
  },
];

export function ShortcutsDialog({ className }: { className?: string }) {
  const [open, setOpen] = useState(false);

  const close = useCallback(() => setOpen(false), []);

  useEffect(() => {
    if (!open) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [open, close ]);

  return (
    <>
      <button
        onClick={() => setOpen(true)}
        title="Keyboard shortcuts"
        className={cn(
          "rounded-md p-2 text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground [&_svg]:size-4",
          className,
        )}
      >
        <Keyboard />
      </button>

      {/* Backdrop */}
      <div
        className="fixed inset-0 z-50 bg-background/70 backdrop-blur-sm"
        style={{
          opacity: open ? 1 : 0,
          pointerEvents: open ? "auto" : "none",
          transition: "opacity 400ms cubic-bezier(0.32, 0.72, 0, 1)",
        }}
        onPointerDown={close}
      />

      {/* Dialog */}
      <div
        className="fixed inset-0 z-50 flex items-start justify-center px-4 pt-[15vh]"
        style={{ pointerEvents: open ? "auto" : "none" }}
        onPointerDown={close}
      >
        <div
          className="w-full max-w-md"
          style={{
            opacity: open ? 1 : 0,
            transform: open ? "scale(1) translateY(0)" : "scale(0.96) translateY(-8px)",
            transition:
              "opacity 350ms cubic-bezier(0.16, 1, 0.3, 1), transform 350ms cubic-bezier(0.16, 1, 0.3, 1)",
          }}
          onPointerDown={(e) => e.stopPropagation()}
        >
          <div className="squircle-subtle absolute inset-0 bg-border/60" />
          <div className="squircle-subtle absolute inset-px bg-card" />

          <div className="relative max-h-[60vh] overflow-y-auto px-5 py-4">
            <div className="mb-3 flex items-center justify-between">
              <h2 className="font-heading text-sm font-bold text-foreground">
                Keyboard shortcuts
              </h2>
              <button
                onClick={close}
                className="rounded-md p-1 text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground"
              >
                <X className="size-4" />
              </button>
            </div>

            {GROUPS.map((group) => (
              <div key={group.title} className="mb-4 last:mb-0">
                <div className="pb-1 pt-1">
                  <span className="font-mono text-[10px] font-medium uppercase tracking-wider text-muted-foreground/60">
                    {group.title}
                  </span>
                </div>
                {group.items.map((item) => (
                  <div
                    key={`${item.action} — ${item.keys.join(" + ")}`}
                    className="flex items-center justify-between gap-3 py-1.5"
                  >
                    <span className="font-sans text-xs text-muted-foreground">
                      {item.action}
                    </span>
                    <span className="flex shrink-0 items-center gap-1">
                      {item.keys.map((k) => (
                        <kbd
                          key={k}
                          className="rounded border border-border bg-secondary px-1.5 py-0.5 font-mono text-[10px] text-foreground"
                        >
                          {k}
                        </kbd>
                      ))}
                    </span>
                  </div>
                ))}
              </div>
            ))}
          </div>
        </div>
      </div>
    </>
  );
}
