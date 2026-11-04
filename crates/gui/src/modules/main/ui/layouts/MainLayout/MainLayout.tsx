import { Link, Outlet } from '@tanstack/react-router'

import { Power, SlidersHorizontal, Terminal } from 'lucide-react'
import { RouterTextLink, TextLink } from '@/shared/ui'
import { GITHUB_REPOSITORY_URL } from '@/shared/config/links'

const navigation = [
    { to: '/', label: 'Overview', icon: Power },
    { to: '/settings', label: 'Settings', icon: SlidersHorizontal },
    { to: '/logs', label: 'Logs', icon: Terminal },
] as const

export const MainLayout = () => {
    return (
        <div className="flex h-dvh flex-col overflow-hidden bg-background text-foreground">
            <header className="shrink-0 border-b">
                <div className="mx-auto w-full max-w-xl px-6 pt-5 pb-4">
                    <div className="mb-5 flex items-center justify-between gap-4">
                        <span className="text-sm font-semibold tracking-tight">Azerothkit</span>

                        <TextLink
                            href={GITHUB_REPOSITORY_URL}
                            target="_blank"
                            rel="noopener noreferrer"
                        >
                            Unofficial app
                        </TextLink>
                    </div>

                    <nav
                        aria-label="Main navigation"
                        className="grid grid-cols-3 gap-1 rounded-lg bg-muted p-1"
                    >
                        {navigation.map(({ to, label, icon: Icon }) => (
                            <Link
                                key={to}
                                to={to}
                                activeOptions={{ exact: to === '/' }}
                                activeProps={{
                                    className: 'bg-background text-foreground shadow-sm',
                                }}
                                inactiveProps={{
                                    className: 'text-muted-foreground hover:text-foreground',
                                }}
                                className="flex items-center justify-center gap-2 rounded-md px-3 py-2 text-xs font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                            >
                                <Icon
                                    className="size-4 shrink-0"
                                    strokeWidth={1.75}
                                    aria-hidden="true"
                                />
                                <span>{label}</span>
                            </Link>
                        ))}
                    </nav>
                </div>
            </header>

            <main className="min-h-0 flex-1 overflow-y-auto">
                <div className="mx-auto w-full max-w-xl px-6 py-6">
                    <Outlet />
                </div>
            </main>

            <footer className="shrink-0 border-t">
                <div className="mx-auto flex w-full max-w-xl items-center justify-between gap-4 px-6 py-3 text-xs text-muted-foreground">
                    <span>Preview · Rust service not connected</span>

                    <RouterTextLink to="/logs">View logs</RouterTextLink>
                </div>
            </footer>
        </div>
    )
}