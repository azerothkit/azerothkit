import { forwardRef, type ComponentPropsWithoutRef } from 'react'
import { createLink } from '@tanstack/react-router'

export const TextLink = forwardRef<HTMLAnchorElement, ComponentPropsWithoutRef<'a'>>(
    function TextLink({ className = '', ...props }, ref) {
        return (
            <a
                {...props}
                ref={ref}
                className={`shrink-0 text-xs text-muted-foreground underline-offset-4 hover:text-foreground hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${className}`}
            />
        )
    },
)

export const RouterTextLink = createLink(TextLink)