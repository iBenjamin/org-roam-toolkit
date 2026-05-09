---
name: fetch
description: |
  Use when WebFetch fails, returns empty content, or a page requires JavaScript rendering;
  also when fetching WeChat posts, archive.* pages, or OCR-ing fetched images.

  Triggers: WebFetch failure, empty content, JavaScript rendering, WeChat post,
  mp.weixin.qq.com, archive.today, archive.ph, OCR
---

# Fetch Skill

Fetch web pages that need JavaScript rendering and automatically choose the extraction strategy by hostname.

## When To Use

Use this when WebFetch:
- Returns empty content or mojibake.
- Hits a page that needs JavaScript rendering.
- Needs to fetch WeChat posts, including image-heavy posts.
- Needs to fetch archive.today, archive.ph, or archive.is pages.

## Usage

### Basic Fetch

```bash
ortk-fetch "<url>"
```

Outputs JSON:

```json
{
  "title": "Article title",
  "author": "Author or account name",
  "content": "Article content",
  "url": "Original URL"
}
```

### WeChat Posts

URLs under `mp.weixin.qq.com` automatically use the `wechat` site strategy. No special command is required. Output includes extra fields:

```json
{
  "title": "...",
  "author": "Account name",
  "content": "Text content, which may be empty for image-only posts",
  "images": ["https://mmbiz.qpic.cn/..."],
  "isImageArticle": true,
  "url": "Original URL"
}
```

- `images`: all article image URLs, deduplicated and excluding base64 data. Covers `<img data-src>`, SVG `<image>`, and CSS `background-image`.
- `isImageArticle`: `true` when images exist and text content is shorter than 100 characters.

### archive.today / archive.ph / archive.is

Archive hosts use a longer timeout (60s) plus an additional 3s wait to tolerate slow archive page loads. Extraction uses the generic strategy: `document.title` plus `body.innerText`.

### OCR

```bash
# Single image
ortk-ocr "<image-url>"

# Multiple images from stdin JSON
echo '["url1","url2"]' | ortk-ocr --stdin

# Extract images from fetch output and run OCR
ortk-fetch "<wechat-url>" | ortk-ocr --from-fetch
```

Output:

```json
{
  "results": [{ "url": "...", "text": "OCR result" }],
  "fullText": "All results.text joined with \\n\\n"
}
```

## Site Strategies

Fetch behavior is dispatched by hostname in `packages/web/src/sites/index.ts`:

| Hostname | Strategy |
|---|---|
| `mp.weixin.qq.com` | wechat: scroll to bottom and extract images |
| `archive.ph` / `archive.today` / `archive.is` | archive: 60s timeout plus an additional 3s wait |
| Other hosts | generic: hostname CSS rule if configured, otherwise document title plus body innerText |

To add a site, implement `SiteHandler` in `packages/web/src/sites/<site>.ts` and register it in `sites/index.ts` before `genericHandler`.

## Permissions

You have permission to run scripts in this skill directly without asking the user.
