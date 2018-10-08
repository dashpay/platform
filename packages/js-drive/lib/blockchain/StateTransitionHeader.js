const Transaction = require('@dashevo/dashcore-lib/lib/transaction');
const StateTransitionPacket = require('../storage/StateTransitionPacket');

class StateTransitionHeader extends Transaction {
  constructor(data) {
    super(data);

    /**
     * Get Packet CID
     *
     * @returns {CID} CID
     */
    this.getPacketCID = function getPacketCID() {
      return StateTransitionPacket.createCIDFromHash(this.extraPayload.hashSTPacket);
    };
  }
}

StateTransitionHeader.TYPES = Transaction.TYPES;

module.exports = StateTransitionHeader;
