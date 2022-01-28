const sleep = (time) => new Promise((resolve) => setTimeout(resolve, time));

class TempChainCache {
  constructor() {
    this.transactionsByBlockHash = {};
    this.blockHashesPerTx = {};
    this.transactionsMetadata = {};
    this.self = null;
    this.blockHeadersProvider = null;
  }

  static i() {
    if (!this.self) {
      this.self = new TempChainCache();
    }
    return this.self;
  }

  getTransactionMetadata(txHash) {
    const blockHash = this.blockHashesPerTx[txHash];

    if (blockHash) {
      const headerHeight = this.blockHeadersProvider.spvChain.headersHeights.get(blockHash);
      const longestChain = this.blockHeadersProvider.spvChain.getLongestChain();
      const header = longestChain[headerHeight];
      return {
        height: headerHeight,
        blockHash,
        blockHeader: header,
      };
    }
    return null;
  }

  addTransactionsForBlockHash(hash, txHashes) {
    this.transactionsByBlockHash[hash] = txHashes;
    txHashes.forEach((txHash) => {
      this.blockHashesPerTx[txHash] = hash;
    });
  }
}

module.exports = TempChainCache;
