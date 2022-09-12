const DAPIAddress = require('@dashevo/dapi-client/lib/dapiAddressProvider/DAPIAddress');

function getDAPISeeds() {
  return process.env.DAPI_SEED
    .split(',')
    .map((seed) => {
      const address = new DAPIAddress(seed);

      return address.toJSON();
    });
}

module.exports = getDAPISeeds;
