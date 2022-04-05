const BLOCK_TIME_WINDOW_MINUTES = 5;

/**
 *
 * @param {number} lastBlockHeaderTime
 * @returns {{timeWindowStart: Date, timeWindowEnd: Date}}
 */
function getBlockTimeWindowRange(lastBlockHeaderTime) {
  // Define time window
  const timeWindowStart = new Date(lastBlockHeaderTime);
  timeWindowStart.setMinutes(
    timeWindowStart.getMinutes() - BLOCK_TIME_WINDOW_MINUTES,
  );

  const timeWindowEnd = new Date(lastBlockHeaderTime);
  timeWindowEnd.setMinutes(
    timeWindowEnd.getMinutes() + BLOCK_TIME_WINDOW_MINUTES,
  );

  return {
    timeWindowStart,
    timeWindowEnd,
  };
}

module.exports = getBlockTimeWindowRange;
