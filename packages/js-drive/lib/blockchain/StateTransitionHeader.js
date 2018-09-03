const multihashes = require('multihashes');
const CID = require('cids');
const Transaction = require('@dashevo/dashcore-lib/lib/transaction');

class StateTransitionHeader extends Transaction {
  constructor(data) {
    super(data);

    /**
     * Get Packet CID
     *
     * @returns {string} CID
     */
    this.getPacketCID = function getPacketCID() {
      const buffer = Buffer.from(this.extraPayload.hashSTPacket, 'hex');
      const multihash = multihashes.encode(buffer, 'sha2-256');
      const cid = new CID(1, 'dag-cbor', multihash);
      return cid.toBaseEncodedString();
    };
  }
}

StateTransitionHeader.TYPES = Transaction.TYPES;

module.exports = StateTransitionHeader;
