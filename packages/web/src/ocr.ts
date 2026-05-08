import { createWorker, type Worker } from 'tesseract.js';

export interface OcrResult {
  results: Array<{ url: string; text: string }>;
  fullText: string;
}

async function newWorker(lang = 'chi_sim'): Promise<Worker> {
  return createWorker(lang, 1, { logger: () => {} });
}

export async function ocrImages(
  urls: string[],
  lang = 'chi_sim',
): Promise<OcrResult> {
  if (urls.length === 0) return { results: [], fullText: '' };

  const worker = await newWorker(lang);
  try {
    const results = [] as OcrResult['results'];
    for (const url of urls) {
      const {
        data: { text },
      } = await worker.recognize(url);
      results.push({ url, text: text.trim() });
    }
    return { results, fullText: results.map((r) => r.text).join('\n\n') };
  } finally {
    await worker.terminate();
  }
}
