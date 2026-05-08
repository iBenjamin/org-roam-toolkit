import type { Page } from 'playwright';

/** Standard fetch result. Site handlers may add fields (e.g., images for wechat). */
export interface FetchResult {
  title: string;
  author: string;
  content: string;
  url: string;
  /** Optional: extracted image URLs (currently set by wechat handler). */
  images?: string[];
  /** Optional: true when the page is mostly images with little text. */
  isImageArticle?: boolean;
}

export interface NavOptions {
  waitUntil?: 'networkidle' | 'domcontentloaded' | 'load';
  /** Timeout in milliseconds. */
  timeout?: number;
}

export interface SiteHandler {
  /** Stable identifier for logging / debugging. */
  name: string;
  /** Returns true if this handler should be used for the given URL. */
  match(url: URL): boolean;
  /** Optional override of page.goto options. */
  navOptions?: NavOptions;
  /** Optional milliseconds to wait after navigation completes. */
  postNavWait?: number;
  /**
   * Extract a result from the loaded page. Handlers may scroll, click,
   * etc. before reading.
   */
  extract(page: Page, url: URL): Promise<FetchResult>;
}
