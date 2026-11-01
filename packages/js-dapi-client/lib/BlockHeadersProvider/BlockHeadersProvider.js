const EventEmitter = require('events');
const { SpvChain, SPVError } = require('@dashevo/dash-spv');

const BlockHeadersReader = require('./BlockHeadersReader');

/**
 * @typedef {BlockHeadersProviderOptions} BlockHeadersProviderOptions
 * @property {string} [network='testnet'] network type
 * @property {number} [maxParallelStreams=10] max parallel streams to read historical block headers
 * @property {number} [targetBatchSize=10000] a target batch size per stream
 * @property {number} [fromBlockHeight=1] a target batch size per stream
 * @property {number} [maxRetries=10] max amount of retries per stream connection
 * @property {number} [powTargetSpacing] devnets only: the -powtargetspacing the
 *   devnet was started with
 * @property {number} [minimumDifficultyBlocks] devnets only: the
 *   -minimumdifficultyblocks the devnet was started with
 */
const defaultOptions = {
  network: 'testnet',
  maxParallelStreams: 10,
  // Headers read ahead of the chain wait in memory, up to about
  // maxParallelStreams * targetBatchSize of them
  targetBatchSize: 10000,
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
   * @param {BlockHeadersProviderOptions} options for block headers
   * @param {Function} [createHistoricalSyncStream] createHistoricalSyncStream
   * @param {Function} [createContinuousSyncStream] createContinuousSyncStream
   */
  // TODO move options to as last param
  // eslint-disable-next-line default-param-last
  constructor(options = {}, createHistoricalSyncStream, createContinuousSyncStream) {
    super();
    this.options = {
      ...defaultOptions,
      ...options,
    };

    this.state = STATES.IDLE;

    // How far below the tip continuous sync asks for headers again when new
    // ones don't connect: DAPI streams only each new tip, so after a reorg or
    // several blocks at once the parents have to be fetched by height
    this.continuousLookback = 1;

    this.errorHandler = this.errorHandler.bind(this);
    this.headersHandler = this.headersHandler.bind(this);
    this.chainConnectedHandler = this.chainConnectedHandler.bind(this);
    this.historicalDataObtainedHandler = this.historicalDataObtainedHandler.bind(this);
    this.createHistoricalSyncStream = createHistoricalSyncStream;
    this.createContinuousSyncStream = createContinuousSyncStream;

    const { powTargetSpacing, minimumDifficultyBlocks } = this.options;
    const devnetOptions = powTargetSpacing !== undefined || minimumDifficultyBlocks !== undefined
      ? { powTargetSpacing, minimumDifficultyBlocks }
      : undefined;
    this.setSpvChain(new SpvChain(this.options.network, 100, devnetOptions));
  }

  /**
   * @param {BlockHeadersReader} blockHeadersReader instance
   */
  setBlockHeadersReader(blockHeadersReader) {
    this.blockHeadersReader = blockHeadersReader;
  }

  /**
   *
   * @param {SpvChain} spvChain instance
   */
  setSpvChain(spvChain) {
    if (this.spvChain) {
      this.spvChain.removeListener(SpvChain.EVENTS.CONNECTED, this.chainConnectedHandler);
    }
    this.spvChain = spvChain;
    this.spvChain.on(SpvChain.EVENTS.CONNECTED, this.chainConnectedHandler);
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
          getConnectedHeight: () => (this.spvChain.startBlockHeight === null
            ? -Infinity
            : this.spvChain.getTipHeight()),
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
   * @param {BlockHeader[]} headers array of headers
   * @param firstHeaderHeight {number} first block header height
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
   * @param {number} height block height
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
   * @param {number} fromBlockHeight height block height
   * @param {number} toBlockHeight height block height
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

    try {
      await this.blockHeadersReader.readHistorical(
        fromBlockHeight,
        toBlockHeight,
      );
    } catch (e) {
      // init() adds them again on the next attempt
      this.removeReaderListeners();
      throw e;
    }

    this.state = STATES.HISTORICAL_SYNC;
    this.emit(EVENTS.HISTORICAL_SYNC_STARTED);
  }

  async startContinuousSync(fromBlockHeight) {
    if (this.state !== STATES.IDLE) {
      throw new Error(`BlockHeaderProvider can not sync continuous data while being in ${this.state} state.`);
    }

    await this.init();
    this.ensureChainRoot(fromBlockHeight);
    this.continuousLookback = 1;
    this.continuousGapReported = false;
    try {
      await this.blockHeadersReader.subscribeToNew(
        fromBlockHeight,
        () => this.getContinuousResumeHeight(),
      );
    } catch (e) {
      // init() adds them again on the next attempt
      this.removeReaderListeners();
      throw e;
    }
    this.state = STATES.CONTINUOUS_SYNC;
    this.emit(EVENTS.CONTINUOUS_SYNC_STARTED);
  }

  async stop() {
    if (this.state === STATES.IDLE) {
      return;
    }

    // errorHandler() already destroyed the reader, whose streams stopped
    // before it reported the error
    if (this.blockHeadersReader) {
      if (this.state === STATES.HISTORICAL_SYNC) {
        this.blockHeadersReader.stopReadingHistorical();
      } else if (this.state === STATES.CONTINUOUS_SYNC) {
        this.blockHeadersReader.unsubscribeFromNew();
      }

      this.destroyReader();
    }

    this.state = STATES.IDLE;

    this.emit(EVENTS.STOPPED);
  }

  errorHandler(e) {
    this.destroyReader();
    this.emit(EVENTS.ERROR, e);
  }

  /**
   * @private
   * @param {object} headersData object with header and headHeight
   * @param {function} reject callback function
   */
  headersHandler(headersData, reject) {
    const { headers, headHeight } = headersData;

    // TODO: simulate headers rejection and debug
    // BlockHeadersReader might not correctly set startBlockHeight
    // which results in orphan chunks in SPV

    let rejection = null;
    let gapError = null;
    try {
      // Headers that reach the main chain come back through chainConnectedHandler
      this.spvChain.addHeaders(headers, headHeight);

      if (this.state === STATES.CONTINUOUS_SYNC && this.spvChain.getOrphanChunks().length) {
        // Their parents were never sent: ask again from further below the tip
        this.spvChain.dropOrphans();
        if (this.isAtMaxLookback()) {
          gapError = new SPVError(`New headers don't connect to the chain even ${this.spvChain.confirmsBeforeFinal + 1} headers below its tip`);
        }
        this.continuousLookback = Math.min(
          this.continuousLookback * 2,
          this.spvChain.confirmsBeforeFinal + 1,
        );
        rejection = new SPVError(`New headers don't connect to the chain, requesting them again from height ${this.getContinuousResumeHeight()}`);
      }
    } catch (e) {
      if (!(e instanceof SPVError)) {
        this.errorHandler(e);
        return;
      }
      rejection = e;
      if (this.state === STATES.CONTINUOUS_SYNC && this.isAtMaxLookback()) {
        // Such as a reorg that would replace confirmed headers
        gapError = e;
      }
    }

    // Retry once, then report, outside the validation above
    if (rejection) {
      reject(rejection);
    }
    if (gapError) {
      this.reportContinuousGap(gapError);
    }
  }

  /**
   * @private
   * @param {BlockHeader[]} headers - the main chain from firstHeight on;
   *   after a reorg firstHeight is at or below the previous tip
   * @param {number} firstHeight
   */
  chainConnectedHandler(headers, firstHeight) {
    if (this.blockHeadersReader && typeof this.blockHeadersReader.chainAdvanced === 'function') {
      this.blockHeadersReader.chainAdvanced();
    }
    this.continuousLookback = 1;
    this.continuousGapReported = false;
    this.emit(EVENTS.CHAIN_UPDATED, headers, firstHeight);
  }

  /**
   * @private
   * Continuous sync keeps retrying headers it can't add, but reports once,
   * after looking back as far as it can, that it can't get back on the chain
   * @param {SPVError} error
   */
  reportContinuousGap(error) {
    if (!this.continuousGapReported && this.listenerCount(EVENTS.ERROR) > 0) {
      this.continuousGapReported = true;
      this.emit(EVENTS.ERROR, error);
    }
  }

  /** @private */
  isAtMaxLookback() {
    return this.continuousLookback === this.spvChain.confirmsBeforeFinal + 1;
  }

  /**
   * @private
   * @return {number}
   */
  getContinuousResumeHeight() {
    if (this.spvChain.startBlockHeight === null) {
      // No headers yet: wait for the first one where sync was asked to start
      return this.spvChain.pendingStartBlockHeight || 1;
    }
    return Math.max(1, this.spvChain.getTipHeight() - this.continuousLookback + 1);
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
