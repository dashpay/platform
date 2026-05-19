import { DUFFS_PER_DASH } from '../CONSTANTS.js';

function duffsToDash(duffs) {
  if (duffs === undefined || duffs.constructor.name !== Number.name) {
    throw new Error('Can only convert a number');
  }
  return duffs / DUFFS_PER_DASH;
}
export default duffsToDash;