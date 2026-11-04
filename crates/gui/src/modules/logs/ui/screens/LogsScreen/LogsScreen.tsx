import { Card, CardDescription, CardHeader, CardTitle } from '@/shared/ui'

export const LogsScreen = () => {
    return (
        <section className="space-y-5">
            <div className="space-y-2">
                <h1 className="text-lg font-semibold tracking-tight">Logs</h1>

                <p className="text-sm text-muted-foreground">
                    Runtime messages from the Rust service.
                </p>
            </div>

            <Card>
                <CardHeader>
                    <CardTitle>No logs yet</CardTitle>

                    <CardDescription>
                        The Rust service is not connected in this preview.
                    </CardDescription>
                </CardHeader>
            </Card>
        </section>
    )
}