const {
  UnknownDPA,
} = require('../../../errors');
/**
 * Get a DPA by it's name
 * @param dpaName
 * @return {*}
 */
function getDPA(dpaName) {
  const loweredDPAName = dpaName.toLowerCase();
  const DpaList = Object.keys(this.plugins.DPAs).map((key) => key.toLowerCase());
  if (DpaList.includes(loweredDPAName)) {
    return this.plugins.DPAs[loweredDPAName];
  }
  throw new UnknownDPA(loweredDPAName);
}

module.exports = getDPA;
