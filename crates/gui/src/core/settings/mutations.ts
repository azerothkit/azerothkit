import { useMutation, useQueryClient } from '@tanstack/react-query'
import type { AppSettings, SettingsSnapshot } from '@/shared/api'
import { settingsApi } from './api'
import { settingsSnapshotQuery } from './queries'

function useSettingsMutation<TVariables>(
    mutationFn: (variables: TVariables) => Promise<AppSettings>,
    onSaved?: (settings: AppSettings) => void,
) {
    const queryClient = useQueryClient()

    return useMutation({
        mutationFn,
        mutationKey: ['settings', 'write'],
        scope: { id: 'settings-write' },
        networkMode: 'always',
        retry: false,

        onMutate: async () => {
            await queryClient.cancelQueries({
                queryKey: settingsSnapshotQuery.queryKey,
            })
        },

        onSuccess: async (settings) => {
            await queryClient.cancelQueries({
                queryKey: settingsSnapshotQuery.queryKey,
            })

            queryClient.setQueryData<SettingsSnapshot>(
                settingsSnapshotQuery.queryKey,
                (snapshot) => {
                    if (!snapshot) return undefined

                    return {
                        ...snapshot,
                        settings,
                    }
                },
            )

            onSaved?.(settings)
        },
    })
}

export const useSetSelectedClient = (onSaved?: () => void) => {
    return useSettingsMutation(settingsApi.setSelectedClient, onSaved)
}

export const useSaveClientSettings = () => {
    return useSettingsMutation(settingsApi.saveClientSettings)
}