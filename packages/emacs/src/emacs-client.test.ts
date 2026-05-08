import { describe, expect, it } from 'vitest';
import {
  buildKeywordArgs,
  escapeElispString,
  parseElispResult,
  quoteElispString,
} from './emacs-client.js';

describe('parseElispResult', () => {
  it('parses nil as null', () => {
    expect(parseElispResult('nil')).toBeNull();
  });

  it('parses t as true', () => {
    expect(parseElispResult('t')).toBe(true);
  });

  it('parses integers and floats', () => {
    expect(parseElispResult('42')).toBe(42);
    expect(parseElispResult('-7')).toBe(-7);
    expect(parseElispResult('3.14')).toBe(3.14);
  });

  it('unquotes elisp strings and unescapes \\" and \\\\', () => {
    expect(parseElispResult('"hello"')).toBe('hello');
    expect(parseElispResult('"a\\"b"')).toBe('a"b');
    expect(parseElispResult('"a\\\\b"')).toBe('a\\b');
  });

  it('decodes \\n \\t \\r escape sequences', () => {
    expect(parseElispResult('"line1\\nline2"')).toBe('line1\nline2');
    expect(parseElispResult('"col1\\tcol2"')).toBe('col1\tcol2');
    expect(parseElispResult('"a\\rb"')).toBe('a\rb');
  });

  it('preserves backslash followed by non-escape char (drops the backslash)', () => {
    // elisp would not normally print such a sequence, but be lenient
    expect(parseElispResult('"a\\zb"')).toBe('azb');
  });

  it('handles backslash before quote (\\\\\\")', () => {
    // raw bytes: \ \ \ " → string content: \ "
    expect(parseElispResult('"\\\\\\""')).toBe('\\"');
  });

  it('returns raw string for complex structures', () => {
    expect(parseElispResult('(1 2 3)')).toBe('(1 2 3)');
  });
});

describe('escapeElispString / quoteElispString', () => {
  it('escapes backslashes and quotes', () => {
    expect(escapeElispString('a"b')).toBe('a\\"b');
    expect(escapeElispString('a\\b')).toBe('a\\\\b');
  });

  it('quotes a string into an elisp literal', () => {
    expect(quoteElispString('hello')).toBe('"hello"');
    expect(quoteElispString('a"b')).toBe('"a\\"b"');
  });
});

describe('buildKeywordArgs', () => {
  it('camelCase → :kebab-case keywords', () => {
    expect(buildKeywordArgs({ sourceUrl: 'http://x' })).toBe(
      ':source-url "http://x"',
    );
  });

  it('handles leading-capital keys without producing :- prefix', () => {
    expect(buildKeywordArgs({ Foo: 1 })).toBe(':foo 1');
    expect(buildKeywordArgs({ OpenArchive: true })).toBe(':open-archive t');
  });

  it('skips null/undefined values', () => {
    expect(buildKeywordArgs({ a: 'x', b: null, c: undefined })).toBe(':a "x"');
  });

  it('encodes booleans, numbers, arrays', () => {
    expect(buildKeywordArgs({ flag: true, count: 3 })).toBe(
      ':flag t :count 3',
    );
    expect(buildKeywordArgs({ tags: ['a', 'b'] })).toBe(":tags '(\"a\" \"b\")");
  });

  it('encodes objects as alist with string keys', () => {
    expect(buildKeywordArgs({ properties: { K: 'V' } })).toBe(
      ":properties '((\"K\" . \"V\"))",
    );
  });

  it('escapes quotes inside string values', () => {
    expect(buildKeywordArgs({ x: 'a"b' })).toBe(':x "a\\"b"');
  });

  it('escapes quotes inside array string items', () => {
    expect(buildKeywordArgs({ tags: ['a"b'] })).toBe(":tags '(\"a\\\"b\")");
  });
});
