const ValidationResult = require('./ValidationResult');

const BLOCK_TIME_WINDOW_MINUTES = 5;

/**
 *
 * @param {number} lastBlockHeaderTime
 * @param {number} timeToCheck
 * @returns {ValidationResult}
 */
function validateTimeInBlockTimeWindow(lastBlockHeaderTime, timeToCheck) {
// Define time window
  const timeWindowStart = new Date(lastBlockHeaderTime);
  timeWindowStart.setMinutes(
    timeWindowStart.getMinutes() - BLOCK_TIME_WINDOW_MINUTES,
  );

  const timeWindowEnd = new Date(lastBlockHeaderTime);
  timeWindowEnd.setMinutes(
    timeWindowEnd.getMinutes() + BLOCK_TIME_WINDOW_MINUTES,
  );

  const isValid = timeToCheck >= timeWindowStart.getTime()
    && timeToCheck <= timeWindowEnd.getTime();

  return new ValidationResult(isValid, timeWindowStart, timeWindowEnd);
}

module.exports = validateTimeInBlockTimeWindow;
