import { commands, unwrapCommand, type ClientId, type ClientSettingsInput } from '@/shared/api'

export type SaveClientSettingsInput = {
    clientId: ClientId
    settings: ClientSettingsInput
}

export const settingsApi = {
    getSnapshot: () => {
        return unwrapCommand(commands.getSettingsSnapshot())
    },

    setSelectedClient: (clientId: ClientId) => {
        return unwrapCommand(commands.setSelectedClient(clientId))
    },

    saveClientSettings: ({ clientId, settings }: SaveClientSettingsInput) => {
        return unwrapCommand(commands.saveClientSettings(clientId, settings))
    },
}