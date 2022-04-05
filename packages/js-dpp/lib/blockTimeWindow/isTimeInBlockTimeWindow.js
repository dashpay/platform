const getBlockTimeWindowRange = require('./getBlockTimeWindowRange');

/**
 *
 * @param {number} lastBlockHeaderTime
 * @param {number} timeToCheck
 * @returns {boolean}
 */
function isTimeInBlockTimeWindow(lastBlockHeaderTime, timeToCheck) {
  const {
    timeWindowStart,
    timeWindowEnd,
  } = getBlockTimeWindowRange(lastBlockHeaderTime);
  return timeToCheck >= timeWindowStart.getTime() && timeToCheck <= timeWindowEnd.getTime();
}

module.exports = isTimeInBlockTimeWindow;
