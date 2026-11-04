import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
} from '@/shared/ui'

type RestartEngineDialogProps = {
    open: boolean
    onOpenChange: (open: boolean) => void
    busy: boolean
    onRestart: () => Promise<void>
}

export function RestartEngineDialog({
    open,
    onOpenChange,
    busy,
    onRestart,
}: RestartEngineDialogProps) {
    return (
        <AlertDialog
            open={open}
            onOpenChange={onOpenChange}
        >
            <AlertDialogContent>
                <AlertDialogHeader>
                    <AlertDialogTitle>Settings saved</AlertDialogTitle>

                    <AlertDialogDescription>
                        Restart the engine to apply the changes.
                    </AlertDialogDescription>
                </AlertDialogHeader>

                <AlertDialogFooter>
                    <AlertDialogCancel onClick={() => onOpenChange(false)}>Later</AlertDialogCancel>

                    <AlertDialogAction
                        disabled={busy}
                        onClick={() => {
                            onOpenChange(false)
                            void onRestart()
                        }}
                    >
                        Restart now
                    </AlertDialogAction>
                </AlertDialogFooter>
            </AlertDialogContent>
        </AlertDialog>
    )
}