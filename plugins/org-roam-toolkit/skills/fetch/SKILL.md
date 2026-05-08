---
name: fetch
description: |
  Playwright headless browser for fetching web pages that require JavaScript rendering.
  Use when WebFetch fails or returns empty content. Includes per-site extraction strategy
  (WeChat 图文 with image extraction, archive.* with extended timeout) and OCR helpers.

  Triggers: WebFetch失败, 微信公众号, mp.weixin.qq.com, 需要JS渲染, archive.today, archive.ph
---

# Fetch Skill

抓取需要 JavaScript 渲染的网页，并按域名自动选择提取策略。

## 何时使用

当 WebFetch 工具：
- 返回空内容或乱码
- 遇到需要 JS 渲染的页面
- 抓取微信公众号文章（图文 / 纯图片均支持）
- 抓取 archive.today / archive.ph / archive.is

## 用法

### 普通 fetch

```bash
ortk-fetch "<url>"
```

输出 JSON：

```json
{
  "title": "文章标题",
  "author": "作者/公众号名",
  "content": "正文内容",
  "url": "原始URL"
}
```

### 微信图文文章

微信 URL（`mp.weixin.qq.com`）会自动走 wechat 站点策略，**无需特殊命令**。输出额外字段：

```json
{
  "title": "...",
  "author": "公众号名",
  "content": "正文文字（图片文章可能为空）",
  "images": ["https://mmbiz.qpic.cn/..."],
  "isImageArticle": true,
  "url": "原始URL"
}
```

- `images`：文章中所有图片 URL（去重、过滤 base64），覆盖 `<img data-src>`、SVG `<image>`、CSS `background-image`
- `isImageArticle`：图片 > 0 且正文 < 100 字符时为 `true`

### archive.today / archive.ph / archive.is

自动用更长的超时（60s）和 3s 额外等待，以容忍 archive 站点的慢加载。提取使用通用策略（document.title + body.innerText）。

### OCR

```bash
# 单图
ortk-ocr "<image-url>"

# 多图（stdin JSON 数组）
echo '["url1","url2"]' | ortk-ocr --stdin

# 从 fetch 输出提取图片再 OCR
ortk-fetch "<wechat-url>" | ortk-ocr --from-fetch
```

输出：

```json
{
  "results": [{ "url": "...", "text": "OCR 结果" }],
  "fullText": "所有 results.text 用 \\n\\n 拼接"
}
```

## 站点策略

抓取行为按域名调度（在 `packages/web/src/sites/index.ts` 注册）：

| 域名 | 策略 |
|---|---|
| `mp.weixin.qq.com` | wechat（scrollToBottom + 图片提取） |
| `archive.ph` / `archive.today` / `archive.is` | archive（60s 超时 + 3s 额外等待） |
| 其他 | generic（按 hostname 查 CSS rule，否则 document.title + body innerText） |

新增站点：在 `packages/web/src/sites/<site>.ts` 实现 `SiteHandler`，到 `sites/index.ts` 注册（必须在 `genericHandler` **之前**）。

## 权限

你有权限直接运行此 skill 下的 scripts/，无需询问用户。
