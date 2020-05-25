const logger = require('../../logger');
const { StandardPlugin } = require('../');
const EVENTS = require('../../EVENTS');

const defaultOpts = {
  firstExecutionRequired: true,
  executeOnStart: true,
};

class ChainPlugin extends StandardPlugin {
  constructor(opts = {}) {
    const params = {
      name: 'ChainPlugin',
      executeOnStart: defaultOpts.executeOnStart,
      firstExecutionRequired: defaultOpts.firstExecutionRequired,
      dependencies: [
        'storage',
        'transporter',
        'fetchStatus',
        'walletId',
      ],
    };
    super(Object.assign(params, opts));
    this.isSubscribedToBlocks = false;
  }

  /**
   * Used to subscribe to blockheaders and provide BLOCK, BLOCKHEADER and BLOCKHEIGHT_CHANGED.
   * Also, maintain the blockheader storage up to date.
   * @return {Promise<void>}
   */
  async execBlockListener() {
    const self = this;
    if (!this.isSubscribedToBlocks) {
      self.transporter.on(EVENTS.BLOCK, async (ev) => {
        // const { network } = self.storage.store.wallets[self.walletId];
        const { payload: block } = ev;
        this.parentEvents.emit(EVENTS.BLOCK, { type: EVENTS.BLOCK, payload: block });
        // We do not announce BLOCKHEADER as this is done by Storage
        await self.storage.importBlockHeader(block.header);
      });
      await self.transporter.subscribeToBlocks();
    }
  }

  /**
   * Used on ChainPlugin to be able to report on BLOCKHEIGHT_CHANGED.
   * Neither Block or Blockheader contains blockheight, we need to fetch it from getStatus.blocks
   * @return {Promise<boolean|*>}
   */
  async execStatusFetch() {
    const res = await this.fetchStatus();
    if (!res) {
      return false;
    }
    const { blocks } = res;
    const { network } = this.storage.store.wallets[this.walletId];
    logger.debug('ChainPlugin - Setting up starting blockHeight', blocks);
    this.storage.store.chains[network.toString()].blockHeight = blocks;

    const bestBlock = await this.transporter.getBlockHeaderByHeight(blocks);
    await this.storage.importBlockHeader(bestBlock);

    return true;
  }

  async onStart() {
    await this.execStatusFetch();
    await this.execBlockListener();
  }
}

module.exports = ChainPlugin;
