import { useState, useEffect, useMemo, useContext } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { ActivePathContext } from "@/hooks/usePageVisible";
import { useSettings } from "@/hooks/useSettings";
import { cn, stripHtml, timeAgo } from "@/lib/utils";
import {
  PlayIcon as Play,
  NoteIcon as Note,
  BookOpenIcon as BookOpen,
} from "@phosphor-icons/react";
import LottieLib from "lottie-react";
import loadingAnimation from "@/assets/lotties/loading.json";
import { EASE_OUT } from "@/lib/constants";
import { ProgressBar } from "@/components/ui/ProgressBar";
import { ActionButton } from "@/components/ui/ActionButton";
import { DashboardStatsBar } from "@/components/dashboard/DashboardStats";
import { EmptyLibrary } from "@/components/dashboard/EmptyLibrary";
import type { Course, DashboardStats, NoteWithCourse } from "@/types";
import {
  getCourses,
  getDashboardStats,
  getResumeTarget,
  getAllNotes,
  type ResumeTarget,
} from "@/lib/store";

// Handle CJS/ESM default export interop: in some Vite/Rollup build modes
// lottie-react resolves to the module namespace object rather than the component
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const Lottie: React.ComponentType<{
  animationData: unknown;
  loop?: boolean;
  className?: string;
}> = (LottieLib as any).default ?? LottieLib;

interface HomeProps {
  className?: string;
}

function greeting(): string {
  const h = new Date().getHours();
  if (h < 5) return "Good night";
  if (h < 12) return "Good morning";
  if (h < 18) return "Good afternoon";
  return "Good evening";
}

function lessonCountLabel(c: Course): string {
  return `${c.completedLessons}/${c.totalLessons} lessons`;
}

function watchedLabel(c: Course): string {
  return c.lastWatched ? `watched ${timeAgo(c.lastWatched)}` : "not started";
}

export function Home({ className }: HomeProps) {
  const navigate = useNavigate();
  const location = useLocation();
  const activePath = useContext(ActivePathContext);
  const isVisible = activePath === "/";
  const { settings } = useSettings();

  const [stats, setStats] = useState<DashboardStats | null>(null);
  const [courses, setCourses] = useState<Course[]>([]);
  const [notes, setNotes] = useState<NoteWithCourse[]>([]);
  const [resume, setResume] = useState<ResumeTarget | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState(false);

  useEffect(() => {
    if (!isVisible) return;
    let cancelled = false;

    (async () => {
      try {
        const [s, c, n, r] = await Promise.all([
          getDashboardStats(),
          getCourses(),
          getAllNotes(),
          getResumeTarget(),
        ]);
        if (cancelled) return;
        setStats(s);
        setCourses(c);
        setNotes(n);
        setResume(r);
      } catch {
        if (!cancelled) setLoadError(true);
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [isVisible]);

  // Most recently watched course — powers "Continue watching" when no
  // explicit resume target exists yet.
  const recentCourse = useMemo(
    () =>
      [...courses]
        .filter((c) => c.lastWatched)
        .sort(
          (a, b) =>
            new Date(b.lastWatched!).getTime() -
            new Date(a.lastWatched!).getTime(),
        )[0] ?? null,
    [courses],
  );

  const heroCourse = useMemo(() => {
    if (!resume) return recentCourse;
    return courses.find((c) => c.id === resume.courseId) ?? recentCourse;
  }, [resume, recentCourse, courses]);

  const recentCourses = useMemo(
    () =>
      [...courses]
        .filter((c) => c.id !== heroCourse?.id)
        .sort((a, b) => {
          const aTime = a.lastWatched ? new Date(a.lastWatched).getTime() : 0;
          const bTime = b.lastWatched ? new Date(b.lastWatched).getTime() : 0;
          return bTime - aTime;
        })
        .slice(0, 5),
    [courses, heroCourse],
  );

  const recentNotes = useMemo(
    () =>
      [...notes]
        .sort(
          (a, b) =>
            new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime(),
        )
        .slice(0, 6),
    [notes],
  );

  const fromParam = encodeURIComponent(location.pathname + location.search);

  const handleResume = () => {
    if (!heroCourse) return;
    navigate(
      resume && resume.courseId === heroCourse.id
        ? `/course/${heroCourse.id}?lesson=${resume.lessonId}&from=${fromParam}`
        : `/course/${heroCourse.id}?from=${fromParam}`,
    );
  };

  if (!isVisible) return null;

  if (loading) {
    return (
      <div
        className={cn(
          "flex flex-col items-center justify-center gap-3 py-32",
          className,
        )}
      >
        <Lottie animationData={loadingAnimation} loop className="size-28" />
        <p className="font-sans text-sm font-semibold text-foreground">
          Loading your library
        </p>
      </div>
    );
  }

  if (loadError) {
    return (
      <div
        className={cn(
          "mx-auto flex max-w-6xl flex-col items-center justify-center gap-3 py-32",
          className,
        )}
      >
        <p className="font-sans text-sm text-muted-foreground">
          Failed to load your home dashboard.
        </p>
        <button
          onClick={() => window.location.reload()}
          className="font-sans text-xs font-medium text-primary transition-colors hover:text-primary/80"
        >
          Try again
        </button>
      </div>
    );
  }

  if (courses.length === 0) {
    return (
      <div className={cn("mx-auto max-w-6xl", className)}>
        <EmptyLibrary onImport={() => navigate("/import")} />
      </div>
    );
  }

  return (
    <div className={cn("mx-auto max-w-6xl", className)}>
      {/* Greeting */}
      <div
        className="mb-6"
        style={{ animation: `card-in 350ms ${EASE_OUT} both` }}
      >
        <h2 className="font-heading text-2xl font-bold text-foreground">
          {greeting()}
        </h2>
        <p className="mt-1 font-sans text-sm text-muted-foreground">
          Continue learning and make progress today.
        </p>
      </div>

      {stats && settings.show_stats && (
        <DashboardStatsBar stats={stats} className="mb-6" />
      )}

      {/* Continue watching hero */}
      {heroCourse && (
        <div
          className="mb-6 rounded-xl border border-border bg-card p-5"
          style={{ animation: `card-in 350ms ${EASE_OUT} 50ms both` }}
        >
          <div className="mb-4 flex items-center gap-2">
            <Play className="size-4 text-primary" weight="fill" />
            <h3 className="font-heading text-base font-bold text-foreground">
              Continue watching
            </h3>
          </div>

          <div className="flex flex-col gap-4 sm:flex-row sm:items-center">
            {/* Solid accent thumbnail block */}
            <button
              onClick={handleResume}
              className="relative flex aspect-video w-full shrink-0 items-center justify-center overflow-hidden rounded-xl sm:w-64"
              style={{ backgroundColor: `${heroCourse.accentColor}10` }}
            >
              <div
                className="absolute inset-x-0 top-0 h-1.5"
                style={{ backgroundColor: heroCourse.accentColor }}
              />
              <span
                className="font-heading text-xl font-bold opacity-20"
                style={{ color: heroCourse.accentColor }}
              >
                {heroCourse.title.split(/[\s—-]/)[0]}
              </span>
              <span className="absolute inset-0 flex items-center justify-center">
                <span className="flex size-11 items-center justify-center rounded-full bg-background/65 text-foreground backdrop-blur-sm transition-transform duration-150 ease-out group-hover:scale-105">
                  <Play className="size-5" weight="fill" />
                </span>
              </span>
            </button>

            <div className="min-w-0 flex-1">
              <h4 className="font-sans text-base font-semibold text-foreground">
                {heroCourse.title}
              </h4>
              <p className="mt-0.5 font-sans text-xs text-muted-foreground">
                {heroCourse.author || "Unknown author"}
                {heroCourse.category !== "other" &&
                  ` · ${heroCourse.category}`}
              </p>

              <div className="mt-3">
                <div className="mb-1 flex items-center justify-between font-mono text-[11px] text-muted-foreground">
                  <span>{lessonCountLabel(heroCourse)}</span>
                  <span>{watchedLabel(heroCourse)}</span>
                </div>
                <ProgressBar
                  value={
                    heroCourse.totalLessons > 0
                      ? (heroCourse.completedLessons /
                          heroCourse.totalLessons) *
                        100
                      : 0
                  }
                />
              </div>

              <div className="mt-4 flex items-center gap-2">
                <ActionButton variant="primary" onClick={handleResume}>
                  <Play className="size-4" weight="fill" />
                  Resume
                </ActionButton>
                <ActionButton
                  variant="ghost"
                  onClick={() =>
                    navigate(`/course/${heroCourse.id}?from=${fromParam}`)
                  }
                >
                  <BookOpen className="size-4" />
                  Open course
                </ActionButton>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Recent courses + notes */}
      <div className="grid grid-cols-1 items-start gap-6 lg:grid-cols-[1.6fr_1.15fr]">
        <div
          className="rounded-xl border border-border bg-card"
          style={{ animation: `card-in 350ms ${EASE_OUT} 100ms both` }}
        >
          <div className="flex items-center justify-between px-5 pt-4">
            <h3 className="flex items-center gap-2 font-heading text-base font-bold text-foreground">
              <BookOpen className="size-4 text-primary" />
              Recent courses
            </h3>
            <button
              onClick={() => navigate("/library")}
              className="font-sans text-xs font-medium text-primary transition-colors hover:text-primary/80"
            >
              See all
            </button>
          </div>

          <div className="mt-2">
            {recentCourses.map((c, i) => (
              <RecentCourseRow
                key={c.id}
                course={c}
                index={i}
                onOpen={() => navigate(`/course/${c.id}?from=${fromParam}`)}
              />
            ))}
            {recentCourses.length === 0 && (
              <p className="px-5 py-8 text-center font-sans text-sm text-muted-foreground">
                No other courses yet.
              </p>
            )}
          </div>
        </div>

        <div
          className="rounded-xl border border-border bg-card"
          style={{ animation: `card-in 350ms ${EASE_OUT} 150ms both` }}
        >
          <div className="flex items-center justify-between px-5 pt-4">
            <h3 className="flex items-center gap-2 font-heading text-base font-bold text-foreground">
              <Note className="size-4 text-primary" />
              Recent notes
            </h3>
            <button
              onClick={() => navigate("/notes")}
              className="font-sans text-xs font-medium text-primary transition-colors hover:text-primary/80"
            >
              See all
            </button>
          </div>

          <div className="mt-2">
            {recentNotes.map((n, i) => (
              <NoteRow
                key={n.id}
                note={n}
                index={i}
                onOpen={() =>
                  navigate(
                    `/course/${n.courseId}?lesson=${n.lessonId}&from=${fromParam}`,
                  )
                }
              />
            ))}
            {recentNotes.length === 0 && (
              <p className="px-5 py-8 text-center font-sans text-sm text-muted-foreground">
                Notes you add while watching will show up here.
              </p>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

interface RecentCourseRowProps {
  course: Course;
  index: number;
  onOpen: () => void;
}

function RecentCourseRow({ course, index, onOpen }: RecentCourseRowProps) {
  const percentage =
    course.totalLessons > 0
      ? Math.round((course.completedLessons / course.totalLessons) * 100)
      : 0;

  return (
    <button
      onClick={onOpen}
      className="flex w-full items-center gap-3 border-t border-border/60 px-5 py-3 text-left transition-colors duration-150 hover:bg-secondary"
      style={{ animation: `card-in 350ms ${EASE_OUT} ${index * 30}ms both` }}
    >
      {/* Solid accent chip */}
      <div
        className="flex size-10 shrink-0 items-center justify-center rounded-lg"
        style={{ backgroundColor: `${course.accentColor}1A` }}
      >
        <span
          className="font-heading text-sm font-bold"
          style={{ color: course.accentColor }}
        >
          {course.title.split(/[\s—-]/)[0].slice(0, 2).toUpperCase()}
        </span>
      </div>

      <div className="min-w-0 flex-1">
        <div className="flex items-baseline justify-between gap-3">
          <span className="truncate font-sans text-sm font-semibold text-foreground">
            {course.title}
          </span>
          <span className="shrink-0 font-mono text-[11px] text-muted-foreground">
            {percentage}%
          </span>
        </div>
        <div className="mt-0.5 font-sans text-[11px] text-muted-foreground">
          {lessonCountLabel(course)} · {watchedLabel(course)}
        </div>
        <ProgressBar value={percentage} className="mt-1.5 h-1" />
      </div>
    </button>
  );
}

interface NoteRowProps {
  note: NoteWithCourse;
  index: number;
  onOpen: () => void;
}

function NoteRow({ note, index, onOpen }: NoteRowProps) {
  return (
    <button
      onClick={onOpen}
      className="flex w-full items-start gap-2.5 border-t border-border/60 px-5 py-3 text-left transition-colors duration-150 hover:bg-secondary"
      style={{ animation: `card-in 350ms ${EASE_OUT} ${index * 30}ms both` }}
    >
      <Note className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
      <div className="min-w-0 flex-1">
        <p className="line-clamp-2 font-sans text-sm leading-snug text-foreground">
          {stripHtml(note.content)}
        </p>
        <p className="mt-1 font-sans text-[11px] text-muted-foreground">
          <span
            className="font-medium"
            style={{ color: note.accentColor }}
          >
            {note.courseTitle}
          </span>
          {" · "}
          {timeAgo(note.updatedAt)}
        </p>
      </div>
    </button>
  );
}
