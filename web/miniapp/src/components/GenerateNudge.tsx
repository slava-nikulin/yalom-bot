import { useMutation } from '@tanstack/solid-query';
import { createEffect, createMemo, createSignal, onCleanup } from 'solid-js';

import { ApiRequestError } from '../api/error';
import { generateNudge } from '../api/nudge';
import { formatWait } from './format-wait';

type RateLimitCode = 'rate_limited' | 'daily_limit';

interface RateLimit {
  code: RateLimitCode;
  until: number;
}

function errorMessage(error: unknown): string {
  if (!(error instanceof ApiRequestError)) {
    return 'Не удалось получить сообщение. Попробуй ещё раз.';
  }

  switch (error.code) {
    case 'inappropriate_time':
      return 'Ночью сообщения недоступны.';
    case 'invalid_timezone':
      return 'Не удалось определить часовой пояс.';
    case 'user_not_found':
      return 'Сначала открой бота и отправь /start.';
    case 'unauthorized':
      return 'Сессия истекла. Открой приложение заново.';
    case 'rate_limited':
    case 'daily_limit':
      return 'Лимит запросов. Попробуй позже.';
    default:
      return 'Не удалось получить сообщение. Попробуй ещё раз.';
  }
}

export default function GenerateNudge() {
  const [limit, setLimit] = createSignal<RateLimit | null>(null);
  const [now, setNow] = createSignal(Date.now());

  // Derive the remaining time from a deadline, not a decremented counter.
  createEffect(() => {
    if (!limit()) return;

    const tick = () => setNow(Date.now());
    tick();

    const interval = window.setInterval(tick, 1000);
    window.addEventListener('focus', tick);
    document.addEventListener('visibilitychange', tick);

    onCleanup(() => {
      window.clearInterval(interval);
      window.removeEventListener('focus', tick);
      document.removeEventListener('visibilitychange', tick);
    });
  });

  const remainingSeconds = createMemo(() => {
    const current = limit();
    if (!current) return 0;

    return Math.max(0, Math.ceil((current.until - now()) / 1000));
  });

  const mutation = useMutation(() => ({
    mutationFn: generateNudge,
    retry: false,
    onSuccess: () => setLimit(null),
    onError: (error) => {
      if (
        error instanceof ApiRequestError &&
        (error.code === 'rate_limited' || error.code === 'daily_limit') &&
        error.retryAfterSeconds !== null
      ) {
        setLimit({
          code: error.code,
          until: Date.now() + error.retryAfterSeconds * 1000,
        });
      } else {
        setLimit(null);
      }
    },
  }));

  const buttonLabel = createMemo(() => {
    const seconds = remainingSeconds();
    if (seconds > 0) return `Через ${formatWait(seconds)}`;
    return mutation.isPending ? 'Генерирую…' : 'Получить сообщение';
  });

  const note = createMemo(() => {
    if (remainingSeconds() > 0) {
      return limit()?.code === 'daily_limit' ? 'На сегодня всё.' : 'Не торопись.';
    }

    if (mutation.isPending) return null;
    if (mutation.isSuccess) return 'Готово. Загляни в чат.';

    if (mutation.isError) {
      const error = mutation.error;
      if (
        error instanceof ApiRequestError &&
        (error.code === 'rate_limited' || error.code === 'daily_limit') &&
        limit()
      ) {
        return 'Сообщение появится в чате с ботом';
      }
      return errorMessage(error);
    }

    return 'Сообщение появится в чате с ботом';
  });

  return (
    <section aria-label="Ручная генерация сообщения">
      <button
        type="button"
        disabled={mutation.isPending || remainingSeconds() > 0}
        onClick={() => mutation.mutate()}
        class="flex h-14 w-full items-center justify-center rounded-2xl bg-[var(--app-button)] px-5 text-base font-semibold text-[var(--app-button-text)] transition-opacity active:opacity-85 disabled:cursor-not-allowed disabled:opacity-50"
      >
        {buttonLabel()}
      </button>

      <p
        aria-live="polite"
        class="mt-3 min-h-5 text-center text-sm leading-5 text-[var(--app-hint)]"
      >
        {note()}
      </p>
    </section>
  );
}
