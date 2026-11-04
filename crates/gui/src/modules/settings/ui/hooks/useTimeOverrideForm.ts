import { useEffect } from 'react'
import { useForm } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import { useSaveClientSettings } from '@/core/settings'
import type { ClientDescriptor, ClientSettings } from '@/shared/api'
import {
    applyTimeOverrideForm,
    createTimeOverrideSchema,
    toTimeOverrideForm,
    type TimeOverrideFormValues,
} from '../forms/time-override'

type UseTimeOverrideFormOptions = {
    client: ClientDescriptor
    settings?: ClientSettings
}

export const useTimeOverrideForm = ({ client, settings }: UseTimeOverrideFormOptions) => {
    const capabilities = client.capabilities.timeOverride
    const save = useSaveClientSettings()

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

    const onSubmit = form.handleSubmit((values) => {
        if (!settings || !capabilities) return

        save.mutate(
            {
                clientId: client.id,
                settings: applyTimeOverrideForm(values, settings, capabilities),
            },
            {
                onSuccess: (savedSettings) => {
                    const saved = savedSettings.settingsByClient[client.id]

                    if (saved) {
                        reset(toTimeOverrideForm(saved.features.timeOverride))
                    }
                },
            },
        )
    })

    const discard = () => {
        reset(toTimeOverrideForm(settings?.features.timeOverride))
        save.reset()
    }

    const initialize = () => {
        save.mutate({
            clientId: client.id,
            settings: {},
        })
    }

    return {
        form,
        save,
        onSubmit,
        discard,
        initialize,
    }
}