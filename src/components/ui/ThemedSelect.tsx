import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { CaretDownIcon as CaretDown, CheckIcon as Check } from "@phosphor-icons/react";
import { cn } from "@/lib/utils";

export interface ThemedSelectOption {
  value: string;
  label: string;
}

interface ThemedSelectProps {
  value: string;
  onChange: (value: string) => void;
  options: ThemedSelectOption[];
  className?: string;
  variant?: "default" | "inline";
}

/**
 * Custom dropdown that renders its menu in a portal on document.body, so it
 * always floats above surrounding cards instead of being clipped or painted
 * over by later siblings (each settings card forms its own stacking context).
 */
export function ThemedSelect({ value, onChange, options, className, variant = "default" }: ThemedSelectProps) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const [menuPos, setMenuPos] = useState<{ top: number; right: number; minWidth: number }>({ top: 0, right: 0, minWidth: 0 });
  const selected = options.find((o) => o.value === value);

  const updateMenuPos = () => {
    const btn = buttonRef.current;
    if (!btn) return;
    const rect = btn.getBoundingClientRect();
    setMenuPos({
      top: rect.bottom + 6,
      right: Math.max(8, window.innerWidth - rect.right),
      minWidth: Math.max(rect.width, 120),
    });
  };

  useLayoutEffect(() => {
    if (open) updateMenuPos();
  }, [open ]);

  useEffect(() => {
    if (!open) return;
    const onPointerDown = (e: PointerEvent) => {
      const target = e.target as Node;
      // Clicks inside the button toggle; clicks inside the portal menu select.
      // Everything else closes.
      if (rootRef.current?.contains(target)) return;
      if (menuRef.current?.contains(target)) return;
      setOpen(false);
    };
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    const onReposition = () => updateMenuPos();
    window.addEventListener("pointerdown", onPointerDown);
    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("scroll", onReposition, true);
    window.addEventListener("resize", onReposition);
    return () => {
      window.removeEventListener("pointerdown", onPointerDown);
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("scroll", onReposition, true);
      window.removeEventListener("resize", onReposition);
    };
  }, [open ]);

  return (
    <div ref={rootRef} className={cn("relative", className)}>
      <button
        ref={buttonRef}
        type="button"
        aria-haspopup="listbox"
        aria-expanded={open}
        onClick={() => setOpen((o) => !o)}
        className={
          variant === "inline"
            ? cn(
                "flex cursor-pointer items-center gap-1 bg-transparent font-sans text-xs font-medium text-muted-foreground outline-none transition-colors hover:text-foreground",
              )
            : cn(
                "flex cursor-pointer items-center gap-2 rounded-lg border border-border bg-secondary px-3 py-1.5",
                "font-sans text-sm text-foreground outline-none transition-colors hover:border-muted-foreground/30",
              )
        }
      >
        <span className="truncate">{selected?.label ?? value}</span>
        <CaretDown
          className={cn("size-3.5 shrink-0 text-muted-foreground transition-transform duration-150", open && "rotate-180")}
        />
      </button>

      {open &&
        createPortal(
          <div
            ref={menuRef}
            role="listbox"
            style={{ top: menuPos.top, right: menuPos.right, minWidth: menuPos.minWidth }}
            className="fixed z-[100] max-h-64 overflow-y-auto rounded-lg border border-border bg-card py-1 shadow-xl"
          >
            {options.map((opt) => {
              const isSelected = opt.value === value;
              return (
                <button
                  key={opt.value}
                  type="button"
                  role="option"
                  aria-selected={isSelected}
                  onClick={() => {
                    onChange(opt.value);
                    setOpen(false);
                  }}
                  className={cn(
                    "flex w-full items-center justify-between gap-3 whitespace-nowrap px-3 py-1.5 text-left font-sans text-sm transition-colors",
                    isSelected
                      ? "bg-primary font-medium text-primary-foreground"
                      : "text-foreground hover:bg-secondary",
                  )}
                >
                  {opt.label}
                  {isSelected && <Check className="size-3.5 shrink-0" weight="bold" />}
                </button>
              );
            })}
          </div>,
          document.body,
        )}
    </div>
  );
}
