/**
 * Next.js collects telemetry on errors thrown in the app in some cases (i.e., in error rating)
 * and we use this helper to stitch the error code to the digest.
 */
export const appendErrorCodeToDigest = (
  thrownValue: unknown,
  originalDigest: string
): string => {
  if (
    typeof thrownValue === 'object' &&
    thrownValue !== null &&
    '__NEXT_ERROR_CODE' in thrownValue
  ) {
    return `${originalDigest};${thrownValue.__NEXT_ERROR_CODE}`
  }
  return originalDigest
}

export const extractNextErrorCode = (error: unknown): string | undefined => {
  if (
    typeof error === 'object' &&
    error !== null &&
    '__NEXT_ERROR_CODE' in error &&
    typeof (error as any).__NEXT_ERROR_CODE === 'string'
  ) {
    return (error as any).__NEXT_ERROR_CODE
  }

  if (
    typeof error === 'object' &&
    error !== null &&
    'digest' in error &&
    typeof error.digest === 'string'
  ) {
    const segments = error.digest.split(';')
    const errorCode = segments.find((segment) => segment.startsWith('E'))
    return errorCode
  }

  return undefined
}

export const removeNextErrorCode = (error: unknown): void => {
  if (
    typeof error === 'object' &&
    error !== null &&
    '__NEXT_ERROR_CODE' in error
  ) {
    delete (error as any).__NEXT_ERROR_CODE
  }
}
