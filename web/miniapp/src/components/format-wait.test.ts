import { describe, expect, it } from 'vitest';

import { formatWait } from './format-wait';

describe('formatWait', () => {
  it('uses mm:ss for short waits', () => {
    expect(formatWait(42)).toBe('00:42');
    expect(formatWait(60)).toBe('01:00');
    expect(formatWait(3599)).toBe('59:59');
  });

  it('uses hours and minutes for longer waits', () => {
    expect(formatWait(3600)).toBe('1 ч');
    expect(formatWait(3601)).toBe('1 ч 1 мин');
    expect(formatWait(8 * 3600 + 20 * 60)).toBe('8 ч 20 мин');
    expect(formatWait(86399)).toBe('1 д');
  });

  it('supports a wait of one day or more', () => {
    expect(formatWait(86400)).toBe('1 д');
    expect(formatWait(90000)).toBe('1 д 1 ч');
  });
});
