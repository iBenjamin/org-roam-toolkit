import type { FetchResult, SiteHandler } from '../types.js';
import { scrollToBottom } from '../browser.js';

/**
 * Threshold below which a wechat post with images is considered "image-only".
 * 100 chars is empirically chosen: shorter than typical promo blurb, longer
 * than a single decorative caption.
 */
const IMAGE_ARTICLE_TEXT_THRESHOLD = 100;

/**
 * mp.weixin.qq.com — handles both text articles and image-only posts.
 *
 * Specifics:
 *   - Lazy-loaded images require scrollToBottom() before extraction.
 *   - Image articles use .img_swiper_area; text articles use #js_content.
 *   - Output adds `images[]` and `isImageArticle` fields.
 */
export const wechatHandler: SiteHandler = {
  name: 'wechat',

  match: (url) => url.hostname === 'mp.weixin.qq.com',

  navOptions: { waitUntil: 'networkidle', timeout: 30_000 },

  async extract(page, url): Promise<FetchResult> {
    await scrollToBottom(page);

    const data = await page.evaluate(() => {
      const titleEl = document.querySelector('#activity-name');
      const authorEl = document.querySelector('#js_name');
      const contentEl = document.querySelector('#js_content');
      const swiperArea = document.querySelector('.img_swiper_area');

      const title = (titleEl?.textContent ?? document.title ?? '').trim();
      const author = (authorEl?.textContent ?? '').trim();
      const content = (contentEl as HTMLElement | null)?.innerText ?? '';

      const seen = new Set<string>();
      const images: string[] = [];
      const add = (u: string | null | undefined): void => {
        if (!u || !u.startsWith('http') || seen.has(u)) return;
        seen.add(u);
        images.push(u);
      };

      // Image-message format: .img_swiper_area uses [data-src] on divs.
      if (swiperArea) {
        for (const el of swiperArea.querySelectorAll('[data-src]')) {
          if (el.tagName === 'SCRIPT') continue;
          add(el.getAttribute('data-src'));
        }
        // Fallback: empty data-src → use img[src] inside .swiper_item_img
        if (images.length === 0) {
          for (const img of swiperArea.querySelectorAll(
            '.swiper_item_img img',
          )) {
            add(img.getAttribute('src'));
          }
        }
      }

      // Text-message format: images live inside #js_content.
      const articleContainer = (contentEl as HTMLElement | null) ?? document.body;

      for (const img of articleContainer.querySelectorAll('img')) {
        add(img.getAttribute('data-src') ?? img.getAttribute('src') ?? '');
      }

      // SVG <image> tags (used for some embedded illustrations)
      for (const img of articleContainer.querySelectorAll('image')) {
        add(img.getAttribute('xlink:href') ?? img.getAttribute('href') ?? '');
      }

      // CSS background-image
      for (const el of articleContainer.querySelectorAll(
        '[style*="background"]',
      )) {
        const style = (el as HTMLElement).style.backgroundImage;
        const match = style?.match(/url\(["']?(https?:\/\/[^"')]+)["']?\)/);
        if (match) add(match[1]);
      }

      return { title, author, content, images, hasSwiper: !!swiperArea };
    });

    const isImageArticle =
      data.hasSwiper ||
      (data.images.length > 0 &&
        data.content.length < IMAGE_ARTICLE_TEXT_THRESHOLD);

    return {
      title: data.title,
      author: data.author,
      content: data.content,
      images: data.images,
      isImageArticle,
      url: url.toString(),
    };
  },
};
