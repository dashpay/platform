const Reference = require('./revisions/Reference');

const ReaderMediator = require('../blockchain/reader/BlockchainReaderMediator');
const StateTransition = require('../blockchain/StateTransition');

/**
 * @param {STPacketIpfsRepository} stPacketRepository
 * @param {updateSVContract} updateSVContract
 * @param {updateSVObject} updateSVObject
 * @param {BlockchainReaderMediator} readerMediator
 * @returns {applyStateTransition}
 */
function applyStateTransitionFactory(
  stPacketRepository,
  updateSVContract,
  updateSVObject,
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

    if (stPacket.getDPContract()) {
      const reference = new Reference({
        blockHash: block.hash,
        blockHeight: block.height,
        stHash: stateTransition.hash,
        stPacketHash: stateTransition.extraPayload.hashSTPacket,
        hash: stPacket.getDPContract().hash(),
      });

      await updateSVContract(
        stPacket.getDPContractId(),
        stateTransition.extraPayload.regTxId,
        reference,
        stPacket.getDPContract(),
        reverting,
      );

      await readerMediator.emitSerial(ReaderMediator.EVENTS.DP_CONTRACT_APPLIED, {
        userId: stateTransition.extraPayload.regTxId,
        contractId: stPacket.getDPContractId(),
        reference,
        contract: stPacket.getDPContract().toJSON(),
      });

      return;
    }

    for (const dpObject of stPacket.getDPObjects()) {
      const reference = new Reference({
        blockHash: block.hash,
        blockHeight: block.height,
        stHash: stateTransition.hash,
        stPacketHash: stateTransition.extraPayload.hashSTPacket,
        hash: dpObject.hash(),
      });

      await updateSVObject(
        stPacket.getDPContractId(),
        stateTransition.extraPayload.regTxId,
        reference,
        dpObject,
        reverting,
      );

      await readerMediator.emitSerial(ReaderMediator.EVENTS.DP_OBJECT_APPLIED, {
        userId: stateTransition.extraPayload.regTxId,
        contractId: stPacket.getDPContractId(),
        objectId: dpObject.getId(),
        reference,
        object: dpObject.toJSON(),
      });
    }
  }

  return applyStateTransition;
}

module.exports = applyStateTransitionFactory;
