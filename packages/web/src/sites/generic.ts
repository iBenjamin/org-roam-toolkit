import type { Page } from 'playwright';
import type { FetchResult, SiteHandler } from '../types.js';
import { simulateHumanActivity } from '../browser.js';

/**
 * Optional CSS-selector rules. If a rule is registered for a hostname,
 * `genericExtract` uses it; otherwise it falls back to document title + body innerText.
 */
export interface ExtractionRule {
  title: string;
  author: string;
  content: string;
}

const HOST_RULES: Record<string, ExtractionRule> = {
  'zhuanlan.zhihu.com': {
    title: '.Post-Title',
    author: '.AuthorInfo-name',
    content: '.Post-RichTextContainer',
  },
  'www.zhihu.com': {
    title: '.QuestionHeader-title',
    author: '.AuthorInfo-name',
    content: '.RichContent-inner',
  },
};

/** Try CSS-selector extraction by hostname; fall back to whole-page text. */
export async function genericExtract(
  page: Page,
  url: URL,
): Promise<FetchResult> {
  await simulateHumanActivity(page);
  const rule = HOST_RULES[url.hostname] ?? null;

  const result = await page.evaluate((r: ExtractionRule | null) => {
    if (r) {
      const t = document.querySelector(r.title);
      const c = document.querySelector(r.content);
      if (t && c) {
        const a = document.querySelector(r.author);
        return {
          title: (t.textContent ?? '').trim(),
          author: (a?.textContent ?? '').trim(),
          content: (c as HTMLElement).innerText,
        };
      }
    }
    return {
      title: document.title || '',
      author: '',
      content: document.body?.innerText || '',
    };
  }, rule);

  return { ...result, url: url.toString() };
}

/** Catch-all handler. Should be the last entry in the registry. */
export const genericHandler: SiteHandler = {
  name: 'generic',
  match: () => true,
  extract: genericExtract,
};
