export class ApiRequestError extends Error {
  constructor(
    readonly status: number,
    readonly code: string | null,
    readonly retryAfterSeconds: number | null,
  ) {
    super(code ?? `HTTP ${status}`);
    this.name = 'ApiRequestError';
  }
}

export async function readApiError(response: Response): Promise<ApiRequestError> {
  // Proxies and framework-level errors may return plain text or empty bodies.
  let payload: unknown = null;
  try {
    payload = await response.json();
  } catch {
    // Fall back to the HTTP status.
  }

  const body =
    payload !== null && typeof payload === 'object' && !Array.isArray(payload)
      ? (payload as Record<string, unknown>)
      : null;

  const code = typeof body?.code === 'string' ? body.code : null;
  const bodyWait = body?.retry_after_seconds;
  const headerWait = response.headers.get('Retry-After');
  const parsedHeaderWait =
    headerWait !== null && /^\d+$/.test(headerWait)
      ? Number(headerWait)
      : null;

  const retryAfterSeconds =
    typeof bodyWait === 'number' &&
    Number.isSafeInteger(bodyWait) &&
    bodyWait > 0
      ? bodyWait
      : parsedHeaderWait !== null &&
          Number.isSafeInteger(parsedHeaderWait) &&
          parsedHeaderWait > 0
        ? parsedHeaderWait
        : null;

  return new ApiRequestError(response.status, code, retryAfterSeconds);
}
