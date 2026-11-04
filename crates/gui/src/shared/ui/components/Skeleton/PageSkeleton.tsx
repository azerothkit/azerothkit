import { Skeleton } from './skeleton'

export const PageSkeleton = () => {
    return (
        <>
            <div className="flex w-full flex-col gap-4">
                <div className={'flex w-full flex-col gap-2'}>
                    <Skeleton className="h-4 w-2/3" />
                    <Skeleton className="h-4 w-1/2" />
                </div>
                <Skeleton className="aspect-video w-full" />
            </div>
        </>
    )
}