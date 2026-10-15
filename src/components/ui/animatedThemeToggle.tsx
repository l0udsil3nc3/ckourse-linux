"use client"

import { useCallback, useRef } from "react"
import { Moon, Sun } from "lucide-react"
import { flushSync } from "react-dom"

import { cn } from "@/lib/utils"
import { useTheme } from "@/hooks/useTheme"

interface AnimatedThemeTogglerProps extends React.ComponentPropsWithoutRef<"button"> {
  duration?: number
}

export const AnimatedThemeToggler = ({
  className,
  duration = 400,
  ...props
}: AnimatedThemeTogglerProps) => {
  const { mode, toggleTheme } = useTheme()
  const buttonRef = useRef<HTMLButtonElement>(null)

  const toggleThemeWithTransition = useCallback(() => {
    if (!buttonRef.current) return

    const applyTheme = () => {
      toggleTheme()
    }

    if (
      typeof document === "undefined" ||
      !("startViewTransition" in document)
    ) {
      applyTheme()
      return
    }

    const transition = document.startViewTransition(() => {
      flushSync(applyTheme)
    })

    const ready = transition?.ready
    if (ready && typeof ready.then === "function") {
      ready.then(() => {
        const button = buttonRef.current
        if (!button) return

        const { top, left, width, height } = button.getBoundingClientRect()

        const x = left + width / 2
        const y = top + height / 2

        const maxRadius = Math.hypot(
          Math.max(left, window.innerWidth - left),
          Math.max(top, window.innerHeight - top)
        )

        document.documentElement.animate(
          {
            clipPath: [
              `circle(0px at ${x}px ${y}px)`,
              `circle(${maxRadius}px at ${x}px ${y}px)`,
            ],
          },
          {
            duration,
            easing: "ease-in-out",
            pseudoElement: "::view-transition-new(root)",
          }
        )
      })
    }
  }, [toggleTheme, duration])

  return (
    <button
      ref={buttonRef}
      onClick={toggleThemeWithTransition}
      className={cn(className)}
      aria-label={`Current: ${mode}. Click to toggle light/dark.`}
      {...props}
    >
      {mode === "dark" ? <Sun /> : <Moon />}
      <span className="sr-only">Toggle theme ({mode})</span>
    </button>
  )
}
