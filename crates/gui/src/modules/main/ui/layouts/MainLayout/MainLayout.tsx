import { Link, Outlet } from '@tanstack/react-router'

const navigation = [
    { to: '/', label: 'Overview' },
    { to: '/settings', label: 'Settings' },
    { to: '/logs', label: 'Logs' },
] as const

export const MainLayout = () => {
    return (
        <div className="flex h-dvh flex-col overflow-hidden bg-background text-foreground">
            <header className="shrink-0 border-b">
                <div className="mx-auto w-full max-w-xl px-6 pt-5 pb-4">
                    <div className="mb-5 flex items-center justify-between gap-4">
                        <span className="text-sm font-semibold tracking-tight">Azerothkit</span>

                        <span className="text-xs text-muted-foreground">UI preview</span>
                    </div>

                    <nav
                        aria-label="Main navigation"
                        className="grid grid-cols-3 gap-1 rounded-lg bg-muted p-1"
                    >
                        {navigation.map(({ to, label }) => (
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
                                className="rounded-md px-3 py-2 text-center text-xs font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                            >
                                {label}
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

                    <Link
                        to="/logs"
                        className="shrink-0 underline-offset-4 hover:text-foreground hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                    >
                        View logs
                    </Link>
                </div>
            </footer>
        </div>
    )
}