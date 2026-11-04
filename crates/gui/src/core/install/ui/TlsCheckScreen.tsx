import { useEffect, useState } from 'react'
import { InstallScreen } from './InstallScreen'

const SHOW_DELAY_MS = 300

type TlsCheckScreenProps = {
    title?: string
    description?: string
}

export function TlsCheckScreen({
    title = 'Checking TLS certificate',
    description = 'Please wait while AzerothKit checks the certificate.',
}: TlsCheckScreenProps) {
    // Existing visible state, timer and cleanup remain here.
    const [visible, setVisible] = useState(false)

    useEffect(() => {
        const timer = window.setTimeout(() => {
            setVisible(true)
        }, SHOW_DELAY_MS)

        return () => {
            window.clearTimeout(timer)
        }
    }, [])

    if (!visible) {
        return (
            <div
                className="min-h-0 flex-1"
                role="status"
                aria-label={title}
                aria-busy="true"
            />
        )
    }

    return (
        <InstallScreen
            title={title}
            description={description}
        />
    )
}