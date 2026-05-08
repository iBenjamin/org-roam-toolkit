/**
 * Shared browser launcher and stealth context setup.
 * One source of truth for UA pool, locale, fingerprint hardening.
 */

import { chromium } from 'playwright-extra';
import type { Browser, BrowserContext, Page } from 'playwright';
// playwright-extra's type for plugin-stealth is loose; treat as any for the .use() call.
import StealthPlugin from 'puppeteer-extra-plugin-stealth';

chromium.use(StealthPlugin());

const USER_AGENTS = [
  'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
  'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Safari/605.1.15',
  'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
  'Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0',
];

const pickUserAgent = (): string =>
  USER_AGENTS[Math.floor(Math.random() * USER_AGENTS.length)]!;

/** Resolve after `ms` milliseconds. */
export const sleep = (ms: number): Promise<void> =>
  new Promise((resolve) => setTimeout(resolve, ms));

export const randomDelay = (min = 500, max = 2000): Promise<void> =>
  sleep(Math.random() * (max - min) + min);

export async function launchStealthBrowser(): Promise<Browser> {
  return chromium.launch({
    headless: true,
    args: [
      '--disable-blink-features=AutomationControlled',
      '--no-sandbox',
      '--disable-setuid-sandbox',
    ],
  });
}

export async function newStealthContext(browser: Browser): Promise<BrowserContext> {
  const context = await browser.newContext({
    userAgent: pickUserAgent(),
    viewport: { width: 1920, height: 1080 },
    locale: 'zh-CN',
    timezoneId: 'Asia/Shanghai',
  });
  return context;
}

/** Apply per-page stealth init scripts and headers. */
export async function preparePage(page: Page): Promise<void> {
  await page.addInitScript(() => {
    Object.defineProperty(navigator, 'webdriver', { get: () => undefined });
    Object.defineProperty(navigator, 'plugins', { get: () => [1, 2, 3, 4, 5] });
    Object.defineProperty(navigator, 'languages', {
      get: () => ['zh-CN', 'zh', 'en'],
    });
    // @ts-expect-error - injected at runtime in the page context
    window.chrome = { runtime: {} };
  });
  await page.setExtraHTTPHeaders({
    'Accept-Language': 'zh-CN,zh;q=0.9,en;q=0.8',
  });
}

/** Scroll the page until no further height change (useful for lazy-load). */
export async function scrollToBottom(
  page: Page,
  stepPx = 800,
  delayMs = 300,
): Promise<void> {
  let prevHeight = 0;
  let currHeight = await page.evaluate(() => document.body.scrollHeight);
  while (prevHeight < currHeight) {
    await page.evaluate((step: number) => window.scrollBy(0, step), stepPx);
    await sleep(delayMs);
    prevHeight = currHeight;
    currHeight = await page.evaluate(() => document.body.scrollHeight);
  }
}

/** Light human-like interaction (mouse + small scroll) to defeat naive bot detection. */
export async function simulateHumanActivity(page: Page): Promise<void> {
  await randomDelay(500, 1500);
  await page.mouse.move(
    Math.random() * 500 + 100,
    Math.random() * 300 + 100,
  );
  await page.evaluate(() => window.scrollBy(0, Math.random() * 300 + 100));
  await randomDelay(300, 800);
}
