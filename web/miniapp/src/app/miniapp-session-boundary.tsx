import type { ParentProps } from 'solid-js';
import { Match, Switch, onSettled } from 'solid-js';
import { useMutation } from '@tanstack/solid-query';

import { establishSession } from '../api/session';
import { getTelegramInitData } from '../telegram/webapp';

export default function MiniAppSessionBoundary(props: ParentProps) {
  const session = useMutation(() => ({
    mutationKey: ['miniapp', 'session'],
    mutationFn: () => establishSession(getTelegramInitData()),
  }));

  onSettled(() => {
    void session.mutate();
  });

  return (
    <Switch>
      <Match when={session.isSuccess}>{props.children}</Match>

      <Match when={session.isError}>
        <main class="grid min-h-dvh place-items-center p-6 text-center">
          <p>Failed to start Mini App.</p>
        </main>
      </Match>

      <Match when={true}>
        <main class="grid min-h-dvh place-items-center p-6 text-center">
          <p>Starting Yalom…</p>
        </main>
      </Match>
    </Switch>
  );
}
