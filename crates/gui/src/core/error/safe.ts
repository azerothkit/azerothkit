import type { Tags } from './report'
import { report } from './report'

type Result = void | Promise<void>
type Callback<Args extends unknown[]> = (...args: Args) => Result
type OnError = () => void

export const safe = <Args extends unknown[]>(
    fn: Callback<Args>,
    args: Args,
    props?: {
        onError?: OnError
        tags?: Tags
    },
): Result => {
    const { onError, tags } = props ?? {}

    const handleError = (e: unknown) => {
        report(e, tags)
        onError?.()
    }

    try {
        const result = fn(...args)

        if (result instanceof Promise) {
            return result.catch(handleError)
        }
    } catch (e) {
        handleError(e)
    }
}

export const lazySafe = <Args extends unknown[]>(
    fn: Callback<Args>,
    props?: {
        onError?: OnError
        tags?: Tags
    },
) => {
    return (...args: Args) => safe(fn, args, { ...props })
}