import { z } from 'zod'
import type { ClientSettings, TimeOverrideCapabilities, TimeOverrideSettings } from '@/shared/api'
import { fromDurationMs, toDurationMs, type DurationUnit } from '@/shared/lib/time'
import { formatTimeOfDay, isTimeOfDay, parseTimeOfDay } from '@/shared/lib/time'

export const durationOptions: {
    value: DurationUnit
    label: string
}[] = [
    { value: 'seconds', label: 'Seconds' },
    { value: 'minutes', label: 'Minutes' },
    { value: 'hours', label: 'Hours' },
]

const durationPattern = /^(?:\d+(?:[.,]\d*)?|[.,]\d+)$/

export const parseDuration = (value: string): number => {
    const normalized = value.trim()

    if (!durationPattern.test(normalized)) return NaN

    return Number(normalized.replace(',', '.'))
}

export const createTimeOverrideSchema = (capabilities: TimeOverrideCapabilities | null) => {
    return z
        .object({
            enabled: z.boolean(),
            entryTime: z.string(),
            resetOnServerSync: z.boolean(),
            dayLength: z.string(),
            dayLengthUnit: z.enum(['seconds', 'minutes', 'hours']),
        })
        .superRefine((values, context) => {
            if (!values.enabled || !capabilities) return

            if (capabilities.entryTime && !isTimeOfDay(values.entryTime)) {
                context.addIssue({
                    code: 'custom',
                    path: ['entryTime'],
                    message: 'Enter a time between 00:00 and 23:59.',
                })
            }

            if (capabilities.dayLength) {
                const milliseconds = toDurationMs(
                    parseDuration(values.dayLength),
                    values.dayLengthUnit,
                )

                if (!Number.isSafeInteger(milliseconds) || milliseconds < 1) {
                    context.addIssue({
                        code: 'custom',
                        path: ['dayLength'],
                        message:
                            'Enter a positive duration of at least 1 millisecond within the supported numeric range.',
                    })
                }
            }
        })
}

export type TimeOverrideFormValues = z.infer<ReturnType<typeof createTimeOverrideSchema>>

export const toTimeOverrideForm = (settings?: TimeOverrideSettings): TimeOverrideFormValues => {
    if (!settings) {
        return {
            enabled: false,
            entryTime: '',
            resetOnServerSync: false,
            dayLength: '',
            dayLengthUnit: 'seconds',
        }
    }

    return {
        enabled: settings.enabled,
        entryTime: formatTimeOfDay(settings.entryTimeMinutes),
        resetOnServerSync: settings.resetOnServerSync,
        dayLength: String(fromDurationMs(settings.dayLengthMs, settings.dayLengthUnit)),
        dayLengthUnit: settings.dayLengthUnit,
    }
}

export const applyTimeOverrideForm = (
    values: TimeOverrideFormValues,
    current: ClientSettings,
    capabilities: TimeOverrideCapabilities,
): ClientSettings => {
    const timeOverride = {
        ...current.features.timeOverride,
        enabled: values.enabled,
    }

    if (values.enabled) {
        if (capabilities.entryTime) {
            timeOverride.entryTimeMinutes = parseTimeOfDay(values.entryTime)
        }

        if (capabilities.resetOnServerSync) {
            timeOverride.resetOnServerSync = values.resetOnServerSync
        }

        if (capabilities.dayLength) {
            timeOverride.dayLengthMs = toDurationMs(
                parseDuration(values.dayLength),
                values.dayLengthUnit,
            )
            timeOverride.dayLengthUnit = values.dayLengthUnit
        }
    }

    return {
        ...current,
        features: {
            ...current.features,
            timeOverride,
        },
    }
}