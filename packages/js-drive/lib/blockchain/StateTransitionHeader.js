const multihashes = require('multihashes');
const CID = require('cids');
const TransitionHeader = require('@dashevo/dashcore-lib/lib/stateTransition/transitionHeader');

class StateTransitionHeader extends TransitionHeader {
  constructor(data) {
    super(data);

    /**
     * Get Packet CID
     *
     * @returns {string} CID
     */
    this.getPacketCID = function getPacketCID() {
      const buffer = Buffer.from(this.hashDataMerkleRoot, 'hex');
      const multihash = multihashes.encode(buffer, 'sha2-256');
      const cid = new CID(1, 'dag-cbor', multihash);
      return cid.toBaseEncodedString();
    };
  }
}

module.exports = StateTransitionHeader;
