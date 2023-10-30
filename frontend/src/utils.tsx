// vi: shiftwidth=2 softtabstop=2

export function animationFrame(): Promise<void> {
  return new Promise(res => {
    requestAnimationFrame(() => res());
  })
}

export function time(ms: number): Promise<void> {
  return new Promise(res => setTimeout(res, ms));
}
