import type { SiteHandler } from '../types.js';
import { genericExtract } from './generic.js';

const ARCHIVE_HOSTS = new Set(['archive.ph', 'archive.today', 'archive.is']);

/**
 * archive.* sites need a longer timeout and a fixed extra wait.
 * Extraction itself is generic.
 */
export const archiveHandler: SiteHandler = {
  name: 'archive',
  match: (url) => ARCHIVE_HOSTS.has(url.hostname),
  navOptions: { waitUntil: 'domcontentloaded', timeout: 60_000 },
  postNavWait: 3000,
  extract: genericExtract,
};
