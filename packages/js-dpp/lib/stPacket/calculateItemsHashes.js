const Document = require('../document/Document');
const DPContract = require('../contract/DPContract');

const createDPContract = require('../contract/createDPContract');

/**
 * Get all hashes of all items in a packets as an array of buffers
 *
 * @private
 * @returns {{
 *   documents: Buffer[],
 *   contracts: Buffer[]
 * }}
 */
function calculateItemsHashes({ contracts, documents }) {
  return {
    documents: documents.map((document) => {
      const doc = document instanceof Document ? document : new Document(document);

      return Buffer.from(doc.hash(), 'hex');
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
