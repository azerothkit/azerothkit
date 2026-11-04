import type { ClientId } from '@/shared/api'
import foreverIcon from '@/assets/f-silver.svg'
import retailIcon from '@/assets/r-gold.svg'

const clientIcons = {
    forever: foreverIcon,
    retail: retailIcon,
} satisfies Record<ClientId, string>

type ClientLabelProps = {
    clientId: ClientId
    label: string
}

export const ClientLabel = ({ clientId, label }: ClientLabelProps) => (
    <span className="flex min-w-0 items-center gap-2">
        <img
            src={clientIcons[clientId]}
            alt=""
            aria-hidden="true"
            className="pointer-events-none size-4 shrink-0 object-contain"
        />
        <span className="truncate">{label}</span>
    </span>
)