const { DUFFS_PER_DASH } = require('../CONSTANTS');

function dashToDuffs(dash) {
  if (dash === undefined || dash.constructor.name !== Number.name) {
    throw new Error('Can only convert a number');
  }
  return parseInt((dash * DUFFS_PER_DASH).toFixed(0), 10);
}
module.exports = dashToDuffs;
