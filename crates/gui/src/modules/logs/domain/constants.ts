export const LOG_LEVELS = ['error', 'warn', 'info', 'debug', 'trace'] as const

export const LOG_LEVEL_ITEMS = [
    { value: 'all' as const, label: 'All levels' },
    ...LOG_LEVELS.map((level) => ({
        value: level,
        label: level.toUpperCase(),
    })),
]