/**
 * Values a POSIX shell passes through untouched.
 */
const SHELL_SAFE = /^[A-Za-z0-9._@%+=:,/-]+$/;

/**
 * What a terminal acts on rather than shows: C0 controls, DEL and C1 controls.
 */
// eslint-disable-next-line no-control-regex
const CONTROL_CHARACTER = /[\u0000-\u001f\u007f-\u009f]/u;

/**
 * Whether a value can be one argument of a printed command, on one line, that
 * every POSIX shell reads the same way
 *
 * Plain single quotes are that quoting, and they keep a control character as
 * it is: a line break would split the command in two, the second line reading
 * as a command of its own. Dollar-single quotes escape it, but older shells,
 * such as dash (Debian's and Ubuntu's /bin/sh), read them as a dollar sign
 * and a quoted string, and the command would act on another value.
 *
 * @param {string} value
 * @return {boolean}
 */
export function isShellQuotable(value) {
  return !CONTROL_CHARACTER.test(String(value));
}

/**
 * Quote a value as one argument of a command an operator is meant to copy and
 * run.
 *
 * Single quotes keep everything literal but a single quote, which is written
 * as a quote that ends the quoting, an escaped quote and a quote that starts
 * it again.
 *
 * @param {string} value - one isShellQuotable accepts
 * @return {string}
 */
export default function quoteShellArgument(value) {
  const text = String(value);

  if (SHELL_SAFE.test(text)) {
    return text;
  }

  if (!isShellQuotable(text)) {
    throw new Error('A value holding control characters cannot be quoted for a POSIX shell');
  }

  return `'${text.replace(/'/g, "'\\''")}'`;
}
