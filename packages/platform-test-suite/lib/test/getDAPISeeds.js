const DAPIAddress = require('@dashevo/dapi-client/lib/dapiAddressProvider/DAPIAddress');

function getDAPISeeds() {
  if (typeof process.env.DAPI_SEED === 'undefined') {
    return undefined;
  }

  return process.env.DAPI_SEED
    .split(',')
    .map((seed) => {
      const address = new DAPIAddress(seed);

      return address.toJSON();
    });
}

module.exports = getDAPISeeds;
