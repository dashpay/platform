const DPObject = require('../object/DPObject');
const DPContract = require('../contract/DPContract');

const createDPContract = require('../contract/createDPContract');

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
      const dpObject = object instanceof DPObject ? object : new DPObject(object);

      return Buffer.from(dpObject.hash(), 'hex');
    }),
    contracts: contracts.map((contract) => {
      const dpContract = contract instanceof DPContract
        ? contract
        : createDPContract(contract);

      return Buffer.from(dpContract.hash(), 'hex');
    }),
  };
}

module.exports = calculateItemsHashes;
