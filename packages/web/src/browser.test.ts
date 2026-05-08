import type { Page } from 'playwright';
import { describe, expect, it } from 'vitest';
import { scrollToBottom } from './browser.js';

describe('scrollToBottom', () => {
  it('stops after the configured maximum number of scrolls', async () => {
    let scrolls = 0;
    const page = {
      evaluate: async (_fn: unknown, arg?: unknown) => {
        if (typeof arg === 'number') {
          scrolls += 1;
          return undefined;
        }
        return 1000 + scrolls * 1000;
      },
    } as unknown as Page;

    await scrollToBottom(page, 800, 0, 3);

    expect(scrolls).toBe(3);
  });
});
