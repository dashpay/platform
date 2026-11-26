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
 * @param {string} character
 * @return {string} the character as a dollar-single-quoted string holds it
 */
function escapeForDollarQuotes(character) {
  if (character === '\\' || character === "'") {
    return `\\${character}`;
  }

  if (!CONTROL_CHARACTER.test(character)) {
    return character;
  }

  // Three octal digits always end the escape, whatever follows.
  return Array.from(Buffer.from(character, 'utf8'))
    .map((byte) => `\\${byte.toString(8).padStart(3, '0')}`)
    .join('');
}

/**
 * Quote a value as one argument of a command an operator is meant to copy and
 * run.
 *
 * Single quotes keep everything literal but a single quote, which is written
 * as a quote that ends the quoting, an escaped quote and a quote that starts
 * it again. Control characters would go into the printed command as they are,
 * a line break splitting it in two, so a value holding one is written as a
 * dollar-single-quoted string (POSIX.1-2024, bash, zsh, ksh) with each of them
 * escaped, which keeps the command on one line.
 *
 * @param {string} value
 * @return {string}
 */
export default function quoteShellArgument(value) {
  const text = String(value);

  if (SHELL_SAFE.test(text)) {
    return text;
  }

  if (!CONTROL_CHARACTER.test(text)) {
    return `'${text.replace(/'/g, "'\\''")}'`;
  }

  return `$'${Array.from(text).map(escapeForDollarQuotes).join('')}'`;
}
