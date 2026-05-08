#!/usr/bin/env node
import { fetchUrl } from './fetch.js';

async function main(): Promise<void> {
  const url = process.argv[2];
  if (!url) {
    console.error(JSON.stringify({ error: 'Usage: ortk-fetch <url>' }));
    process.exit(1);
  }
  try {
    const result = await fetchUrl(url);
    console.log(JSON.stringify(result, null, 2));
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    console.error(JSON.stringify({ error: msg, url }));
    process.exit(1);
  }
}

void main();
