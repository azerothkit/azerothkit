import { useQuery } from '@tanstack/react-query'
import { commands } from '@/shared/api'

export function useAppInfo() {
    return useQuery({
        queryKey: ['app', 'info'],
        queryFn: () => commands.getAppInfo(),
        staleTime: Infinity,
        gcTime: Infinity,
        networkMode: 'always',
    })
}