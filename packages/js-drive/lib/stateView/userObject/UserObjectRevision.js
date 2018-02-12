module.exports = class UserObjectRevision {
  constructor(revision, objectHash, blockHeight, packetId, createdAt) {
    this.revision = revision;
    this.objectHash = objectHash;
    this.blockHeight = blockHeight;
    this.packetId = packetId;
    this.createdAt = createdAt;
  }
};
