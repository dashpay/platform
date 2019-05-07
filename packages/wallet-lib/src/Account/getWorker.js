const {
  UnknownDAP,
} = require('../errors/index');
/**
 * Get a worker by it's name
 * @param dapName
 * @return {*}
 */
function getDAP(dapName) {
  const loweredDapName = dapName.toLowerCase();
  const dapsList = Object.keys(this.plugins.workers).map(key => key.toLowerCase());
  if (dapsList.includes(loweredDapName)) {
    return this.plugins.workers[loweredDapName];
  }
  throw new UnknownDAP(loweredDapName);
}

module.exports = getDAP;
