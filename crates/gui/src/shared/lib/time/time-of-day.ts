const timeOfDayPattern = /^([01]\d|2[0-3]):[0-5]\d$/

export const isTimeOfDay = (value: string): boolean => {
    return timeOfDayPattern.test(value)
}

export const formatTimeOfDay = (minutesSinceMidnight: number): string => {
    const hours = Math.floor(minutesSinceMidnight / 60)
    const minutes = minutesSinceMidnight % 60

    return `${String(hours).padStart(2, '0')}:${String(minutes).padStart(2, '0')}`
}

export const parseTimeOfDay = (value: string): number => {
    if (!isTimeOfDay(value)) return NaN

    const [hours, minutes] = value.split(':').map(Number)

    return hours * 60 + minutes
}