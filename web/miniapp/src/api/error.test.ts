import { describe, expect, it } from 'vitest';

import { readApiError } from './error';

describe('readApiError', () => {
  it('reads a backend error code and retry delay', async () => {
    const response = new Response(
      JSON.stringify({ code: 'daily_limit', retry_after_seconds: 481 }),
      {
        status: 429,
        headers: {
          'Content-Type': 'application/json',
          'Retry-After': '481',
        },
      },
    );

    const error = await readApiError(response);

    expect(error.status).toBe(429);
    expect(error.code).toBe('daily_limit');
    expect(error.retryAfterSeconds).toBe(481);
  });

  it('accepts a Retry-After header if the response is not JSON', async () => {
    const response = new Response('Too many requests', {
      status: 429,
      headers: { 'Retry-After': '17' },
    });

    const error = await readApiError(response);

    expect(error.status).toBe(429);
    expect(error.code).toBeNull();
    expect(error.retryAfterSeconds).toBe(17);
  });

  it('handles ordinary proxy errors without JSON or retry headers', async () => {
    const response = new Response('Bad gateway', { status: 502 });

    const error = await readApiError(response);

    expect(error.status).toBe(502);
    expect(error.code).toBeNull();
    expect(error.retryAfterSeconds).toBeNull();
  });
});
