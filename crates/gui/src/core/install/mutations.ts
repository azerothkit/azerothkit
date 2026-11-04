import { useMutation, useQueryClient } from '@tanstack/react-query'
import { installApi } from './api'
import { tlsCertificateStatusQuery } from './queries'

export function useInstallTlsCertificate() {
    const queryClient = useQueryClient()

    return useMutation({
        mutationKey: ['install', 'tls', 'write'],
        mutationFn: installApi.installTlsCertificate,
        scope: { id: 'tls-installation' },
        networkMode: 'always',
        retry: false,

        onSettled: () => {
            return queryClient.resetQueries({
                queryKey: tlsCertificateStatusQuery.queryKey,
                exact: true,
            })
        },
    })
}