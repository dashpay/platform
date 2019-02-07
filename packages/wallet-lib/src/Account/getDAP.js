const {
  UnknownDAP,
} = require('../errors/index');
/**
 * Get a dap by it's name
 * @param dapName
 * @return {*}
 */
function getDAP(dapName) {
  const loweredDapName = dapName.toLowerCase();
  const dapsList = Object.keys(this.plugins.daps).map(key => key.toLowerCase());
  if (dapsList.includes(loweredDapName)) {
    return this.plugins.daps[loweredDapName];
  }
  throw new UnknownDAP(loweredDapName);
}

module.exports = getDAP;
