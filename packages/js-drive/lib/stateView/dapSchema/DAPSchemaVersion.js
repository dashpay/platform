module.exports = class DAPSchemaVersion {
  constructor(version, objectHash, packetId, blockHeight, createdAt, updatedAt) {
    this.version = version;
    this.objectHash = objectHash;
    this.packetId = packetId;
    this.blockHeight = blockHeight;
    this.createdAt = createdAt;
    this.updatedAt = updatedAt;
  }
};
