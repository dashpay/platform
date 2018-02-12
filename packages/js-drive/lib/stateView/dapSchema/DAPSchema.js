module.exports = class DAPSchema {
  constructor(dapId, schema, createdAt) {
    this.dapId = dapId;
    this.schema = schema;
    this.createdAt = createdAt;
  }
};
