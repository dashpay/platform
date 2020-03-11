const EVENTS = require('../../../../EVENTS');

module.exports = async function subscribeToBlocks() {
  const self = this;
  const { subscriptions } = this.state;

  const executor = async () => {
    const chainHash = await this.getBestBlockHash();
    if (!self.state.block || self.state.block.hash !== chainHash) {
      self.state.block = await self.getBlockByHash(await self.getBestBlockHash());
      self.announce(EVENTS.BLOCK, self.state.block);
    }
  };
  await executor();
  const refreshBlockInterval = 10 * 1000;// Every 10s
  subscriptions.blocks = setInterval(() => executor(), refreshBlockInterval);
};
