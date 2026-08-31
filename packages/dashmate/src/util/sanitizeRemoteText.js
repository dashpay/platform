/**
 * Longest remote message that is printed. Long enough for a registry error to
 * stay useful, short enough that it cannot push the rest of the output out of
 * the operator's scrollback.
 */
const MAX_LENGTH = 500;

/**
 * Characters that must never reach a terminal: the C0 controls, which include
 * ESC and therefore every ANSI sequence, DEL, the C1 controls, and the
 * bidirectional overrides that let text reorder how it is displayed.
 */
// eslint-disable-next-line no-control-regex
const UNPRINTABLE = /[\u0000-\u001F\u007F-\u009F\u200E-\u200F\u202A-\u202E\u2066-\u2069]/g;

/**
 * Make text received from a remote service safe to print
 *
 * A Docker registry chooses the text of the errors it returns, and that text is
 * relayed to the operator's terminal. Escape sequences in it can rewrite lines
 * that were already printed, hide what follows or address the terminal itself,
 * so nothing but printable characters is passed on.
 *
 * @param {*} text
 * @return {*} the text with control characters removed and its length bounded,
 *             or the value unchanged when it is not a string
 */
export default function sanitizeRemoteText(text) {
  if (typeof text !== 'string') {
    return text;
  }

  const printable = text
    .replace(UNPRINTABLE, ' ')
    .replace(/ {2,}/g, ' ')
    .trim();

  if (printable.length <= MAX_LENGTH) {
    return printable;
  }

  return `${printable.slice(0, MAX_LENGTH)} (truncated)`;
}
