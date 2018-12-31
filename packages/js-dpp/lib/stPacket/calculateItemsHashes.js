const DapObject = require('../dapObject/DapObject');
const DapContract = require('../dapContract/DapContract');

const createDapContract = require('../dapContract/createDapContract');

/**
 * Get all hashes of all items in a packets as an array of buffers
 *
 * @private
 * @returns {{
 *   objects: Buffer[],
 *   contracts: Buffer[]
 * }}
 */
function calculateItemsHashes({ contracts, objects }) {
  return {
    objects: objects.map((object) => {
      const dapObject = object instanceof DapObject ? object : new DapObject(object);

      return Buffer.from(dapObject.hash(), 'hex');
    }),
    contracts: contracts.map((contract) => {
      const dapContract = contract instanceof DapContract
        ? contract
        : createDapContract(contract);

      return Buffer.from(dapContract.hash(), 'hex');
    }),
  };
}

module.exports = calculateItemsHashes;
