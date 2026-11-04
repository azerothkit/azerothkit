import { commands, listenEngineStatus, unwrapCommand } from '@/shared/api'

export const engineApi = {
    getSnapshot: () => unwrapCommand(commands.getEngineSnapshot()),
    start: () => unwrapCommand(commands.startEngine()),
    stop: () => unwrapCommand(commands.stopEngine()),
    restart: () => unwrapCommand(commands.restartEngine()),
    subscribe: listenEngineStatus,
}