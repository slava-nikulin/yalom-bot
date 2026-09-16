export async function establishSession(initData: string): Promise<void> {
  const response = await fetch('/api/miniapp/session', {
    method: 'POST',
    headers: {
      'content-type': 'application/json',
    },
    credentials: 'same-origin',
    body: JSON.stringify({ initData }),
  });

  if (!response.ok) {
    throw new Error(`Failed to establish Mini App session: ${response.status}`);
  }
}
