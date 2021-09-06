const { DUFFS_PER_DASH } = require('../CONSTANTS');

function duffsToDash(duffs) {
  if (duffs === undefined || duffs.constructor.name !== Number.name) {
    throw new Error('Can only convert a number');
  }
  return duffs / DUFFS_PER_DASH;
}
module.exports = duffsToDash;
