const Document = require('../document/Document');
const Contract = require('../contract/Contract');

const createContract = require('../contract/createContract');

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
      const ct = contract instanceof Contract
        ? contract
        : createContract(contract);

      return Buffer.from(ct.hash(), 'hex');
    }),
  };
}

module.exports = calculateItemsHashes;
