/* eslint-disable no-console */

module.exports = function wait(ms, description = '') {
  const details = description ? `${description} (${ms}ms)` : `${ms}ms delay`;

  console.debug(`Waiting for ${details}`);

  return new Promise((res) => { setTimeout(res, ms); });
};
