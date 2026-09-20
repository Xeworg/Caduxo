import type { FormattersInitializer } from 'typesafe-i18n'
import type { Locales, Formatters } from './i18n-types'

// The current `Locales` are bare ISO-639 codes ("en", "es"); Intl needs a
// region to pick stable defaults. We keep `es-MX` to preserve the previous
// locale-agnostic behaviour used by the report export and resolve-quantity
// dialog (and the PDF backend), and `en-US` for the base locale.
const LOCALE_TAG: Record<Locales, string> = {
  en: 'en-US',
  es: 'es-MX',
}

const shortDateFmt = (locale: Locales) =>
  new Intl.DateTimeFormat(LOCALE_TAG[locale], {
    day: '2-digit',
    month: '2-digit',
    year: 'numeric',
  })

const dateTimeFmt = (locale: Locales) =>
  new Intl.DateTimeFormat(LOCALE_TAG[locale], {
    day: '2-digit',
    month: '2-digit',
    year: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  })

// ISO date-only strings must be parsed as local calendar dates, not as UTC
// midnight (which is what `new Date('YYYY-MM-DD')` does per ES spec and would
// shift the visible day for users west of UTC). We cap the fraction digits
// well above the previous 3 to avoid visible rounding for normal decimal
// quantities while still keeping trailing-zero noise bounded.
const ISO_DATE_ONLY = /^\d{4}-\d{2}-\d{2}$/
const QTY_MAX_FRACTION_DIGITS = 10

const qtyFmt = (locale: Locales) =>
  new Intl.NumberFormat(LOCALE_TAG[locale], {
    maximumFractionDigits: QTY_MAX_FRACTION_DIGITS,
  })

function parseDate(value: unknown): Date | null {
  if (value instanceof Date) return Number.isNaN(value.getTime()) ? null : value
  if (typeof value === 'string') {
    if (ISO_DATE_ONLY.test(value)) {
      const [year, month, day] = value.split('-').map(Number)
      const d = new Date(year, month - 1, day)
      return Number.isNaN(d.getTime()) ? null : d
    }
    const d = new Date(value)
    return Number.isNaN(d.getTime()) ? null : d
  }
  if (typeof value === 'number') {
    const d = new Date(value)
    return Number.isNaN(d.getTime()) ? null : d
  }
  return null
}

export const initFormatters: FormattersInitializer<Locales, Formatters> = (locale: Locales) => {
  const formatters: Formatters = {
    /**
     * Locale-aware short date (DD/MM/YYYY in es-MX, MM/DD/YYYY in en-US).
     * Accepts ISO date strings ("YYYY-MM-DD"), `Date` objects, or timestamps.
     * Returns an empty string for nullish / unparseable values so it can be
     * rendered inside larger templates without producing "Invalid Date".
     */
    shortDate: (value: unknown) => {
      const d = parseDate(value)
      return d ? shortDateFmt(locale).format(d) : ''
    },
    /**
     * Locale-aware date+time for resolution-history rows.
     */
    dateTime: (value: unknown) => {
      const d = parseDate(value)
      return d ? dateTimeFmt(locale).format(d) : ''
    },
    /**
     * Locale-aware quantity / number that preserves precision (up to
     * `QTY_MAX_FRACTION_DIGITS` fractional digits) so it matches the previous
     * `qty.toString()` behaviour without rounding normal decimal quantities.
     * Returns an empty string for nullish / non-numeric values.
     */
    quantity: (value: unknown) => {
      if (typeof value !== 'number' || Number.isNaN(value)) return ''
      return qtyFmt(locale).format(value)
    },
  }

  return formatters
}