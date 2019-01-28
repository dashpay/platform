const Transaction = require('@dashevo/dashcore-lib/lib/transaction');

const createCIDFromHash = require('../storage/stPacket/createCIDFromHash');

class StateTransition extends Transaction {
  constructor(data) {
    super(data);

    /**
     * Get Packet CID
     *
     * @returns {CID} CID
     */
    this.getPacketCID = function getPacketCID() {
      return createCIDFromHash(this.extraPayload.hashSTPacket);
    };
  }
}

StateTransition.TYPES = Transaction.TYPES;

module.exports = StateTransition;
