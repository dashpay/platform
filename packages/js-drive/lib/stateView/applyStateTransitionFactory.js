const StateTransitionHeader = require('../blockchain/StateTransitionHeader');
const StateTransitionPacket = require('../storage/StateTransitionPacket');
const Reference = require('./Reference');
const doubleSha256 = require('../util/doubleSha256');
const PacketNotFoundError = require('../storage/errors/PacketNotFoundError');
const rejectAfter = require('../util/rejectAfter');

/**
 * @param {IpfsAPI} ipfs
 * @param {updateDapContract} updateDapContract
 * @param {updateDapObject} updateDapObject
 * @param {number} ipfsGetTimeout
 * @returns {applyStateTransition}
 */
function applyStateTransitionFactory(ipfs, updateDapContract, updateDapObject, ipfsGetTimeout) {
  /**
   * @typedef {Promise} applyStateTransition
   * @param {object} header
   * @param {object} block
   * @returns {Promise<void>}
   */
  async function applyStateTransition(header, block) {
    try {
      const stHeader = new StateTransitionHeader(header);

      const getPromise = ipfs.dag.get(stHeader.getPacketCID());
      const error = new PacketNotFoundError();
      const { value: packetData } = await rejectAfter(getPromise, error, ipfsGetTimeout);

      const packet = new StateTransitionPacket(packetData);

      if (packet.dapcontract) {
        const dapId = packet.dapcontract.upgradedapid || doubleSha256(packet.dapcontract);
        const reference = new Reference(
          block.hash,
          block.height,
          header.hash,
          header.extraPayload.hashSTPacket,
        );
        await updateDapContract(dapId, reference, packet.dapcontract);
        return;
      }

      for (let i = 0; i < packet.dapobjects.length; i++) {
        const objectData = packet.dapobjects[i];
        const reference = new Reference(
          block.hash,
          block.height,
          header.hash,
          header.extraPayload.hashSTPacket,
        );
        await updateDapObject(packet.dapid, header.extraPayload.regTxId, reference, objectData);
      }
    } catch (e) {
      const errorContext = {
        blockHeight: block.height,
        blockHash: block.hash,
        packetHash: header.extraPayload.hashSTPacket,
        packetCid: header.getPacketCID().toBaseEncodedString(),
      };
      console.error(new Date(), e, errorContext);
    }
  }

  return applyStateTransition;
}

module.exports = applyStateTransitionFactory;
