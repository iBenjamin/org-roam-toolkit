import { describe, expect, it } from 'vitest';
import { TtlCache } from './cache.js';

describe('TtlCache', () => {
  it('caches the producer result for the TTL window', async () => {
    const c = new TtlCache<number>(1_000_000);
    let calls = 0;
    const v1 = await c.get('k', async () => ++calls);
    const v2 = await c.get('k', async () => ++calls);
    expect(v1).toBe(1);
    expect(v2).toBe(1);
    expect(calls).toBe(1);
  });

  it('refreshes after TTL expiry', async () => {
    const c = new TtlCache<number>(0);
    let calls = 0;
    await c.get('k', async () => ++calls);
    await new Promise((r) => setTimeout(r, 5));
    await c.get('k', async () => ++calls);
    expect(calls).toBe(2);
  });

  it('coalesces concurrent requests for the same key', async () => {
    const c = new TtlCache<number>(1_000_000);
    let calls = 0;
    const slow = () => new Promise<number>((res) => setTimeout(() => res(++calls), 20));
    const [a, b, d] = await Promise.all([
      c.get('k', slow),
      c.get('k', slow),
      c.get('k', slow),
    ]);
    expect(a).toBe(1);
    expect(b).toBe(1);
    expect(d).toBe(1);
    expect(calls).toBe(1);
  });

  it('does not cache producer rejection (next call retries)', async () => {
    const c = new TtlCache<number>(1_000_000);
    let calls = 0;
    await expect(
      c.get('k', async () => {
        calls++;
        throw new Error('boom');
      }),
    ).rejects.toThrow('boom');
    const v = await c.get('k', async () => {
      calls++;
      return 42;
    });
    expect(v).toBe(42);
    expect(calls).toBe(2);
  });

  it('invalidate clears one or all keys', async () => {
    const c = new TtlCache<number>(1_000_000);
    let a = 0;
    let b = 0;
    await c.get('a', async () => ++a);
    await c.get('b', async () => ++b);
    c.invalidate('a');
    await c.get('a', async () => ++a);
    await c.get('b', async () => ++b);
    expect(a).toBe(2);
    expect(b).toBe(1);
    c.invalidate();
    await c.get('a', async () => ++a);
    await c.get('b', async () => ++b);
    expect(a).toBe(3);
    expect(b).toBe(2);
  });
});
