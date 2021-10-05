const EVENTS = require('../../../EVENTS');

module.exports = async function subscribeToBlocks() {
  const self = this;
  const { executors } = this.state;

  const executor = async () => {
    const chainHash = await this.getBestBlockHash();
    if (!self.state.block || self.state.block.hash !== chainHash) {
      self.state.block = await self.getBlockByHash(await self.getBestBlockHash());
      self.announce(EVENTS.BLOCK, self.state.block);
      if (self.state.block && self.state.block.transactions[0].extraPayload.height) {
        const { height } = self.state.block.transactions[0].extraPayload;
        self.announce(EVENTS.BLOCKHEIGHT_CHANGED, height);
      }
    }
  };
  await executor();
  const refreshBlockInterval = 30 * 1000;// Every 30s
  executors.blocks = setInterval(() => executor(), refreshBlockInterval);
};
