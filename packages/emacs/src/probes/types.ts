/** Discriminated union for any health/state probe result. */
export type Probe<T> =
  | { status: 'up'; data: T; probedAt: string }
  | { status: 'down'; error: string; probedAt: string };

/** Wrap a (sync or async) probe function so it never throws. */
export async function runProbe<T>(
  fn: () => T | Promise<T>,
): Promise<Probe<T>> {
  const probedAt = new Date().toISOString();
  try {
    const data = await fn();
    return { status: 'up', data, probedAt };
  } catch (e) {
    const error = e instanceof Error ? e.message : String(e);
    return { status: 'down', error, probedAt };
  }
}
