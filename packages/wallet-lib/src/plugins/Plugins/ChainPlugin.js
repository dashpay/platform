const logger = require('../../logger');
const { StandardPlugin } = require('..');
const { dashToDuffs } = require('../../utils');

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
      ],
    };
    super(Object.assign(params, opts));
  }

  /**
   * Used on ChainPlugin to be able to report on BLOCKHEIGHT_CHANGED.
   * Neither Block or Blockheader contains blockheight, we need to fetch it
   * from getBlockchainStatus.blocks
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

    logger.debug('ChainPlugin - Setting up starting chainHeight', blocks);

    chainStore.updateChainHeight(blocks);

    if (relay) {
      chainStore.state.fees.minRelay = dashToDuffs(relay);
    }

    return true;
  }

  async onStart() {
    await this.execStatusFetch();
  }
}

module.exports = ChainPlugin;
