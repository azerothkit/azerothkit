import { useForm } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import { useSaveClientSettings } from '@/core/settings'
import type { AppSettings, ClientDescriptor, ClientSettings } from '@/shared/api'
import {
    applyTimeOverrideForm,
    createTimeOverrideSchema,
    toTimeOverrideForm,
    type TimeOverrideFormValues,
} from '../forms/time-override'
import { useEffect, useState } from 'react'
import { useEngine } from '@/core/engine'

type UseTimeOverrideFormOptions = {
    client: ClientDescriptor
    settings?: ClientSettings
}

export const useTimeOverrideForm = ({ client, settings }: UseTimeOverrideFormOptions) => {
    const capabilities = client.capabilities.timeOverride
    const save = useSaveClientSettings()
    const engine = useEngine()
    const [restartOpen, setRestartOpen] = useState(false)

    const form = useForm<TimeOverrideFormValues>({
        resolver: zodResolver(createTimeOverrideSchema(capabilities)),
        defaultValues: toTimeOverrideForm(settings?.features.timeOverride),
    })

    const { reset } = form
    const { isDirty } = form.formState

    useEffect(() => {
        if (settings && !isDirty && !save.isPending) {
            reset(toTimeOverrideForm(settings.features.timeOverride))
        }
    }, [settings, isDirty, save.isPending, reset])

    const handleSaved = (savedSettings: AppSettings) => {
        const saved = savedSettings.settingsByClient[client.id]

        if (saved) {
            reset(toTimeOverrideForm(saved.features.timeOverride))
        }

        const status = engine.getStatus()

        if (status !== null && status !== 'stopped' && status !== 'stopping') {
            setRestartOpen(true)
        }
    }

    const onSubmit = form.handleSubmit((values) => {
        if (!settings || !capabilities) return

        save.mutate(
            {
                clientId: client.id,
                settings: applyTimeOverrideForm(values, settings, capabilities),
            },
            {
                onSuccess: handleSaved,
            },
        )
    })

    const discard = () => {
        reset(toTimeOverrideForm(settings?.features.timeOverride))
        save.reset()
    }

    const initialize = () => {
        save.mutate(
            {
                clientId: client.id,
                settings: {},
            },
            {
                onSuccess: handleSaved,
            },
        )
    }

    return {
        form,
        save,
        onSubmit,
        discard,
        initialize,
        restartOpen,
        setRestartOpen,
        restart: engine.restart,
        restartBusy: engine.isBusy,
    }
}