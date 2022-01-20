const { SpvChain } = require('@dashevo/dash-spv');

const BlockHeadersReader = require('./BlockHeadersReader');

/**
 * @typedef {BlockHeadersProviderOptions} BlockHeadersProviderOptions
 * @property {string} [network=testnet]
 * @property {boolean} [autoStart=false]
 * @property {number} [maxParallelStreams=5] max parallel streams to read historical block headers
 * @property {number} [targetBatchSize=100000] a target batch size per stream
 */
const defaultOptions = {
  network: 'testnet',
  autoStart: false,
  maxParallelStreams: 5,
  targetBatchSize: 100000,
  fromBlockHeight: 1,
};

class BlockHeadersProvider {
  /**
   * @param {BlockHeadersProviderOptions} options
   */
  constructor(options = {}) {
    this.options = {
      ...defaultOptions,
      ...options,
    };

    // TODO: Dash SPV does not understand 'regtest'. Fix
    const network = this.options.network === 'regtest' ? 'devnet' : this.options.network;
    this.spvChain = new SpvChain(network);
  }

  /**
   * @param {CoreMethodsFacade} coreMethods
   */
  setCoreMethods(coreMethods) {
    this.coreMethods = coreMethods;
  }

  async start() {
    if (!this.coreMethods) {
      throw new Error('Core methods have not been provided. Please use "setCoreMethods"');
    }

    const { chain: { blocksCount: bestBlockHeight } } = await this.coreMethods.getStatus();
    const blockHeadersReader = new BlockHeadersReader(
      {
        coreMethods: this.coreMethods,
        maxParallelStreams: this.options.maxParallelStreams,
        targetBatchSize: this.options.targetBatchSize,
      },
    );

    blockHeadersReader.on(BlockHeadersReader.EVENTS.BLOCK_HEADERS, (headers) => {
      this.spvChain.addHeaders(headers.map((header) => Buffer.from(header)));
    });

    blockHeadersReader
      .on(BlockHeadersReader.EVENTS.HISTORICAL_BLOCK_HEADERS_OBTAINED, () => {
        blockHeadersReader
          .subscribeToNew(bestBlockHeight)
          .catch((e) => {
            throw e;
          });
      });

    blockHeadersReader.on('error', (e) => {
      throw e;
    });

    await blockHeadersReader.readHistorical(
      this.options.fromBlockHeight,
      bestBlockHeight,
    );
  }
}

BlockHeadersProvider.defaultOptions = defaultOptions;

module.exports = BlockHeadersProvider;
