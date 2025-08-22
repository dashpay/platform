class NetworkStatus {
  /**
   * @param {string} chainId - Chain id
   * @param {number} peersCount - Peers count
   * @param {boolean} listening - Is listening to P2P network
   */
  constructor(chainId, peersCount, listening) {
    this.chainId = chainId;
    this.peersCount = peersCount;
    this.listening = listening;
  }

  /**
   * @returns {string} chain id
   */
  getChainId() {
    return this.chainId;
  }

  /**
   * @returns {number} peers count
   */
  getPeersCount() {
    return this.peersCount;
  }

  /**
   * @returns {boolean} is listening to p2p
   */
  isListening() {
    return this.listening;
  }
}

module.exports = NetworkStatus;
