const Reference = require('./revisions/Reference');

const StateTransition = require('../blockchain/StateTransition');

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
   * @param {Object} rawStateTransition
   * @param {Object} block
   * @param {boolean} [reverting]
   * @returns {Promise<void>}
   */
  async function applyStateTransition(rawStateTransition, block, reverting = false) {
    const stateTransition = new StateTransition(rawStateTransition);

    // @TODO implement getStPacket
    const stPacket = null;

    if (stPacket.getContract()) {
      const reference = new Reference({
        blockHash: block.hash,
        blockHeight: block.height,
        stHash: stateTransition.hash,
        stPacketHash: stateTransition.extraPayload.hashSTPacket,
        hash: stPacket.getContract().hash(),
      });

      await updateSVContract(
        stPacket.getContractId(),
        stateTransition.extraPayload.regTxId,
        reference,
        stPacket.getContract(),
        reverting,
      );

      return;
    }

    for (const document of stPacket.getDocuments()) {
      const reference = new Reference({
        blockHash: block.hash,
        blockHeight: block.height,
        stHash: stateTransition.hash,
        stPacketHash: stateTransition.extraPayload.hashSTPacket,
        hash: document.hash(),
      });

      await updateSVDocument(
        stPacket.getContractId(),
        stateTransition.extraPayload.regTxId,
        reference,
        document,
        reverting,
      );
    }
  }

  return applyStateTransition;
}

module.exports = applyStateTransitionFactory;
