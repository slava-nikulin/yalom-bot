import { useQuery } from '@tanstack/solid-query';

import { menuQueryOptions } from '../queries/menu';

export default function Menu() @{
  const menu = useQuery(() => menuQueryOptions);

  <main class="
    min-h-dvh
    bg-[var(--app-bg)]
    pl-[calc(1rem+var(--app-safe-left))]
    pr-[calc(1rem+var(--app-safe-right))]
    pt-[calc(1.5rem+var(--app-safe-top))]
    pb-[calc(1.5rem+var(--app-safe-bottom))]
    text-[var(--app-text)]
  ">
    <section class="mx-auto flex w-full max-w-sm flex-col gap-6">
      <header class="space-y-1">
        <h1 class="text-2xl font-semibold tracking-tight">
          Yalom
        </h1>

        <p class="text-sm text-[var(--app-hint)]">
          Agent control
        </p>
      </header>

      @if (menu.isPending) {
        <div class="rounded-2xl border border-[var(--app-border)] bg-[var(--app-surface)] p-4">
          <p class="text-sm text-[var(--app-hint)]">
            Loading…
          </p>
        </div>
      }

      @if (menu.isError) {
        <div class="rounded-2xl border border-[var(--app-border)] bg-[var(--app-surface)] p-4">
          <p class="text-sm font-medium text-[var(--app-destructive)]">
            Failed to load menu.
          </p>
        </div>
      }

      @if (menu.isSuccess) {
        <>
          <div class="rounded-2xl border border-[var(--app-border)] bg-[var(--app-surface)] p-4">
            <div class="flex items-center justify-between gap-4">
              <span class="text-sm text-[var(--app-hint)]">
                Agent status
              </span>

              <span
                class="text-sm font-medium"
                aria-live="polite"
              >
                {menu.data.paused ? 'Paused' : 'Active'}
              </span>
            </div>
          </div>

          <div class="grid gap-3">
            <button
              type="button"
              disabled={menu.data.paused}
              class="
                h-12 w-full rounded-xl
                bg-[var(--app-button)]
                px-4 text-sm font-semibold
                text-[var(--app-button-text)]
                transition-opacity
                hover:opacity-90
                disabled:cursor-not-allowed
                disabled:opacity-40
              "
            >
              Pause
            </button>

            <button
              type="button"
              disabled={!menu.data.paused}
              class="
                h-12 w-full rounded-xl
                border border-[var(--app-border)]
                bg-[var(--app-surface)]
                px-4 text-sm font-semibold
                text-[var(--app-text)]
                transition-opacity
                hover:opacity-80
                disabled:cursor-not-allowed
                disabled:opacity-40
              "
            >
              Resume
            </button>
          </div>
        </>
      }
    </section>
  </main>
}