export interface MenuState {
  paused: boolean;
}

export async function getMenuState(): Promise<MenuState> {
  // TODO: Replace this stub with the real Axum request:
  //
  // const response = await fetch('/miniapp/menu');
  // if (!response.ok) {
  //   throw new Error(`Failed to load menu: ${response.status}`);
  // }
  // return response.json();

  await new Promise((resolve) => setTimeout(resolve, 500));

  return {
    paused: false,
  };
}