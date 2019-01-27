const Reference = require('./revisions/Reference');

const ReaderMediator = require('../blockchain/reader/BlockchainReaderMediator');
const StateTransitionHeader = require('../blockchain/StateTransitionHeader');

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
   * @param {object} header
   * @param {object} block
   * @param {boolean} [reverting]
   * @returns {Promise<void>}
   */
  async function applyStateTransition(header, block, reverting = false) {
    const stHeader = new StateTransitionHeader(header);

    const stPacket = await stPacketRepository
      .find(stHeader.getPacketCID());

    if (stPacket.getDPContract()) {
      const reference = new Reference({
        blockHash: block.hash,
        blockHeight: block.height,
        stHeaderHash: header.hash,
        stPacketHash: header.extraPayload.hashSTPacket,
        hash: stPacket.getDPContract().hash(),
      });

      await updateSVContract(
        stPacket.getDPContractId(),
        reference,
        stPacket.getDPContract(),
        reverting,
      );

      await readerMediator.emitSerial(ReaderMediator.EVENTS.DP_CONTRACT_APPLIED, {
        userId: header.extraPayload.regTxId,
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
        stHeaderHash: header.hash,
        stPacketHash: header.extraPayload.hashSTPacket,
        hash: dpObject.hash(),
      });

      await updateSVObject(
        stPacket.getDPContractId(),
        header.extraPayload.regTxId,
        reference,
        dpObject,
        reverting,
      );

      await readerMediator.emitSerial(ReaderMediator.EVENTS.DP_OBJECT_APPLIED, {
        userId: header.extraPayload.regTxId,
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
