const EVENTS = require('../../../../EVENTS');

module.exports = async function subscribeToBlockHeaders() {
  const self = this;
  const { subscriptions } = this.state;

  const executor = async () => {
    const chainHash = await this.getBestBlockHash();
    if (!self.state.blockHeader || self.state.blockHeader.hash !== chainHash) {
      self.state.blockHeader = await self.getBlockHeaderByHash(chainHash);
      self.announce(EVENTS.BLOCKHEADER, self.state.blockHeader);
    }
  };
  await executor();
  const refreshBlockInterval = 10 * 1000;// Every 10s
  subscriptions.blockHeaders = setInterval(() => executor(), refreshBlockInterval);
};
