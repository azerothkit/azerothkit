import { useId, useState } from 'react'
import { useSetSelectedClient } from '@/core/settings'
import type { ClientDescriptor, ClientId } from '@/shared/api'
import { Field, FieldDescription, FieldError, FieldLabel } from '@/shared/ui'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/shared/ui'
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
import { ClientLabel } from '@/shared/ui'

type ClientSelectorProps = {
    clients: ClientDescriptor[]
    selectedClient: ClientId
    locked: boolean
    disabled: boolean
}

export const ClientSelector = ({
    clients,
    selectedClient,
    locked: hasUnsavedChanges,
    disabled,
}: ClientSelectorProps) => {
    const id = useId()
    const selection = useSetSelectedClient()

    const [pendingClient, setPendingClient] = useState<ClientId | null>(null)

    const items = clients.map((client) => ({
        value: client.id,
        textLabel: client.label,
        label: (
            <ClientLabel
                clientId={client.id}
                label={client.label}
            />
        ),
    }))

    const requestClientChange = (clientId: ClientId) => {
        if (disabled || selection.isPending || clientId === selectedClient) {
            return
        }

        if (hasUnsavedChanges) {
            setPendingClient(clientId)
            return
        }

        selection.mutate(clientId)
    }

    const confirmClientChange = () => {
        if (pendingClient === null) return

        const clientId = pendingClient

        setPendingClient(null)
        selection.mutate(clientId)
    }

    return (
        <>
            <Field>
                <FieldLabel htmlFor={id}>Game client</FieldLabel>

                <Select<ClientId>
                    items={items}
                    value={selectedClient}
                    disabled={disabled || selection.isPending}
                    onValueChange={(value) => {
                        if (typeof value === 'string') {
                            requestClientChange(value)
                        }
                    }}
                >
                    <SelectTrigger
                        id={id}
                        className="w-full"
                    >
                        <SelectValue />
                    </SelectTrigger>

                    <SelectContent>
                        {items.map((item) => (
                            <SelectItem
                                key={item.value}
                                value={item.value}
                                label={item.textLabel}
                                className="pr-7"
                            >
                                {item.label}
                            </SelectItem>
                        ))}
                    </SelectContent>
                </Select>

                <FieldDescription>Settings are stored separately for each client.</FieldDescription>

                <FieldError>{selection.error?.message}</FieldError>
            </Field>

            <AlertDialog
                open={pendingClient !== null}
                onOpenChange={(open) => {
                    if (!open) {
                        setPendingClient(null)
                    }
                }}
            >
                <AlertDialogContent>
                    <AlertDialogHeader>
                        <AlertDialogTitle>Continue without saving?</AlertDialogTitle>

                        <AlertDialogDescription>
                            Switching clients will discard unsaved changes to the current form.
                        </AlertDialogDescription>
                    </AlertDialogHeader>

                    <AlertDialogFooter>
                        <AlertDialogCancel onClick={() => setPendingClient(null)}>
                            Cancel
                        </AlertDialogCancel>

                        <AlertDialogAction
                            variant="destructive"
                            onClick={confirmClientChange}
                        >
                            Continue without saving
                        </AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>
        </>
    )
}