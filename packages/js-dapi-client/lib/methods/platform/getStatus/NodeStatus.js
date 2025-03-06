class NodeStatus {
  /**
   * @param {string} nodeId - Node ID
   * @param {string=} proTxHash - Node's proTxHash
   */
  constructor(nodeId, proTxHash) {
    this.nodeId = nodeId;
    this.proTxHash = proTxHash || null;
  }

  /**
   * @returns {string} Node ID
   */
  getNodeId() {
    return this.nodeId;
  }

  /**
   * @returns {string} Pro Tx Hash
   */
  getProTxHash() {
    return this.proTxHash;
  }
}

module.exports = NodeStatus;
