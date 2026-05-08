/**
 * Site handler registry. Order matters: the first handler whose
 * match() returns true wins. genericHandler is a catch-all and must
 * come last.
 */

import type { SiteHandler } from '../types.js';
import { archiveHandler } from './archive.js';
import { wechatHandler } from './wechat.js';
import { genericHandler } from './generic.js';

export const HANDLERS: SiteHandler[] = [
  archiveHandler,
  wechatHandler,
  // Future: drop new handlers here, before genericHandler.
  genericHandler,
];

export function pickHandler(url: URL): SiteHandler {
  for (const h of HANDLERS) {
    if (h.match(url)) return h;
  }
  // genericHandler always matches; this is a safety net only.
  return genericHandler;
}
