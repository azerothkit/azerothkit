type CommandResult<T> = { status: 'ok'; data: T } | { status: 'error'; error: string }

export async function unwrapCommand<T>(promise: Promise<CommandResult<T>>): Promise<T> {
    const result = await promise

    if (result.status === 'error') {
        throw new Error(result.error)
    }

    return result.data
}