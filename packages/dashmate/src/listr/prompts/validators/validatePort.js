/**
 * Validate string input being a port
 *
 * @param {string} value
 * @returns {boolean}
 */
export function validatePort(value) {
  const portNumber = Math.floor(Number(value));

  return portNumber >= 1
    && portNumber <= 65535
    && portNumber.toString() === value;
}
