export interface MenuState {
  paused: boolean;
}

export async function getMenuState(): Promise<MenuState> {
  const response = await fetch('/api/miniapp/menu');

  if (!response.ok) {
    throw new Error(`Failed to load menu: ${response.status}`);
  }

  return response.json();
}

export async function updateMenuState(paused: boolean): Promise<MenuState> {
  const response = await fetch('/api/miniapp/menu', {
    method: 'PATCH',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ paused }),
  });

  if (!response.ok) {
    throw new Error(`Failed to update menu: ${response.status}`);
  }

  return response.json();
}