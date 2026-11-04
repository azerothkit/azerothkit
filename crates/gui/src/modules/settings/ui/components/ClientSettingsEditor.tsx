import { FormProvider } from 'react-hook-form'
import { useBlocker } from '@tanstack/react-router'
import type { ClientDescriptor, ClientSettings } from '@/shared/api'
import { Button } from '@/shared/ui'
import { Alert, AlertDescription, AlertTitle } from '@/shared/ui'
import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
} from '@/shared/ui'
import { useTimeOverrideForm } from '../hooks/useTimeOverrideForm'
import { ClientSelector } from './ClientSelector'
import { TimeOverrideFields } from './TimeOverrideFields'
import { RestartEngineDialog } from '@/modules/settings/ui/components/RestartEngineDialog'

type ClientSettingsEditorProps = {
    clients: ClientDescriptor[]
    client: ClientDescriptor
    settings?: ClientSettings
    onClientChanged: () => void
}

export const ClientSettingsEditor = ({
    clients,
    client,
    settings,
    onClientChanged,
}: ClientSettingsEditorProps) => {
    const {
        form,
        save,
        onSubmit,
        discard,
        initialize,
        restartOpen,
        setRestartOpen,
        restart,
        restartBusy,
    } = useTimeOverrideForm({ client, settings })

    const { isDirty, isSubmitting } = form.formState
    const busy = save.isPending || isSubmitting
    const capabilities = client.capabilities.timeOverride

    const blocker = useBlocker({
        shouldBlockFn: () => isDirty || busy,
        withResolver: true,
        enableBeforeUnload: isDirty || busy,
    })

    return (
        <div className="space-y-6">
            <ClientSelector
                clients={clients}
                selectedClient={client.id}
                locked={isDirty}
                disabled={busy}
                onClientChanged={onClientChanged}
            />

            {save.isError && (
                <Alert variant="destructive">
                    <AlertTitle>Could not save settings</AlertTitle>
                    <AlertDescription>{save.error.message}</AlertDescription>
                </Alert>
            )}

            {!capabilities ? (
                <p className="text-sm text-muted-foreground">
                    This client has no configurable features yet.
                </p>
            ) : !settings ? (
                <div className="space-y-3 rounded-lg border p-4">
                    <p className="text-sm text-muted-foreground">
                        No settings have been created for this client.
                    </p>

                    <Button
                        type="button"
                        disabled={busy}
                        onClick={initialize}
                    >
                        {busy ? 'Creating…' : 'Create default settings'}
                    </Button>
                </div>
            ) : (
                <FormProvider {...form}>
                    <form
                        noValidate
                        onSubmit={onSubmit}
                        className="space-y-5"
                        aria-busy={busy}
                    >
                        <fieldset
                            disabled={busy}
                            className="min-w-0"
                        >
                            <TimeOverrideFields
                                capabilities={capabilities}
                                disabled={busy}
                            />
                        </fieldset>

                        <div className="flex items-center justify-between gap-4">
                            <p
                                role="status"
                                className="text-xs text-muted-foreground"
                            >
                                {busy
                                    ? 'Saving…'
                                    : isDirty
                                      ? 'Unsaved changes'
                                      : 'All changes saved'}
                            </p>

                            <div className="flex gap-2">
                                <Button
                                    type="button"
                                    variant="outline"
                                    disabled={!isDirty || busy}
                                    onClick={discard}
                                >
                                    Discard
                                </Button>

                                <Button
                                    type="submit"
                                    disabled={!isDirty || busy}
                                >
                                    Save changes
                                </Button>
                            </div>
                        </div>
                    </form>
                </FormProvider>
            )}

            <RestartEngineDialog
                open={restartOpen}
                onOpenChange={setRestartOpen}
                busy={restartBusy}
                onRestart={restart}
            />

            {blocker.status === 'blocked' && (
                <AlertDialog
                    open
                    onOpenChange={(open) => {
                        if (!open) blocker.reset()
                    }}
                >
                    <AlertDialogContent>
                        <AlertDialogHeader>
                            <AlertDialogTitle>
                                {busy ? 'Saving settings' : 'Discard changes?'}
                            </AlertDialogTitle>

                            <AlertDialogDescription>
                                {busy
                                    ? 'Wait until the current operation finishes.'
                                    : 'Your unsaved changes will be lost.'}
                            </AlertDialogDescription>
                        </AlertDialogHeader>

                        <AlertDialogFooter>
                            <AlertDialogCancel onClick={() => blocker.reset()}>
                                Stay
                            </AlertDialogCancel>

                            <AlertDialogAction
                                variant="destructive"
                                disabled={busy}
                                onClick={() => blocker.proceed()}
                            >
                                Discard and leave
                            </AlertDialogAction>
                        </AlertDialogFooter>
                    </AlertDialogContent>
                </AlertDialog>
            )}
        </div>
    )
}