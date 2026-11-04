import { commands, unwrapCommand } from '@/shared/api'

export const installApi = {
    getTlsStatus: () => {
        return unwrapCommand(commands.getTlsCertificateStatus())
    },

    installTlsCertificate: () => {
        return unwrapCommand(commands.installTlsCertificate())
    },

    getCaptureStatus: () => {
        return unwrapCommand(commands.getCaptureSetupStatus())
    },

    openCaptureSettings: () => {
        return unwrapCommand(commands.openCaptureSettings())
    },
}