import 'virtual:uno.css';
import './styles/global.css';

import { QueryClientProvider } from '@tanstack/solid-query';
import { RouterProvider } from '@tanstack/solid-router';

import MiniAppSessionBoundary from './app/miniapp-session-boundary';
import { queryClient } from './app/query-client';
import { router } from './router';

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <MiniAppSessionBoundary>
        <RouterProvider router={router} />
      </MiniAppSessionBoundary>
    </QueryClientProvider>
  );
}
