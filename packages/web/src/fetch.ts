/**
 * Site-aware fetcher. Picks a SiteHandler based on the URL, runs it
 * inside a stealth-configured browser context, and returns a structured result.
 */

import type { FetchResult } from './types.js';
import {
  launchStealthBrowser,
  newStealthContext,
  preparePage,
  randomDelay,
  sleep,
} from './browser.js';
import { pickHandler } from './sites/index.js';

const DEFAULT_NAV: { waitUntil: 'networkidle'; timeout: number } = {
  waitUntil: 'networkidle',
  timeout: 30_000,
};

export async function fetchUrl(rawUrl: string): Promise<FetchResult> {
  const url = new URL(rawUrl);
  const handler = pickHandler(url);

  const browser = await launchStealthBrowser();
  try {
    const ctx = await newStealthContext(browser);
    const page = await ctx.newPage();
    await preparePage(page);
    await randomDelay(300, 800);

    await page.goto(rawUrl, handler.navOptions ?? DEFAULT_NAV);
    if (handler.postNavWait) {
      await sleep(handler.postNavWait);
    }

    return await handler.extract(page, url);
  } finally {
    await browser.close();
  }
}
