import { queryOptions } from '@tanstack/solid-query';

import { getMenuState } from '../api/menu';

export const menuQueryOptions = queryOptions({
  queryKey: ['menu'],
  queryFn: getMenuState,
});