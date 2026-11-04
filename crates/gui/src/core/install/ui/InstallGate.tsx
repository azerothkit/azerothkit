import type { PropsWithChildren } from 'react'
import { useQuery } from '@tanstack/react-query'
import { tlsCertificateStatusQuery } from '../queries'
import { useInstallTlsCertificate } from '../mutations'
import { InstallScreen } from './InstallScreen'
import { TlsCheckScreen } from './TlsCheckScreen'
import { captureSetupQuery } from '../queries'
import { CaptureSetupScreen } from './CaptureSetupScreen'

export function InstallGate({ children }: PropsWithChildren) {
    const tlsStatus = useQuery(tlsCertificateStatusQuery)
    const installation = useInstallTlsCertificate()

    const captureStatus = useQuery({
        ...captureSetupQuery,
        enabled: tlsStatus.isSuccess && tlsStatus.data,
    })

    // Keep this screen mounted while installation and revalidation run.
    if (installation.isPending) {
        return (
            <InstallScreen
                title="Install TLS certificate"
                description="Confirm the system authorization prompt. AzerothKit will check the certificate when installation finishes."
                action={{
                    label: 'Installing...',
                    disabled: true,
                    onClick: () => {},
                }}
            />
        )
    }

    if (tlsStatus.isPending || tlsStatus.isFetching) {
        return <TlsCheckScreen />
    }

    if (tlsStatus.isError) {
        return (
            <InstallScreen
                title="Could not check TLS certificate"
                description={errorMessage(tlsStatus.error)}
                action={{
                    label: 'Try again',
                    onClick: () => {
                        void tlsStatus.refetch()
                    },
                }}
            />
        )
    }

    if (tlsStatus.data) {
        if (captureStatus.isPending) {
            return (
                <TlsCheckScreen
                    title="Checking network capture"
                    description="Please wait while AzerothKit checks the capture component."
                />
            )
        }

        if (captureStatus.isError) {
            return (
                <InstallScreen
                    title="Could not check network capture"
                    description={errorMessage(captureStatus.error)}
                    action={{
                        label: 'Try again',
                        onClick: () => {
                            void captureStatus.refetch()
                        },
                    }}
                />
            )
        }

        if (!captureStatus.data.ready) {
            return (
                <CaptureSetupScreen
                    setup={captureStatus.data}
                    checking={captureStatus.isFetching}
                    onRetry={() => {
                        void captureStatus.refetch()
                    }}
                />
            )
        }

        return <>{children}</>
    }

    let description =
        'AzerothKit needs a trusted local CA certificate to process game connections. Click Install certificate and confirm the system authorization prompt.'

    if (installation.isError) {
        description = errorMessage(installation.error)
    } else if (installation.isSuccess) {
        description = installation.data
            ? 'The certificate is still not trusted. Try installing it again.'
            : 'Installation was cancelled. Click Install certificate to try again.'
    }

    return (
        <InstallScreen
            title="Install TLS certificate"
            description={description}
            action={{
                label: 'Install certificate',
                onClick: () => {
                    installation.mutate()
                },
            }}
        />
    )
}

function errorMessage(error: unknown): string {
    if (error instanceof Error && error.message) {
        return error.message
    }

    if (typeof error === 'string' && error) {
        return error
    }

    return 'An unexpected error occurred. Please try again.'
}