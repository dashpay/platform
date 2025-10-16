const wait = require('./wait');

/**
 * @returns {Promise<void>}
 */
async function waitForSTPropagated() {
  let interval = 3000;

  if (process.env.ST_EXECUTION_INTERVAL) {
    interval = parseInt(process.env.ST_EXECUTION_INTERVAL, 10);
  }

  await wait(interval);
}

module.exports = waitForSTPropagated;
