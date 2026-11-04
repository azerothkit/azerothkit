import { toast } from '@/shared/ui'

export type Tags = Readonly<Record<string, string>> & {
    scope?: string
}

export const report = (error: unknown, tags?: Tags) => {
    console.error('[error]', tags?.scope ?? 'unknown', error, tags)

    const message =
        error instanceof Error
            ? error.message
            : typeof error === 'string'
              ? error
              : 'An unexpected error occurred.'

    toast.add({
        type: 'error',
        title: 'Operation failed',
        description: message || 'An unexpected error occurred.',
        timeout: 6000,
    })
}