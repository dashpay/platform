class Reference {
  constructor(blockHash, blockHeight, stHeaderHash, stPacketHash, objectHash) {
    this.blockHash = blockHash;
    this.blockHeight = blockHeight;
    this.stHeaderHash = stHeaderHash;
    this.stPacketHash = stPacketHash;
    this.objectHash = objectHash;
  }

  toJSON() {
    return {
      blockHash: this.blockHash,
      blockHeight: this.blockHeight,
      stHeaderHash: this.stHeaderHash,
      stPacketHash: this.stPacketHash,
      objectHash: this.objectHash,
    };
  }
}

module.exports = Reference;
