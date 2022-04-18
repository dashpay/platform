const logger = require('../../logger');
const { StandardPlugin } = require('..');
const EVENTS = require('../../EVENTS');
const { dashToDuffs } = require('../../utils');
const ChainSyncMediator = require('../../types/Wallet/ChainSyncMediator');

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
      awaitOnInjection: true,
      dependencies: [
        'storage',
        'transport',
        'fetchStatus',
        'walletId',
        'chainSyncMediator',
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
    const { network } = this.storage.application;
    const chainStore = this.storage.getChainStore(network);
    const walletStore = this.storage.getWalletStore(this.walletId);

    if (!this.isSubscribedToBlocks) {
      self.transport.on(EVENTS.BLOCK, async (ev) => {
        const { payload: block } = ev;
        this.parentEvents.emit(EVENTS.BLOCK, { type: EVENTS.BLOCK, payload: block });
      });
      self.transport.on(EVENTS.BLOCKHEIGHT_CHANGED, async (ev) => {
        const { payload: blockheight } = ev;

        this.parentEvents.emit(EVENTS.BLOCKHEIGHT_CHANGED, {
          type: EVENTS.BLOCKHEIGHT_CHANGED, payload: blockheight,
        });

        chainStore.state.blockHeight = blockheight;

        // Update last known block for the wallet only if we are in the state of the incoming sync.
        // (During the historical sync, it is populated from transactions metadata)
        if (this.chainSyncMediator.state === ChainSyncMediator.STATES.CONTINUOUS_SYNC) {
          walletStore.updateLastKnownBlock(blockheight);
          this.storage.scheduleStateSave();
        }

        logger.debug(`ChainPlugin - setting chain blockheight ${blockheight}`);
      });
      await self.transport.subscribeToBlocks();
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

    const { network } = this.storage.application;
    const chainStore = this.storage.getChainStore(network);
    const { chain: { blocksCount: blocks }, network: { fee: { relay } } } = res;

    logger.debug('ChainPlugin - Setting up starting blockHeight', blocks);

    chainStore.state.blockHeight = blocks;

    if (relay) {
      chainStore.state.fees.minRelay = dashToDuffs(relay);
    }

    return true;
  }

  async onStart() {
    this.chainSyncMediator.state = ChainSyncMediator.STATES.CHAIN_STATUS_SYNC;
    await this.execStatusFetch();
    await this.execBlockListener();
  }
}

module.exports = ChainPlugin;
