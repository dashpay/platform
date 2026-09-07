/**
 * Whether an environment variable carrying a boolean is switched on.
 *
 * An unset variable, "0", "false" and an empty value all mean off. The
 * comparison is case-folded because the systems that set these write TRUE,
 * True and true interchangeably and all three mean the same thing.
 *
 * @param {string|undefined} value
 * @return {boolean}
 */
export default function isEnvironmentFlagSet(value) {
  if (value === undefined || value === null) {
    return false;
  }

  const normalized = String(value).trim().toLowerCase();

  return normalized !== '' && normalized !== '0' && normalized !== 'false';
}
