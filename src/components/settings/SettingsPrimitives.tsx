import { cn } from "@/lib/utils";
import { EASE_OUT } from "@/lib/constants";

// Shared building blocks for the Settings page. Extracted so sections can live
// in their own files without importing from the page itself.

export interface SectionCardProps {
  title: string;
  icon: React.ReactNode;
  children: React.ReactNode;
  index: number;
}

export function SectionCard({ title, icon, children, index }: SectionCardProps) {
  return (
    <div
      className="relative"
      style={{
        animation: `card-in 350ms ${EASE_OUT} ${index * 60}ms both`,
      }}
    >
      <div className="rounded-xl absolute inset-0 bg-border/50" />
      <div className="rounded-xl absolute inset-px bg-card" />
      <div className="relative p-5">
        <div className="mb-4 flex items-center gap-2">
          {icon}
          <h3 className="font-heading text-sm font-bold text-foreground">{title}</h3>
        </div>
        <div className="flex flex-col gap-0.5">{children}</div>
      </div>
    </div>
  );
}

export interface SettingRowProps {
  icon: React.ReactNode;
  label: string;
  description?: string;
  children: React.ReactNode;
}

export function SettingRow({ icon, label, description, children }: SettingRowProps) {
  return (
    <div className="flex items-center justify-between gap-4 rounded-lg px-2 py-3">
      <div className="flex items-center gap-3">
        <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-secondary text-muted-foreground">
          {icon}
        </div>
        <div>
          <div className="font-sans text-sm font-medium text-foreground">{label}</div>
          {description && (
            <div className="font-sans text-xs text-muted-foreground">{description}</div>
          )}
        </div>
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

export function CredInput({
  label,
  value,
  onChange,
  type = "text",
  placeholder,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  type?: string;
  placeholder?: string;
}) {
  return (
    <label className="block px-2">
      <span className="mb-1 block font-sans text-xs font-medium text-muted-foreground">
        {label}
      </span>
      <input
        type={type}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        spellCheck={false}
        autoCapitalize="off"
        autoCorrect="off"
        className={cn(
          "w-full rounded-lg border border-border bg-secondary px-3 py-2",
          "font-mono text-xs text-foreground placeholder:text-muted-foreground/40",
          "outline-none transition-colors focus:border-primary",
        )}
      />
    </label>
  );
}
