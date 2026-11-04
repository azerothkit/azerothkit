import React from 'react'
import ReactDOM from 'react-dom/client'
// oxlint-disable-next-line import/no-unassigned-import
import './index.css'
import { createRouter, RouterProvider, createHashHistory } from '@tanstack/react-router'
import { routeTree } from './routeTree.gen'
import { TanStackRouterDevtools } from '@tanstack/react-router-devtools'
import { initSystemTheme } from '@/bootstrap/theme'

const disposeSystemTheme = initSystemTheme()

if (import.meta.hot) {
    import.meta.hot.dispose(disposeSystemTheme)
}

export const router = createRouter({
    routeTree,
    history: createHashHistory(),
    scrollRestoration: true,
    scrollRestorationBehavior: 'instant',
    getScrollRestorationKey: (location) => location.pathname,
    scrollToTopSelectors: ['#main-scroll-area'],
})

declare module '@tanstack/react-router' {
    interface Register {
        router: typeof router
    }
}

// oxlint-disable-next-line typescript/no-unsafe-type-assertion
ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
    <React.StrictMode>
        <RouterProvider router={router} />
        <TanStackRouterDevtools router={router} />
    </React.StrictMode>,
)