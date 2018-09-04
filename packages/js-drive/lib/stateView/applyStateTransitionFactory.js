const StateTransitionHeader = require('../blockchain/StateTransitionHeader');
const StateTransitionPacket = require('../storage/StateTransitionPacket');
const Reference = require('./Reference');

/**
 * @param {IpfsAPI} ipfs
 * @param {updateDapContract} updateDapContract
 * @param {updateDapObject} updateDapObject
 * @returns {applyStateTransition}
 */
function applyStateTransitionFactory(ipfs, updateDapContract, updateDapObject) {
  /**
   * @typedef {Promise} applyStateTransition
   * @param {object} header
   * @param {object} block
   * @returns {Promise<void>}
   */
  async function applyStateTransition(header, block) {
    const stHeader = new StateTransitionHeader(header);
    const cid = stHeader.getPacketCID();
    const packetData = await ipfs.dag.get(cid);
    const packet = new StateTransitionPacket(packetData.value);

    if (packet.dapcontract) {
      const dapId = header.txid;
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
  }

  return applyStateTransition;
}

module.exports = applyStateTransitionFactory;
