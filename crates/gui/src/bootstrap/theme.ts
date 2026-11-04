export function initSystemTheme(): () => void {
    const media = window.matchMedia('(prefers-color-scheme: dark)')
    const root = document.documentElement

    const applyTheme = () => {
        const isDark = media.matches

        root.classList.toggle('dark', isDark)
        root.classList.toggle('light', !isDark)
        root.style.colorScheme = isDark ? 'dark' : 'light'
    }

    applyTheme()
    media.addEventListener('change', applyTheme)

    return () => {
        media.removeEventListener('change', applyTheme)
    }
}