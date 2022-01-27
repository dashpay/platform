const sleep = (time) => new Promise((resolve) => setTimeout(resolve, time));

class TempChainCache {
  constructor() {
    this.transactionsByBlockHash = {};
    this.transactionsMetadata = {};
    this.self = null;
    this.blockHeadersProvider = null;
  }

  static i() {
    if (!this.self) {
      this.self = new TempChainCache();
      this.self.startStuff().catch((e) => {
        console.log(e);
      });
    }
    return this.self;
  }

  async startStuff() {
    while (true) {
      Object.keys(this.transactionsByBlockHash).forEach((blockHash) => {
        const transactions = this.transactionsByBlockHash[blockHash];

        const headerHeight = this.blockHeadersProvider.spvChain.headersHeights.get(blockHash);
        if (headerHeight) {
          transactions.forEach((txHash) => {
            this.transactionsMetadata[txHash] = {
              height: headerHeight,
              blockHash,
            };
          });
        }
      });

      // eslint-disable-next-line no-await-in-loop
      await sleep(1000);
    }
  }
}

module.exports = TempChainCache;
