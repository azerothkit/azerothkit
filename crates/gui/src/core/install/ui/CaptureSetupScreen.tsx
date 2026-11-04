import { useEffect } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { getCurrentWindow } from '@tauri-apps/api/window'
import type { CapturePlatform, CaptureSetupStatus } from '@/shared/api'
import { report } from '@/core/error'
import { installApi } from '../api'
import { captureSetupQuery } from '../queries'
import { InstallScreen } from './InstallScreen'

const instructions: Record<CapturePlatform, { title: string; description: string }> = {
    macos: {
        title: 'Enable network capture',
        description:
            'Open System Settings → General → Login Items & Extensions → Network Extensions / Extensions. Enable Mitmproxy Redirector and confirm the change.\n\nIf it is not listed, the capture component must first be installed and activated.',
    },
    windows: {
        title: 'Allow network capture',
        description:
            'Confirm the administrator permission request when starting network capture, then check again.',
    },
    linux: {
        title: 'Allow network capture',
        description:
            'Network capture requires administrator authorization. Confirm the system permission request, then check again.',
    },
    other: {
        title: 'Network capture unavailable',
        description: 'Network capture setup is not available on this platform.',
    },
}

type CaptureSetupScreenProps = {
    setup: CaptureSetupStatus
    checking: boolean
    onRetry: () => void
}

export function CaptureSetupScreen({ setup, checking, onRetry }: CaptureSetupScreenProps) {
    const queryClient = useQueryClient()
    const copy = instructions[setup.platform]

    const openSettings = useMutation({
        mutationKey: ['install', 'capture', 'settings'],
        mutationFn: installApi.openCaptureSettings,
        networkMode: 'always',
        retry: false,
    })

    useEffect(() => {
        let disposed = false
        let unlisten: (() => void) | undefined

        void getCurrentWindow()
            .onFocusChanged(({ payload: focused }) => {
                if (disposed || !focused) return

                void queryClient.invalidateQueries({
                    queryKey: captureSetupQuery.queryKey,
                    exact: true,
                })
            })
            .then((stopListening) => {
                if (disposed) {
                    stopListening()
                    return
                }

                unlisten = stopListening
            })
            .catch((error: unknown) => {
                if (!disposed) {
                    report(error, { scope: 'install.capture.focus' })
                }
            })

        return () => {
            disposed = true
            unlisten?.()
        }
    }, [queryClient])

    const retryAction = {
        label: checking ? 'Checking...' : 'Check again',
        disabled: checking || openSettings.isPending,
        onClick: onRetry,
    }

    return (
        <InstallScreen
            title={copy.title}
            description={copy.description}
            action={
                setup.platform === 'macos'
                    ? {
                          label: 'Open settings',
                          disabled: openSettings.isPending,
                          onClick: () => {
                              openSettings.mutate()
                          },
                      }
                    : retryAction
            }
            secondaryAction={setup.platform === 'macos' ? retryAction : undefined}
        />
    )
}