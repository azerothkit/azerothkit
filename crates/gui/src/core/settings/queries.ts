import { queryOptions } from '@tanstack/react-query'
import { settingsApi } from './api'

export const settingsSnapshotQuery = queryOptions({
    queryKey: ['settings', 'snapshot'] as const,
    queryFn: settingsApi.getSnapshot,
    networkMode: 'always',
    staleTime: Infinity,
    gcTime: Infinity,
    retry: false,
})