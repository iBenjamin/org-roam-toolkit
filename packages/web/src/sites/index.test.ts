import { describe, expect, it } from 'vitest';
import { pickHandler } from './index.js';

const pick = (s: string): string => pickHandler(new URL(s)).name;

describe('pickHandler', () => {
  it('routes wechat URLs to the wechat handler', () => {
    expect(pick('https://mp.weixin.qq.com/s/abc')).toBe('wechat');
  });

  it('routes archive.* URLs to the archive handler', () => {
    expect(pick('https://archive.ph/xyz')).toBe('archive');
    expect(pick('https://archive.today/xyz')).toBe('archive');
    expect(pick('https://archive.is/xyz')).toBe('archive');
  });

  it('routes zhihu and unknown URLs to the generic handler', () => {
    expect(pick('https://zhuanlan.zhihu.com/p/12345')).toBe('generic');
    expect(pick('https://www.zhihu.com/question/1')).toBe('generic');
    expect(pick('https://example.com/whatever')).toBe('generic');
  });
});
