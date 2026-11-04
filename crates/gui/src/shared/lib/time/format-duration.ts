import { fromDurationMs, type DurationUnit } from './duration'

const intlDurationUnits = {
    seconds: 'second',
    minutes: 'minute',
    hours: 'hour',
} as const satisfies Record<DurationUnit, string>

export const formatDuration = (
    milliseconds: number,
    unit: DurationUnit,
    locale: string,
): string => {
    return new Intl.NumberFormat(locale, {
        style: 'unit',
        unit: intlDurationUnits[unit],
        unitDisplay: 'short',
        maximumFractionDigits: 3,
    }).format(fromDurationMs(milliseconds, unit))
}