import * as placeholder from 'enquirer/lib/placeholder';

/**
 * @param {string} input
 * @param {Object} choice
 * @returns {*}
 */
export default function formatPercentage(input, choice) {
  let str;

  const number = Number(input);
  if (input === '' || Number.isNaN(number) || number.toFixed(2).length < input.length) {
    str = input;
  } else {
    str = number.toFixed(2);
  }

  const pos = Math.min(choice.cursor, str.length);

  const options = {
    input: str,
    initial: choice.initial,
    pos,
    showCursor: this.state.index === 1,
  };

  return placeholder(this, options);
}
