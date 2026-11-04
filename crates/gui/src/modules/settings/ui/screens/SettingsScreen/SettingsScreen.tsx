import { useQuery } from '@tanstack/react-query'
import { settingsSnapshotQuery } from '@/core/settings'
import { Button } from '@/shared/ui'
import { Alert, AlertDescription, AlertTitle } from '@/shared/ui'
import { ClientSettingsEditor } from '../../components/ClientSettingsEditor'
import { PageSkeleton } from '@/shared/ui'

export const SettingsScreen = () => {
    const query = useQuery(settingsSnapshotQuery)

    if (query.isPending) {
        return <PageSkeleton />
    }

    if (!query.data) {
        return (
            <Alert variant="destructive">
                <AlertTitle>Could not load settings</AlertTitle>

                <AlertDescription className="space-y-3">
                    <p>{query.error?.message}</p>

                    <Button
                        type="button"
                        variant="outline"
                        disabled={query.isFetching}
                        onClick={() => void query.refetch()}
                    >
                        Retry
                    </Button>
                </AlertDescription>
            </Alert>
        )
    }

    const { clients, settings } = query.data
    const client = clients.find((item) => item.id === settings.selectedClient)

    if (!client) {
        return (
            <Alert variant="destructive">
                <AlertTitle>Unknown client</AlertTitle>
                <AlertDescription>
                    The selected client is missing from the settings snapshot.
                </AlertDescription>
            </Alert>
        )
    }

    return (
        <section className="space-y-6">
            <div className="space-y-2">
                <h1 className="text-lg font-semibold tracking-tight">Settings</h1>
            </div>

            <ClientSettingsEditor
                key={client.id}
                clients={clients}
                client={client}
                settings={settings.settingsByClient[client.id]}
            />
        </section>
    )
}