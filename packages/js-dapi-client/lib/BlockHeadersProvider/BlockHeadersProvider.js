const EventEmitter = require('events');
const { SpvChain, SPVError } = require('@dashevo/dash-spv');

const BlockHeadersReader = require('./BlockHeadersReader');
const DAPIStream = require('../transport/DAPIStream');
// const DAPIStream = require('../transport/DAPIStream');

/**
 * @typedef {BlockHeadersProviderOptions} BlockHeadersProviderOptions
 * @property {string} [network=testnet]
 * @property {number} [maxParallelStreams=5] max parallel streams to read historical block headers
 * @property {number} [targetBatchSize=100000] a target batch size per stream
 * @property {number} [maxRetries=10] max amount of retries per stream connection
 */
const defaultOptions = {
  network: 'testnet',
  maxParallelStreams: 10,
  targetBatchSize: 50000,
  fromBlockHeight: 1,
  maxRetries: 10,
  spvChain: null,
};

const EVENTS = {
  ERROR: 'error',
  CHAIN_UPDATED: 'CHAIN_UPDATED',
  HISTORICAL_DATA_OBTAINED: 'HISTORICAL_DATA_OBTAINED',
  STOPPED: 'STOPPED',
};

const STATES = {
  IDLE: 'IDLE',
  HISTORICAL_SYNC: 'HISTORICAL_SYNC',
  CONTINUOUS_SYNC: 'CONTINUOUS_SYNC',
};

class BlockHeadersProvider extends EventEmitter {
  /**
   * @param {BlockHeadersProviderOptions} options
   */
  constructor(options = {}) {
    super();
    this.options = {
      ...defaultOptions,
      ...options,
    };

    // TODO: make sure chain properly maintains it's integrity if confirms is more than chain length
    this.spvChain = this.options.spvChain || new SpvChain(this.options.network, 100);

    this.state = STATES.IDLE;

    this.handleError = this.handleError.bind(this);
    this.handleHeaders = this.handleHeaders.bind(this);
  }

  /**
   * @param {CoreMethodsFacade} coreMethods
   */
  setCoreMethods(coreMethods) {
    this.coreMethods = coreMethods;
  }

  /**
   * @param {BlockHeadersReader} blockHeadersReader
   */
  setBlockHeadersReader(blockHeadersReader) {
    this.blockHeadersReader = blockHeadersReader;
  }

  /**
   *
   * @param {SpvChain} spvChain
   */
  setSpvChain(spvChain) {
    this.spvChain = spvChain;
  }

  /**
   * @private
   */
  initReader() {
    if (!this.blockHeadersReader) {
      const createContinuousSyncStream = (fromBlockHeight) => DAPIStream
        .create(
          this.coreMethods.subscribeToBlockHeadersWithChainLocks,
        )({
          fromBlockHeight,
        });

      const createHistoricalSyncStream = (fromBlockHeight, count) => {
        const { subscribeToBlockHeadersWithChainLocks } = this.coreMethods;
        return subscribeToBlockHeadersWithChainLocks({
          fromBlockHeight,
          count,
        });
      };

      this.blockHeadersReader = new BlockHeadersReader(
        {
          coreMethods: this.coreMethods,
          maxParallelStreams: this.options.maxParallelStreams,
          targetBatchSize: this.options.targetBatchSize,
          maxRetries: this.options.maxRetries,
          createContinuousSyncStream,
          createHistoricalSyncStream,
        },
      );
    }

    this.blockHeadersReader.on(BlockHeadersReader.EVENTS.BLOCK_HEADERS, this.handleHeaders);
    this.blockHeadersReader.on(BlockHeadersReader.EVENTS.ERROR, this.handleError);
  }

  destroyReader() {
    this.removeReaderListeners();
    this.blockHeadersReader = null;
  }

  /**
   * Checks whether spv chain has header at specified height and flushes chains if not
   *
   * @private
   * @param height
   */
  ensureChainRoot(height) {
    // Flush spv chain in case header at specified height was not found
    if (!this.spvChain.hashesByHeight[height - 1]) {
      this.spvChain.reset(height);
    }
  }

  /**
   * Reads historical block headers
   *
   * @param fromBlockHeight
   * @param toBlockHeight
   * @returns {Promise<void>}
   */
  async readHistorical(fromBlockHeight = 1, toBlockHeight) {
    if (!this.coreMethods) {
      throw new Error('Core methods have not been provided. Please use "setCoreMethods"');
    }

    if (this.state !== STATES.IDLE) {
      throw new Error(`BlockHeaderProvider can not read historical data while being in ${this.state} state.`);
    }

    this.initReader();

    this.ensureChainRoot(fromBlockHeight);

    this.blockHeadersReader.once(BlockHeadersReader.EVENTS.HISTORICAL_DATA_OBTAINED, () => {
      this.emit(EVENTS.HISTORICAL_DATA_OBTAINED);
      // TODO: implement spvChain.validate() to ensure that chain is complete
      // and all metadata is present
      this.removeReaderListeners();

      this.state = STATES.IDLE;
    });

    await this.blockHeadersReader.readHistorical(
      fromBlockHeight,
      toBlockHeight,
    );

    this.state = STATES.HISTORICAL_SYNC;
  }

  async startContinuousSync(fromBlockHeight) {
    if (!this.coreMethods) {
      throw new Error('Core methods have not been provided. Please use "setCoreMethods"');
    }

    if (this.state !== STATES.IDLE) {
      throw new Error(`BlockHeaderProvider can not sync continuous data while being in ${this.state} state.`);
    }

    this.initReader();
    this.ensureChainRoot(fromBlockHeight);
    await this.blockHeadersReader.subscribeToNew(fromBlockHeight);
    this.state = STATES.CONTINUOUS_SYNC;
  }

  async stop() {
    if (this.state === STATES.IDLE) {
      return;
    }

    if (this.state === STATES.HISTORICAL_SYNC) {
      this.blockHeadersReader.stopReadingHistorical();
    } else if (this.state === STATES.CONTINUOUS_SYNC) {
      this.blockHeadersReader.unsubscribeFromNew();
    }

    this.destroyReader();

    this.state = STATES.IDLE;

    this.emit(EVENTS.STOPPED);
  }

  handleError(e) {
    this.destroyReader();
    this.emit(EVENTS.ERROR, e);
  }

  /**
   * @private
   * @param headersData
   * @param reject
   */
  handleHeaders(headersData, reject) {
    const { headers, headHeight } = headersData;

    try {
      const headersAdded = this.spvChain.addHeaders(headers, headHeight);

      if (headersAdded.length) {
        // Calculate amount of removed headers in order to properly adjust head height
        const difference = headers.length - headersAdded.length;
        this.emit(EVENTS.CHAIN_UPDATED, headersAdded, headHeight + difference);
      }
    } catch (e) {
      if (e instanceof SPVError) {
        reject(e);
      } else {
        this.handleError(e);
      }
    }
  }

  removeReaderListeners() {
    this.blockHeadersReader.removeListener(BlockHeadersReader.EVENTS.ERROR, this.handleError);
    this.blockHeadersReader
      .removeListener(BlockHeadersReader.EVENTS.BLOCK_HEADERS, this.handleHeaders);
    this.blockHeadersReader
      .removeAllListeners(BlockHeadersReader.EVENTS.HISTORICAL_DATA_OBTAINED);
  }
}

BlockHeadersProvider.EVENTS = EVENTS;
BlockHeadersProvider.STATES = STATES;
BlockHeadersProvider.defaultOptions = defaultOptions;

module.exports = BlockHeadersProvider;
