import { useId } from 'react'
import { Controller, useFormContext } from 'react-hook-form'
import type { DurationUnit } from '@/shared/api'
import { Field, FieldDescription, FieldError, FieldLabel } from '@/shared/ui'
import { Input } from '@/shared/ui'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/shared/ui'
import { durationOptions, parseDuration, type TimeOverrideFormValues } from '../forms/time-override'
import { durationUnitMs } from '@/shared/lib/time'

type DayLengthFieldProps = {
    disabled: boolean
}

export const DayLengthField = ({ disabled }: DayLengthFieldProps) => {
    const id = useId()
    const {
        control,
        register,
        getValues,
        setValue,
        formState: { errors },
    } = useFormContext<TimeOverrideFormValues>()

    return (
        <Field data-invalid={Boolean(errors.dayLength)}>
            <FieldLabel htmlFor={id}>Day length</FieldLabel>

            <div className="flex gap-2">
                <Input
                    {...register('dayLength')}
                    id={id}
                    type="text"
                    inputMode="decimal"
                    autoComplete="off"
                    aria-invalid={Boolean(errors.dayLength)}
                    aria-describedby={`${id}-description ${id}-error`}
                    className="flex-1 tabular-nums"
                />

                <Controller
                    control={control}
                    name="dayLengthUnit"
                    render={({ field }) => (
                        <Select<DurationUnit>
                            items={durationOptions}
                            value={field.value}
                            disabled={disabled}
                            onValueChange={(unit) => {
                                if (typeof unit !== 'string') return

                                const current = parseDuration(getValues('dayLength'))
                                const previousUnit = field.value

                                field.onChange(unit)

                                if (Number.isFinite(current) && current > 0) {
                                    setValue(
                                        'dayLength',
                                        String(
                                            (current * durationUnitMs[previousUnit]) /
                                                durationUnitMs[unit],
                                        ),
                                        {
                                            shouldDirty: true,
                                            shouldValidate: true,
                                        },
                                    )
                                }
                            }}
                        >
                            <SelectTrigger
                                ref={field.ref}
                                onBlur={field.onBlur}
                                aria-label="Day length unit"
                                className="w-28"
                            >
                                <SelectValue />
                            </SelectTrigger>

                            <SelectContent>
                                {durationOptions.map((option) => (
                                    <SelectItem
                                        key={option.value}
                                        value={option.value}
                                    >
                                        {option.label}
                                    </SelectItem>
                                ))}
                            </SelectContent>
                        </Select>
                    )}
                />
            </div>

            <FieldDescription id={`${id}-description`}>
                Real-world duration of a full 24-hour in-game day.
            </FieldDescription>

            <FieldError
                id={`${id}-error`}
                errors={[errors.dayLength]}
            />
        </Field>
    )
}