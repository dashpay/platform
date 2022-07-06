/**
 * Get milliseconds time from seconds and nanoseconds
 *
 * @param {number} seconds
 * @param {number} nanoseconds
 *
 * @returns {number}
 */
function timeToMillis(seconds, nanoseconds) {
  const overallNanos = nanoseconds + seconds * (10 ** 9);

  return overallNanos / (10 ** 6);
}

module.exports = timeToMillis;
