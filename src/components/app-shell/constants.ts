import {
  HouseIcon as House,
  BookIcon as Book,
  ChartBarIcon as ChartBar,
  BookmarkSimpleIcon as BookmarkSimple,
  NotepadIcon as Notepad,
  GearSixIcon as GearSix,
} from "@phosphor-icons/react";
import { EASE } from "@/lib/constants";
import type { NavItem } from "@/types";

export { EASE };
export const DUR = "500ms";
export const spring = (extra = "") =>
  `${extra ? extra + " " : ""}${DUR} ${EASE}`.trim();

export const navigationItems: NavItem[] = [
  { icon: House, label: "Home", path: "/" },
  { icon: Book, label: "Library", path: "/library" },
  { icon: ChartBar, label: "Progress", path: "/progress" },
  { icon: BookmarkSimple, label: "Bookmarks", path: "/bookmarks" },
  { icon: Notepad, label: "Notes", path: "/notes" },
];

export const appItems: NavItem[] = [
  { icon: GearSix, label: "Settings", path: "/settings" },
];

export const routeTitles: Record<string, string> = {
  "/": "Home",
  "/library": "Library",
  "/bookmarks": "Bookmarks",
  "/progress": "Progress",
  "/notes": "Notes",
  "/settings": "Settings",
  "/import": "Import Course",
};
