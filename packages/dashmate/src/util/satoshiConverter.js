const SATOSHI_MULTIPLIER = 10 ** 8;

/**
 * Convert satoshis to Dash
 *
 * @param {number} satoshi
 *
 * @returns {number}
 */
export function toDash(satoshi) {
  return satoshi / SATOSHI_MULTIPLIER;
}

/**
 * Convert dash to satoshis
 *
 * @param {number} dash
 *
 * @return {number}
 */
export function toSatoshi(dash) {
  return dash * SATOSHI_MULTIPLIER;
}
