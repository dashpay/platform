const Reference = require('./revisions/Reference');

/**
 * @param {updateSVContract} updateSVContract
 * @param {updateSVDocument} updateSVDocument
 * @returns {applyStateTransition}
 */
function applyStateTransitionFactory(
  updateSVContract,
  updateSVDocument,
) {
  /**
   * @typedef {Promise} applyStateTransition
   * @param {STPacket} stPacket
   * @param {StateTransition} stHeader
   * @param {string} blockHash
   * @param {number} blockHeight
   * @param {MongoDBTransaction} transaction
   * @returns {Promise<Object>}
   */
  async function applyStateTransition(stPacket, stHeader, blockHash, blockHeight, transaction) {
    if (stPacket.getContract()) {
      const reference = new Reference({
        blockHash,
        blockHeight,
        stHash: stHeader.hash,
        stPacketHash: stHeader.extraPayload.hashSTPacket,
        hash: stPacket.getContract().hash(),
      });

      const svContract = await updateSVContract(
        stPacket.getContractId(),
        stHeader.extraPayload.regTxId,
        reference,
        stPacket.getContract(),
        transaction,
      );

      return { svContract };
    }

    for (const document of stPacket.getDocuments()) {
      const reference = new Reference({
        blockHash,
        blockHeight,
        stHash: stHeader.hash,
        stPacketHash: stHeader.extraPayload.hashSTPacket,
        hash: document.hash(),
      });

      await updateSVDocument(
        stPacket.getContractId(),
        stHeader.extraPayload.regTxId,
        reference,
        document,
        transaction,
      );
    }

    return {};
  }

  return applyStateTransition;
}

module.exports = applyStateTransitionFactory;
