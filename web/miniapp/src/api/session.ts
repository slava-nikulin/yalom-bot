export async function establishSession(initData: string): Promise<void> {
  const response = await fetch('/miniapp/session', {
    method: 'POST',
    headers: {
      'X-Tma-Init-Data': initData,
    },
    credentials: 'same-origin',
  });

  if (!response.ok) {
    throw new Error(`Failed to establish Mini App session: ${response.status}`);
  }
}