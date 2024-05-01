const EventEmitter = require('events');
const { SpvChain, SPVError } = require('@dashevo/dash-spv');

const BlockHeadersReader = require('./BlockHeadersReader');

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
};

const EVENTS = {
  ERROR: 'error',
  HISTORICAL_SYNC_STARTED: 'HISTORICAL_SYNC_STARTED',
  CONTINUOUS_SYNC_STARTED: 'CONTINUOUS_SYNC_STARTED',
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
   * @param {Function} [createHistoricalSyncStream]
   * @param {Function} [createContinuousSyncStream]
   */
  // TODO move options to as last param
  // eslint-disable-next-line default-param-last
  constructor(options = {}, createHistoricalSyncStream, createContinuousSyncStream) {
    super();
    this.options = {
      ...defaultOptions,
      ...options,
    };

    this.spvChain = new SpvChain(this.options.network, 100);

    this.state = STATES.IDLE;

    this.errorHandler = this.errorHandler.bind(this);
    this.headersHandler = this.headersHandler.bind(this);
    this.historicalDataObtainedHandler = this.historicalDataObtainedHandler.bind(this);
    this.createHistoricalSyncStream = createHistoricalSyncStream;
    this.createContinuousSyncStream = createContinuousSyncStream;
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
  async init() {
    await SpvChain.wasmX11Ready();

    if (!this.blockHeadersReader) {
      this.blockHeadersReader = new BlockHeadersReader(
        {
          coreMethods: this.coreMethods,
          maxParallelStreams: this.options.maxParallelStreams,
          targetBatchSize: this.options.targetBatchSize,
          maxRetries: this.options.maxRetries,
        },
        this.createHistoricalSyncStream,
        this.createContinuousSyncStream,
      );
    }

    this.blockHeadersReader.on(BlockHeadersReader.EVENTS.BLOCK_HEADERS, this.headersHandler);
    this.blockHeadersReader.on(BlockHeadersReader.EVENTS.ERROR, this.errorHandler);
  }

  destroyReader() {
    this.removeReaderListeners();
    this.blockHeadersReader = null;
  }

  /**
   * Initializes SPV chain with a list of headers and a known lastSyncedHeaderHeight
   * @param headers
   * @param firstHeaderHeight
   */
  async initializeChainWith(headers, firstHeaderHeight) {
    await SpvChain.wasmX11Ready();

    if (headers.length === 0) {
      // If there are no headers, initialize chain from genesis
      this.spvChain.initialize();
    } else {
      this.spvChain.initialize(headers[0], firstHeaderHeight);
      this.spvChain.addHeaders(headers);
    }
  }

  /**
   * Checks whether spv chain has header at specified height and flushes chains if not
   * @private
   * @param height
   */
  ensureChainRoot(height) {
    // Flush spv chain in case header at specified height was not found
    if (!this.spvChain.hashesByHeight.has(height - 1)) {
      this.spvChain.reset();
      this.spvChain.pendingStartBlockHeight = height;
    }
  }

  /**
   * Reads historical block headers
   * @param fromBlockHeight
   * @param toBlockHeight
   * @returns {Promise<void>}
   */
  async readHistorical(fromBlockHeight, toBlockHeight) {
    if (this.state !== STATES.IDLE) {
      throw new Error(`BlockHeaderProvider can not read historical data while being in ${this.state} state.`);
    }

    await this.init();

    this.ensureChainRoot(fromBlockHeight);

    this.blockHeadersReader
      .once(BlockHeadersReader.EVENTS.HISTORICAL_DATA_OBTAINED, this.historicalDataObtainedHandler);

    await this.blockHeadersReader.readHistorical(
      fromBlockHeight,
      toBlockHeight,
    );

    this.state = STATES.HISTORICAL_SYNC;
    this.emit(EVENTS.HISTORICAL_SYNC_STARTED);
  }

  async startContinuousSync(fromBlockHeight) {
    if (this.state !== STATES.IDLE) {
      throw new Error(`BlockHeaderProvider can not sync continuous data while being in ${this.state} state.`);
    }

    await this.init();
    this.ensureChainRoot(fromBlockHeight);
    await this.blockHeadersReader.subscribeToNew(fromBlockHeight);
    this.state = STATES.CONTINUOUS_SYNC;
    this.emit(EVENTS.CONTINUOUS_SYNC_STARTED);
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

  errorHandler(e) {
    this.destroyReader();
    this.emit(EVENTS.ERROR, e);
  }

  /**
   * @private
   * @param headersData
   * @param reject
   */
  headersHandler(headersData, reject) {
    const { headers, headHeight } = headersData;

    // TODO: simulate headers rejection and debug
    // BlockHeadersReader might not correctly set startBlockHeight
    // which results in orphan chunks in SPV

    try {
      const headersAdded = this.spvChain.addHeaders(headers, headHeight);

      if (headersAdded.length) {
        // Calculate amount of removed headers in order to properly adjust head height
        // TODO(spv): move this logic to SpvChain?
        const difference = headers.length - headersAdded.length;
        this.emit(EVENTS.CHAIN_UPDATED, headersAdded, headHeight + difference);
      }
    } catch (e) {
      if (e instanceof SPVError) {
        reject(e);
      } else {
        this.errorHandler(e);
      }
    }
  }

  historicalDataObtainedHandler() {
    try {
      this.spvChain.validate();
      this.emit(EVENTS.HISTORICAL_DATA_OBTAINED);
      this.removeReaderListeners();
    } catch (e) {
      this.errorHandler(e);
    } finally {
      this.state = STATES.IDLE;
    }
  }

  removeReaderListeners() {
    this.blockHeadersReader.removeListener(BlockHeadersReader.EVENTS.ERROR, this.errorHandler);
    this.blockHeadersReader
      .removeListener(BlockHeadersReader.EVENTS.BLOCK_HEADERS, this.headersHandler);
    this.blockHeadersReader
      .removeListener(
        BlockHeadersReader.EVENTS.HISTORICAL_DATA_OBTAINED,
        this.historicalDataObtainedHandler,
      );
  }
}

BlockHeadersProvider.EVENTS = EVENTS;
BlockHeadersProvider.STATES = STATES;
BlockHeadersProvider.defaultOptions = { ...defaultOptions };

module.exports = BlockHeadersProvider;
