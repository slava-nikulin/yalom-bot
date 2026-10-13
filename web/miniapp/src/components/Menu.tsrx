import GenerateNudge from './GenerateNudge.tsrx';

export default function Menu() @{
  <main class="
    min-h-dvh
    bg-[var(--app-bg)]
    pl-[calc(1rem+var(--app-safe-left))]
    pr-[calc(1rem+var(--app-safe-right))]
    pt-[calc(2rem+var(--app-safe-top))]
    pb-[calc(2rem+var(--app-safe-bottom))]
    text-[var(--app-text)]
  ">
    <div class="mx-auto w-full max-w-sm">
      <header class="mb-9">
        <h1 class="text-3xl font-semibold tracking-tight">Yalom</h1>
      </header>

      <GenerateNudge />

      <section class="mt-12 border-t border-[var(--app-border)] pt-5">
        <div class="flex items-center justify-between gap-4">
          <span class="text-sm font-medium">
            Автоматические сообщения
          </span>
          <span class="text-xs text-[var(--app-hint)]">
            Скоро
          </span>
        </div>
      </section>
    </div>
  </main>
}
