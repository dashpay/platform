/**
 * @param {string} input
 * @return {string}
 */
export default function convertToHomographSafeChars(input: string): string {
  return input.toLowerCase().replace(/[oli]/g, (match) => {
    if (match === 'o') {
      return '0';
    }

    if (match === 'l' || match === 'i') {
      return '1';
    }

    return match;
  });
}
