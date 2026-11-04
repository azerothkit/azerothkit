import { commands, type ClientId, type ClientSettingsInput } from '@/shared/api'

type CommandResult<T> = { status: 'ok'; data: T } | { status: 'error'; error: string }

async function unwrap<T>(promise: Promise<CommandResult<T>>): Promise<T> {
    const result = await promise

    if (result.status === 'error') {
        throw new Error(result.error)
    }

    return result.data
}

export type SaveClientSettingsInput = {
    clientId: ClientId
    settings: ClientSettingsInput
}

export const settingsApi = {
    getSnapshot: async () => {
        return unwrap(commands.getSettingsSnapshot())
    },

    setSelectedClient: (clientId: ClientId) => {
        return unwrap(commands.setSelectedClient(clientId))
    },

    saveClientSettings: ({ clientId, settings }: SaveClientSettingsInput) => {
        return unwrap(commands.saveClientSettings(clientId, settings))
    },
}