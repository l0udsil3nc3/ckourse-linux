---
name: ui-conventions
description: UI patterns, styling, and component conventions for the Ckourse app. Use when creating pages, components, or UI elements.
---

# Ckourse UI Guidelines

Instructions for creating any new page, element, or component in the Ckourse app.

---

## Tech Stack

- **Framework:** React 19 + TypeScript (strict mode)
- **Styling:** Tailwind CSS v4 (utility-first, no custom CSS files per component)
- **UI primitives:** shadcn/ui (Radix UI + CVA)
- **Icons:** @phosphor-icons/react (6 weights: thin, light, regular, bold, fill, duotone)
- **Utilities:** `cn()` from `@/lib/utils` (clsx + tailwind-merge)
- **Runtime:** Tauri v2 desktop app
- **Build:** Vite 7

---

## Project Structure

```
src/
  App.tsx                  # Root layout and page composition
  main.tsx                 # Entry point
  index.css                # Centralized theme (single source of truth)
  lib/
    utils.ts               # cn() helper
  components/
    ui/                    # shadcn/ui base primitives (Card, Badge, Button, etc.)
    Header.tsx             # App-level components
    Sidebar.tsx            # App sidebar with navigation
    CourseCard.tsx          # Feature components
    ProgressBar.tsx         # Reusable custom components
    SearchInput.tsx          # Reusable search input (rounded-lg, focus border, clear button)
    ActionButton.tsx         # Reusable button (primary/secondary/ghost)
  pages/
    Dashboard.tsx          # Library / Home page
  data/
    courses.ts             # Data types and mock data
  assets/                  # Static assets (images, logos)
```

### Where to put new files

| Type | Location | Example |
|---|---|---|
| shadcn/ui primitive | `src/components/ui/` | `button.tsx`, `dialog.tsx` |
| Feature component | `src/components/` | `LessonCard.tsx`, `Sidebar.tsx` |
| Page component | `src/pages/` | `Dashboard.tsx`, `Settings.tsx` |
| Data types / mock data | `src/data/` | `lessons.ts`, `users.ts` |
| Utility functions | `src/lib/` | `format.ts`, `api.ts` |
| Static assets | `src/assets/` | `logo-dark.png` |

---

## Theme & Colors

All design tokens live in `src/index.css`. Never hardcode color values in components.

### Color Tokens

| Token | Value | Usage |
|---|---|---|
| `--background` | `#0A0A0A` | Page background |
| `--foreground` | `#EDEDED` | Primary text |
| `--card` | `#111214` | Card surfaces |
| `--primary` | `#C8F135` | Electric Lime accent, CTAs |
| `--secondary` | `#1A1C1F` | Hover states, secondary surfaces |
| `--muted` | `#111214` | Subdued backgrounds |
| `--muted-foreground` | `#7A7D85` | Secondary text, labels |
| `--border` | `#252729` | Borders, dividers |
| `--destructive` | `#FF6B6B` | Errors, danger actions |
| `--info` | `#5B9CF6` | Informational states |
| `--success` | `#C8F135` | Success states (same as primary) |

### Using colors in Tailwind

```tsx
// Correct - use theme tokens
<div className="bg-background text-foreground" />
<p className="text-muted-foreground" />
<button className="bg-primary text-primary-foreground" />

// Wrong - never hardcode hex values
<div style={{ color: "#EDEDED" }} />
```

### Opacity modifiers

Use Tailwind's opacity syntax for transparent variants:

```tsx
<Badge className="bg-primary/15 text-primary border-primary/20" />
<div className="bg-info/15 text-info" />
```

---

## Typography

Three font families are defined in the theme:

| Font | Variable | Tailwind Class | Usage |
|---|---|---|---|
| DM Sans | `--font-sans` | `font-sans` | Body text (default) |
| Syne | `--font-heading` | `font-heading` | Headings, titles |
| JetBrains Mono | `--font-mono` | `font-mono` | Numbers, stats, code |

### Typography patterns

```tsx
// Page heading
<h2 className="font-heading text-2xl font-bold text-foreground">Title</h2>

// Body text
<p className="font-sans text-sm text-muted-foreground">Description</p>

// Stats / numeric data
<span className="font-mono text-xs font-medium text-muted-foreground">42/80</span>

// Bold brand text
<span className="text-primary">CK</span>OURSE
```

---

## Component Patterns

### Creating a new component

1. Use named exports (not default exports)
2. Accept a `className` prop and merge it with `cn()`
3. Type props with an interface
4. Use theme tokens, not hardcoded values

```tsx
// src/components/MyComponent.tsx
import { cn } from "@/lib/utils";

interface MyComponentProps {
  title: string;
  className?: string;
}

export function MyComponent({ title, className }: MyComponentProps) {
  return (
    <div className={cn("rounded-xl border border-border bg-card p-4", className)}>
      <h3 className="font-sans text-sm font-semibold text-foreground">
        {title}
      </h3>
    </div>
  );
}
```

### Reusable input & button components

Two reusable components provide the app's standard search input and action button:

**`SearchInput`** — bordered search input with focus border and clear button:

```tsx
import { SearchInput } from "@/components/ui/SearchInput";

<SearchInput
  value={search}
  onChange={setSearch}
  placeholder="Search courses..."
  className="flex-1"
/>
```

Props: `value`, `onChange`, `placeholder?`, `className?`

**`ActionButton`** — bordered button with three variants:

```tsx
import { ActionButton } from "@/components/ui/ActionButton";

// Primary CTA (accent background)
<ActionButton variant="primary" onClick={handleImport}>
  <Plus className="size-4" weight="bold" />
  Import Course
</ActionButton>

// Secondary with active/toggle state (bordered, switches to accent when active)
<ActionButton variant="secondary" active={showFilters} onClick={toggle}>
  <Funnel className="size-4" />
  Filters
</ActionButton>

// Ghost (no border, hover background)
<ActionButton variant="ghost" onClick={handleClick}>
  Label
</ActionButton>
```

Props: `variant?` (`"primary"` | `"secondary"` | `"ghost"`), `active?`, `className?`, plus all native button attributes.

### Using shadcn/ui components

shadcn/ui primitives live in `src/components/ui/` and use CVA for variants:

```tsx
import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";

<Card className="overflow-hidden border-border">
  <CardContent className="p-4">
    <Badge variant="secondary">Label</Badge>
  </CardContent>
</Card>
```

To add a new shadcn component, run:

```bash
npx shadcn@latest add <component-name>
```

### Composing with `cn()`

Always use `cn()` to merge classes. This ensures Tailwind classes resolve correctly:

```tsx
import { cn } from "@/lib/utils";

<div className={cn(
  "base-classes here",
  isActive && "bg-primary text-primary-foreground",
  className
)} />
```

---

## Spacing & Layout

### Standard patterns

| Pattern | Classes |
|---|---|
| Page container | `mx-auto max-w-6xl px-6 py-8` |
| Section header | `mb-6 flex items-baseline justify-between` |
| Card grid | `grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3` |
| Flex row with gap | `flex items-center gap-3` |
| Flex column with gap | `flex flex-col gap-3` |

### Border radius

Ckourse uses standard `border-radius` shapes only — the squircle (clip-path) system was removed. Match shapes to the element's role:

```tsx
<div className="rounded-xl" />  // Cards, panels, dialogs, banners
<div className="rounded-lg" />  // Buttons, search inputs, nav/tooltips, icon containers
<div className="rounded-md" />  // Small buttons, text inputs, chips
<div className="rounded-full" /> // Badges, pills, avatars, progress bars
```

| Element | Class |
|---|---|
| Cards / panels (stat cards, course cards, section panels) | `rounded-xl` |
| Buttons and search inputs | `rounded-lg` |
| Nav button backgrounds, tooltips, icon containers | `rounded-lg` |
| Badges / pills / filter chips | `rounded-full` |
| Page containers / modals | `rounded-xl` |

Borders are plain Tailwind borders — no layered divs are needed:

```tsx
<div className="rounded-xl border border-border bg-card p-4">
<button className="rounded-lg border border-border bg-card px-4 py-2.5">
```

For hover/focus border changes, use `transition-colors` with `focus-within:border-primary` or `group-hover:` variants directly on the element.

---

## Interactive States

Use `transition-colors` for hover/focus animations:

```tsx
// Button / clickable element
<button className="rounded-md p-2 text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground">

// Card hover
<Card className="transition-colors hover:bg-secondary">
```

---

## Loading States

Use Lottie animations for loading indicators. The library `lottie-react` is installed. Lottie JSON files live in `src/assets/lotties/`.

### Usage pattern

```tsx
import Lottie from "lottie-react";
import loadingAnimation from "@/assets/lotties/loading.json";

// Inline loading (inside a card or section)
<Lottie animationData={loadingAnimation} loop className="size-28" />

// Full-screen loading overlay (for blocking operations like imports)
<div className="fixed inset-0 z-50 flex flex-col items-center justify-center bg-background/80 backdrop-blur-sm">
  <Lottie animationData={loadingAnimation} loop className="size-40" />
  <p className="mt-2 font-sans text-sm font-semibold text-foreground">
    Importing course...
  </p>
  <p className="mt-1.5 font-sans text-xs text-muted-foreground">
    Setting up your library
  </p>
</div>
```

### Guidelines

- **Never use CSS spinners** — always use Lottie for loading states
- Use `size-28` for inline/contextual loading, `size-40` for full-screen overlays
- Full-screen overlays use `bg-background/80 backdrop-blur-sm` for the backdrop
- Add a short label (`font-sans text-sm font-semibold`) and optional sublabel (`text-xs text-muted-foreground`) below the animation
- **Tauri commands that do heavy work must be `async fn`** in Rust so they run off the main thread — otherwise the webview freezes and animations won't play

---

## Icons

Use `@phosphor-icons/react`. Always import with the `Icon` suffix (non-deprecated API), aliased to a clean name. Standard size is `size-4`, default weight is `"regular"`:

```tsx
import { GearSixIcon as GearSix, UserIcon as User } from "@phosphor-icons/react";

<GearSix className="size-4" />
<User className="size-4" weight="bold" />
```

Available weights: `"thin"`, `"light"`, `"regular"` (default), `"bold"`, `"fill"`, `"duotone"`. Use `"bold"` for active/emphasized states and `"regular"` for default.

---

## Data Layer

Define types and data in `src/data/`:

```tsx
// src/data/lessons.ts
export interface Lesson {
  id: number;
  title: string;
  duration: number;
}

export const lessons: Lesson[] = [
  { id: 1, title: "Introduction", duration: 12 },
];
```

---

## Import Aliases

Use the `@/` alias for all project imports (configured in `tsconfig.json`):

```tsx
// Correct
import { cn } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";

// Wrong
import { cn } from "../../lib/utils";
```

---

## Checklist for New UI Work

- [ ] Uses theme tokens from `src/index.css` (no hardcoded colors)
- [ ] Uses `cn()` for class merging
- [ ] Uses `@/` import alias
- [ ] Named export (not default)
- [ ] Props typed with an interface
- [ ] `className` prop supported and merged
- [ ] Correct font family (`font-sans`, `font-heading`, or `font-mono`)
- [ ] Interactive elements have `transition-colors` and hover states
- [ ] Correct shape: `rounded-lg` for interactive surfaces, `rounded-xl` for cards, `rounded-full` for badges
- [ ] Icons from `@phosphor-icons/react` with `size-4` default
- [ ] Placed in the correct directory per the structure table
