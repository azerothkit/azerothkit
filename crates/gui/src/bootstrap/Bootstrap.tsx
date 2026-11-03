import type { PropsWithChildren } from 'react'
import { useState } from 'react'
import { useEffect } from 'react'
import { QueryClientProvider } from '@tanstack/react-query'
import { queryClient } from '@/core/query'

type BootstrapState =
    | { status: 'pending' }
    | { status: 'ready' }
    | { status: 'fail'; eventId: string }

export function Bootstrap({ children }: PropsWithChildren) {
    const [state, setState] = useState<BootstrapState>({ status: 'ready' })

    useEffect(() => {
        // initApp()
        //     .then(() => {
        //         setState({ status: 'ready' })
        //     })
        //     .catch((e) => {
        //         const eventId = report(e, { scope: 'init' })
        //         setState({ status: 'fail', eventId })
        //     })
        //     .finally(() => {
        //     })
    }, [])

    if (state.status === 'fail') {
        // return (
        //     <FailScreen
        //         eventId={state.eventId}
        //         variant={'startup'}
        //     />
        // )
    }

    if (state.status === 'pending') return null

    return (
        <>
            <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
        </>
    )
}