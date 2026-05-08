/**
 * Tiny per-key TTL cache. Wraps an async producer; concurrent calls
 * for the same key share the same in-flight promise (no thundering herd).
 */

interface Entry<T> {
  value: T;
  expiresAt: number;
}

interface InflightEntry<T> {
  promise: Promise<T>;
}

export class TtlCache<T> {
  private readonly cache = new Map<string, Entry<T>>();
  private readonly inflight = new Map<string, InflightEntry<T>>();
  private readonly ttlMs: number;

  constructor(ttlMs: number) {
    this.ttlMs = ttlMs;
  }

  async get(key: string, producer: () => Promise<T>): Promise<T> {
    const now = Date.now();
    const entry = this.cache.get(key);
    if (entry && entry.expiresAt > now) {
      return entry.value;
    }
    const inflight = this.inflight.get(key);
    if (inflight) return inflight.promise;

    const promise = (async () => {
      try {
        const value = await producer();
        this.cache.set(key, { value, expiresAt: Date.now() + this.ttlMs });
        return value;
      } finally {
        this.inflight.delete(key);
      }
    })();
    this.inflight.set(key, { promise });
    return promise;
  }

  invalidate(key?: string): void {
    if (key === undefined) this.cache.clear();
    else this.cache.delete(key);
  }
}
