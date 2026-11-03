export type Tags = Readonly<Record<string, string>> & { scope?: string }

export const report = (error: unknown, tags?: Tags) => {
  console.error('[error]', tags?.scope ?? 'unknow', error)
}
