import { readApiError } from './error';

export interface NudgeResponse {
  text: string;
}

export async function generateNudge(): Promise<NudgeResponse> {
  const timezone = Intl.DateTimeFormat().resolvedOptions().timeZone;

  const response = await fetch('/api/miniapp/nudge', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    credentials: 'same-origin',
    body: JSON.stringify({ timezone }),
  });

  if (!response.ok) {
    throw await readApiError(response);
  }

  return response.json() as Promise<NudgeResponse>;
}
