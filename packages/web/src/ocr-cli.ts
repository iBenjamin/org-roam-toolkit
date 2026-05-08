#!/usr/bin/env node
/**
 * ortk-ocr — run OCR on one or more image URLs.
 *
 *   ortk-ocr <image-url>            # single URL
 *   ortk-ocr --stdin                # JSON array of URLs on stdin
 *   ortk-ocr --from-fetch           # fetch JSON on stdin; pulls .images
 */

import { ocrImages } from './ocr.js';

async function readStdin(): Promise<string> {
  const chunks: Buffer[] = [];
  for await (const chunk of process.stdin) {
    chunks.push(chunk as Buffer);
  }
  return Buffer.concat(chunks).toString('utf8').trim();
}

function usage(): never {
  console.error(
    JSON.stringify({
      error: 'Usage: ortk-ocr <image-url> | --stdin | --from-fetch',
    }),
  );
  process.exit(1);
}

async function main(): Promise<void> {
  const args = process.argv.slice(2);
  let urls: string[] = [];

  if (args.includes('--stdin')) {
    urls = JSON.parse(await readStdin());
  } else if (args.includes('--from-fetch')) {
    const data = JSON.parse(await readStdin());
    urls = (data.images ?? []) as string[];
  } else if (args.length > 0 && !args[0]!.startsWith('-')) {
    urls = [args[0]!];
  } else {
    usage();
  }

  const result = await ocrImages(urls);
  console.log(JSON.stringify(result, null, 2));
}

main().catch((e: unknown) => {
  const msg = e instanceof Error ? e.message : String(e);
  console.error(JSON.stringify({ error: msg }));
  process.exit(1);
});
