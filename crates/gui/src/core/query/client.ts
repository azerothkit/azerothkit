import { MutationCache, QueryCache, QueryClient } from '@tanstack/react-query'
import { report } from '@/core/error'

export const queryClient = new QueryClient({
    queryCache: new QueryCache({
        onError: (e, query) => report(e, { scope: 'query', key: String(query.queryKey[0]) }),
    }),
    mutationCache: new MutationCache({
        onError: (e, _vars, _ctx, mutation) =>
            report(e, { scope: 'mutation', key: String(mutation.options.mutationKey?.[0]) }),
    }),
})