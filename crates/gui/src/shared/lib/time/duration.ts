export const durationUnitMs = {
    seconds: 1_000,
    minutes: 60_000,
    hours: 3_600_000,
} as const

export type DurationUnit = keyof typeof durationUnitMs

export const toDurationMs = (value: number, unit: DurationUnit): number => {
    return Math.round(value * durationUnitMs[unit])
}

export const fromDurationMs = (milliseconds: number, unit: DurationUnit): number => {
    return milliseconds / durationUnitMs[unit]
}