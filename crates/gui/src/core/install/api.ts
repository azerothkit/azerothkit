import { commands, unwrapCommand } from '@/shared/api'

export const installApi = {
    getTlsStatus: async () => {
        return unwrapCommand(commands.getTlsCertificateStatus())
    },

    installTlsCertificate: () => {
        return unwrapCommand(commands.installTlsCertificate())
    },
}