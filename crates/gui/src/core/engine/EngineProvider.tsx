import {
    createContext,
    useCallback,
    useContext,
    useEffect,
    useMemo,
    useRef,
    useState,
    type PropsWithChildren,
} from 'react'
import { report } from '@/core/error'
import type { EngineSnapshot, EngineStatus } from '@/shared/api'
import { engineApi } from './api'

type EngineConnection = 'connecting' | 'ready' | 'error'

type EngineContextValue = {
    status: EngineStatus | null
    statusLabel: string
    connection: EngineConnection
    isActive: boolean
    isBusy: boolean
    toggle: () => Promise<void>
    reconnect: () => void
    restart: () => Promise<void>
    getStatus: () => EngineStatus | null
}

const statusLabels: Record<EngineStatus, string> = {
    stopped: 'Disabled',
    starting: 'Starting...',
    waiting_for_game: 'Waiting for game',
    waiting_for_keys: 'Waiting for keys',
    running: 'Working',
    stopping: 'Stopping...',
}

const EngineContext = createContext<EngineContextValue | null>(null)

export function EngineProvider({ children }: PropsWithChildren) {
    const [snapshot, setSnapshot] = useState<EngineSnapshot | null>(null)
    const [connection, setConnection] = useState<EngineConnection>('connecting')
    const [commandPending, setCommandPending] = useState(false)
    const [attempt, setAttempt] = useState(0)

    const snapshotRef = useRef<EngineSnapshot | null>(null)
    const connectionRef = useRef<EngineConnection>('connecting')
    const commandPendingRef = useRef(false)
    const attemptRef = useRef(0)

    const updateConnection = useCallback((next: EngineConnection) => {
        connectionRef.current = next
        setConnection(next)
    }, [])

    const applySnapshot = useCallback((next: EngineSnapshot) => {
        const current = snapshotRef.current

        if (current && next.revision <= current.revision) {
            return
        }

        snapshotRef.current = next
        setSnapshot(next)
    }, [])

    useEffect(() => {
        let disposed = false
        let unsubscribe: (() => void) | undefined

        const isCurrent = () => !disposed && attempt === attemptRef.current

        async function connect() {
            try {
                const stopListening = await engineApi.subscribe((next) => {
                    if (isCurrent()) {
                        applySnapshot(next)
                    }
                })

                if (!isCurrent()) {
                    stopListening()
                    return
                }

                unsubscribe = stopListening

                const initialSnapshot = await engineApi.getSnapshot()

                if (!isCurrent()) {
                    return
                }

                applySnapshot(initialSnapshot)
                updateConnection('ready')
            } catch (error) {
                if (!isCurrent()) {
                    return
                }

                unsubscribe?.()
                unsubscribe = undefined

                updateConnection('error')
                report(error, { scope: 'engine.connect' })
            }
        }

        void connect()

        return () => {
            disposed = true
            unsubscribe?.()
        }
    }, [attempt, applySnapshot, updateConnection])

    const reconnect = useCallback(() => {
        if (commandPendingRef.current || connectionRef.current === 'connecting') {
            return
        }

        attemptRef.current += 1
        updateConnection('connecting')
        setAttempt(attemptRef.current)
    }, [updateConnection])

    const runCommand = useCallback(
        async (operation: 'start' | 'stop' | 'restart') => {
            const current = snapshotRef.current

            if (
                connectionRef.current !== 'ready' ||
                commandPendingRef.current ||
                !current ||
                current.status === 'starting' ||
                current.status === 'stopping'
            ) {
                return
            }

            commandPendingRef.current = true
            setCommandPending(true)

            try {
                const next = await engineApi[operation]()
                applySnapshot(next)
            } catch (error) {
                report(error, { scope: `engine.${operation}` })

                try {
                    const actualSnapshot = await engineApi.getSnapshot()
                    applySnapshot(actualSnapshot)
                } catch (syncError) {
                    console.error('[engine.resync]', syncError)
                    updateConnection('error')
                }
            } finally {
                commandPendingRef.current = false
                setCommandPending(false)
            }
        },
        [applySnapshot, updateConnection],
    )

    const toggle = useCallback(async () => {
        const current = snapshotRef.current

        if (!current) {
            return
        }

        await runCommand(current.status === 'stopped' ? 'start' : 'stop')
    }, [runCommand])

    const restart = useCallback(() => {
        return runCommand('restart')
    }, [runCommand])

    const getStatus = useCallback(() => {
        return snapshotRef.current?.status ?? null
    }, [])

    const status = snapshot?.status ?? null
    const isActive = status !== null && status !== 'stopped'
    const isBusy =
        connection !== 'ready' || commandPending || status === 'starting' || status === 'stopping'

    const statusLabel =
        connection === 'error'
            ? 'Service unavailable'
            : connection === 'connecting' || status === null
              ? 'Connecting...'
              : statusLabels[status]

    const value = useMemo<EngineContextValue>(
        () => ({
            status,
            statusLabel,
            connection,
            isActive,
            isBusy,
            toggle,
            reconnect,
            restart,
            getStatus,
        }),
        [status, statusLabel, connection, isActive, isBusy, toggle, reconnect, restart, getStatus],
    )

    return <EngineContext.Provider value={value}>{children}</EngineContext.Provider>
}

export function useEngine() {
    const context = useContext(EngineContext)

    if (!context) {
        throw new Error('useEngine must be used within EngineProvider')
    }

    return context
}