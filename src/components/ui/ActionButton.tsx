import { cn } from "@/lib/utils";

type ActionButtonVariant = "primary" | "secondary" | "ghost";

interface ActionButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ActionButtonVariant;
  active?: boolean;
}

const variantStyles: Record<
  ActionButtonVariant,
  { base: string; active: string }
> = {
  primary: {
    base: "border-transparent bg-primary font-semibold text-primary-foreground hover:bg-primary/85",
    active: "border-transparent bg-primary font-semibold text-primary-foreground hover:bg-primary/85",
  },
  secondary: {
    base: "border-border bg-card font-medium text-muted-foreground hover:text-foreground",
    active: "border-primary/30 bg-primary/10 font-medium text-primary",
  },
  ghost: {
    base: "border-transparent bg-transparent font-medium text-muted-foreground hover:bg-secondary hover:text-foreground",
    active: "border-primary/30 bg-primary/10 font-medium text-primary",
  },
};

export function ActionButton({
  variant = "secondary",
  active = false,
  className,
  children,
  ...props
}: ActionButtonProps) {
  const styles = active
    ? variantStyles[variant].active
    : variantStyles[variant].base;

  return (
    <button
      className={cn(
        "flex items-center gap-2 rounded-lg border px-4 py-2.5 font-sans text-sm",
        "transition-[background-color,border-color,color,transform] duration-150 ease-out active:scale-[0.97]",
        styles,
        className
      )}
      {...props}
    >
      {children}
    </button>
  );
}
