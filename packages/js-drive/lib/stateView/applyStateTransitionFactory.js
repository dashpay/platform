const Reference = require('./revisions/Reference');

const ReaderMediator = require('../blockchain/reader/BlockchainReaderMediator');
const StateTransition = require('../blockchain/StateTransition');

/**
 * @param {STPacketIpfsRepository} stPacketRepository
 * @param {updateSVContract} updateSVContract
 * @param {updateSVDocument} updateSVDocument
 * @param {BlockchainReaderMediator} readerMediator
 * @returns {applyStateTransition}
 */
function applyStateTransitionFactory(
  stPacketRepository,
  updateSVContract,
  updateSVDocument,
  readerMediator,
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

    const stPacket = await stPacketRepository
      .find(stateTransition.getPacketCID());

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

      await readerMediator.emitSerial(ReaderMediator.EVENTS.CONTRACT_APPLIED, {
        userId: stateTransition.extraPayload.regTxId,
        contractId: stPacket.getContractId(),
        reference,
        contract: stPacket.getContract().toJSON(),
      });

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

      await readerMediator.emitSerial(ReaderMediator.EVENTS.DOCUMENT_APPLIED, {
        userId: stateTransition.extraPayload.regTxId,
        contractId: stPacket.getContractId(),
        documentId: document.getId(),
        reference,
        document: document.toJSON(),
      });
    }
  }

  return applyStateTransition;
}

module.exports = applyStateTransitionFactory;
