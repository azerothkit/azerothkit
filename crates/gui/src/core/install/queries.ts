import { queryOptions } from '@tanstack/react-query'
import { installApi } from './api'

export const tlsCertificateStatusQuery = queryOptions({
    queryKey: ['install', 'tls', 'status'] as const,
    queryFn: installApi.getTlsStatus,
    networkMode: 'always',
    staleTime: Infinity,
    gcTime: Infinity,
    retry: false,
    refetchOnMount: 'always',
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
})

export const captureSetupQuery = queryOptions({
    queryKey: ['install', 'capture', 'status'] as const,
    queryFn: installApi.getCaptureStatus,
    networkMode: 'always',
    staleTime: Infinity,
    gcTime: Infinity,
    retry: false,
    refetchOnMount: 'always',
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
})