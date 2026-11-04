import { useId } from 'react'
import { Controller, useFormContext, useWatch } from 'react-hook-form'
import type { TimeOverrideCapabilities } from '@/shared/api'
import { Card, CardAction, CardContent, CardDescription, CardHeader, CardTitle } from '@/shared/ui'
import {
    Field,
    FieldContent,
    FieldDescription,
    FieldError,
    FieldGroup,
    FieldLabel,
} from '@/shared/ui'
import { Input } from '@/shared/ui'
import { Switch } from '@/shared/ui'
import type { TimeOverrideFormValues } from '../forms/time-override'
import { DayLengthField } from './DayLengthField'
import { Checkbox } from '@/shared/ui'
import { Clock } from 'lucide-react'

type TimeOverrideFieldsProps = {
    capabilities: TimeOverrideCapabilities
    disabled: boolean
}

export const TimeOverrideFields = ({ capabilities, disabled }: TimeOverrideFieldsProps) => {
    const id = useId()
    const {
        control,
        register,
        formState: { errors },
    } = useFormContext<TimeOverrideFormValues>()

    const enabled = useWatch({
        control,
        name: 'enabled',
    })

    return (
        <Card>
            <CardHeader>
                <CardTitle
                    id={`${id}-title`}
                    className="flex items-center gap-1.5"
                >
                    <Clock
                        className="size-4 shrink-0"
                        strokeWidth={1.75}
                        aria-hidden="true"
                    />
                    Time override
                </CardTitle>

                <CardDescription>Customize the in-game clock and day cycle.</CardDescription>

                <CardAction>
                    <Controller
                        control={control}
                        name="enabled"
                        render={({ field }) => (
                            <Switch
                                ref={field.ref}
                                checked={field.value}
                                onCheckedChange={field.onChange}
                                onBlur={field.onBlur}
                                disabled={disabled}
                                aria-labelledby={`${id}-title`}
                            />
                        )}
                    />
                </CardAction>
            </CardHeader>

            {enabled && (
                <CardContent>
                    <FieldGroup>
                        {capabilities.entryTime && (
                            <Field data-invalid={Boolean(errors.entryTime)}>
                                <FieldLabel htmlFor={`${id}-entry`}>Time on world entry</FieldLabel>

                                <Input
                                    {...register('entryTime')}
                                    id={`${id}-entry`}
                                    type="time"
                                    step={60}
                                    aria-invalid={Boolean(errors.entryTime)}
                                    aria-describedby={`${id}-entry-description ${id}-entry-error`}
                                    className="w-36 tabular-nums"
                                />

                                <FieldError
                                    id={`${id}-entry-error`}
                                    errors={[errors.entryTime]}
                                />
                            </Field>
                        )}

                        {capabilities.resetOnServerSync && (
                            <Field orientation="horizontal">
                                <Controller
                                    control={control}
                                    name="resetOnServerSync"
                                    render={({ field }) => (
                                        <Checkbox
                                            ref={field.ref}
                                            id={`${id}-reset`}
                                            checked={field.value}
                                            onCheckedChange={field.onChange}
                                            onBlur={field.onBlur}
                                            disabled={disabled}
                                            aria-describedby={`${id}-reset-description`}
                                        />
                                    )}
                                />

                                <FieldContent>
                                    <FieldLabel htmlFor={`${id}-reset`}>
                                        Reset on server sync
                                    </FieldLabel>

                                    <FieldDescription id={`${id}-reset-description`}>
                                        Reset to the entry time whenever the server updates the
                                        clock.
                                    </FieldDescription>
                                </FieldContent>
                            </Field>
                        )}

                        {capabilities.dayLength && <DayLengthField disabled={disabled} />}
                    </FieldGroup>
                </CardContent>
            )}
        </Card>
    )
}