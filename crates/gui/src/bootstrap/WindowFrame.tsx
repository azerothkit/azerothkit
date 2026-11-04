import type { PropsWithChildren } from 'react'

export function WindowFrame({ children }: PropsWithChildren) {
    return (
        <>
            <div
                data-tauri-drag-region
                className="fixed inset-x-0 top-0 z-50 h-8"
            />

            <div className="flex h-dvh flex-col overflow-hidden bg-background pt-3.5 text-foreground">
                {children}
            </div>
        </>
    )
}