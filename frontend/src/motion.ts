export const appleSpring = {
  type: "spring" as const,
  stiffness: 420,
  damping: 34,
  mass: 1,
};

export const appleSpringSoft = {
  type: "spring" as const,
  stiffness: 320,
  damping: 30,
  mass: 1,
};

export const appleSpringBouncy = {
  type: "spring" as const,
  stiffness: 420,
  damping: 24,
  mass: 1,
};

// Standard non-spring easing for opacity/color fades where a spring would overshoot.
export const appleEase = {
  duration: 0.22,
  ease: [0.32, 0.72, 0, 1] as [number, number, number, number],
};

