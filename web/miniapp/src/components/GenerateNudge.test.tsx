import { QueryClient, QueryClientProvider } from '@tanstack/solid-query';
import { cleanup, fireEvent, render, screen, waitFor } from '@solidjs/testing-library';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { ApiRequestError } from '../api/error';
import { generateNudge } from '../api/nudge';
import GenerateNudge from './GenerateNudge.tsrx';

vi.mock('../api/nudge', () => ({
  generateNudge: vi.fn(),
}));

function renderNudge() {
  const client = new QueryClient();

  return render(() => (
    <QueryClientProvider client={client}>
      <GenerateNudge />
    </QueryClientProvider>
  ));
}

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.resetAllMocks();
});

describe('GenerateNudge', () => {
  it('mounts under Solid 2 without a runtime effect error', () => {
    renderNudge();

    expect(screen.getByRole('button', { name: 'Получить сообщение' })).toBeEnabled();
    expect(screen.getByText('Сообщение появится в чате с ботом')).toBeInTheDocument();
  });

  it('shows success after a generated message is queued', async () => {
    vi.mocked(generateNudge).mockResolvedValueOnce({ text: 'Привет!' });
    renderNudge();

    fireEvent.click(screen.getByRole('button', { name: 'Получить сообщение' }));

    expect(await screen.findByText('Готово. Загляни в чат.')).toBeInTheDocument();
    expect(generateNudge).toHaveBeenCalledTimes(1);
  });

  it.each([
    ['rate_limited', 42, 'Через 00:42', 'Не торопись.'],
    ['daily_limit', 4800, 'Через 1 ч 20 мин', 'На сегодня всё.'],
  ] as const)('disables the button for %s', async (code, seconds, label, note) => {
    vi.mocked(generateNudge).mockRejectedValueOnce(new ApiRequestError(429, code, seconds));
    const { unmount } = renderNudge();

    fireEvent.click(screen.getByRole('button', { name: 'Получить сообщение' }));

    expect(await screen.findByText(note)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: label })).toBeDisabled();

    const clearInterval = vi.spyOn(window, 'clearInterval');
    unmount();
    expect(clearInterval).toHaveBeenCalled();
  });

  it('unlocks after the server-provided deadline on window focus', async () => {
    const clock = vi.spyOn(Date, 'now').mockReturnValue(1_000_000);
    vi.mocked(generateNudge).mockRejectedValueOnce(
      new ApiRequestError(429, 'rate_limited', 3),
    );
    renderNudge();

    fireEvent.click(screen.getByRole('button', { name: 'Получить сообщение' }));
    expect(await screen.findByText('Не торопись.')).toBeInTheDocument();

    clock.mockReturnValue(1_003_001);
    window.dispatchEvent(new Event('focus'));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Получить сообщение' })).toBeEnabled();
    });
  });

  it('shows an ordinary error without blocking the button', async () => {
    vi.mocked(generateNudge).mockRejectedValueOnce(new Error('Network error'));
    renderNudge();

    fireEvent.click(screen.getByRole('button', { name: 'Получить сообщение' }));

    expect(
      await screen.findByText('Не удалось получить сообщение. Попробуй ещё раз.'),
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Получить сообщение' })).toBeEnabled();
  });
});
