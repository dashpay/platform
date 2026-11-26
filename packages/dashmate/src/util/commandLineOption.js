/**
 * A command-line option given with its value in the same argument:
 * `-name=value` or `--name=value`
 */
const OPTION_WITH_VALUE = /^(--?[^=\s-][^=\s]*=)([\s\S]+)$/;

/**
 * @param {*} argument
 * @return {boolean} whether the argument is an option rather than a value
 */
export function isOption(argument) {
  return typeof argument === 'string' && argument.startsWith('-');
}

/**
 * @param {*} argument
 * @return {string|null} the value of an option given as `-name=value`
 */
export function getOptionValue(argument) {
  const match = typeof argument === 'string' ? argument.match(OPTION_WITH_VALUE) : null;

  return match ? match[2] : null;
}

/**
 * @param {*} argument
 * @param {function(string): string} hide
 * @return {*} the argument with the value of an option given as
 *   `-name=value` hidden and its name kept; any other argument unchanged
 */
export function hideOptionValue(argument, hide) {
  const match = typeof argument === 'string' ? argument.match(OPTION_WITH_VALUE) : null;

  return match ? `${match[1]}${hide(match[2])}` : argument;
}
