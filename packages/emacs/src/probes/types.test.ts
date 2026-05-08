import { describe, expect, it } from 'vitest';
import { runProbe } from './types.js';

describe('runProbe envelope', () => {
  it('wraps successful return value in {status: "up", data}', async () => {
    const r = await runProbe(() => 42);
    expect(r.status).toBe('up');
    if (r.status === 'up') {
      expect(r.data).toBe(42);
      expect(r.probedAt).toMatch(/^\d{4}-\d{2}-\d{2}T/);
    }
  });

  it('awaits async functions', async () => {
    const r = await runProbe(async () => ({ k: 'v' }));
    expect(r.status).toBe('up');
    if (r.status === 'up') expect(r.data).toEqual({ k: 'v' });
  });

  it('captures thrown errors as {status: "down", error}', async () => {
    const r = await runProbe(() => {
      throw new Error('boom');
    });
    expect(r.status).toBe('down');
    if (r.status === 'down') expect(r.error).toBe('boom');
  });

  it('captures thrown non-Error values', async () => {
    const r = await runProbe(() => {
      throw 'not-an-error-instance';
    });
    expect(r.status).toBe('down');
    if (r.status === 'down') expect(r.error).toBe('not-an-error-instance');
  });

  it('captures rejected promises', async () => {
    const r = await runProbe(() => Promise.reject(new Error('async-boom')));
    expect(r.status).toBe('down');
    if (r.status === 'down') expect(r.error).toBe('async-boom');
  });
});
