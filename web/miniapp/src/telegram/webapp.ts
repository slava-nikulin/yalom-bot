interface TelegramWebApp {
  readonly initData: string;
}

declare global {
  interface Window {
    Telegram?: {
      WebApp?: TelegramWebApp;
    };
  }
}

export function getTelegramInitData(): string {
  const initData = window.Telegram?.WebApp?.initData;

  if (!initData) {
    throw new Error('Telegram Mini App initData is unavailable');
  }

  return initData;
}
