/**
 * Bluetooth-based context provider that gets platform state from a mobile device
 */

import { AbstractContextProvider } from '../core/ContextProvider';
import { BluetoothConnection } from './BluetoothConnection';
import { BluetoothProtocol } from './protocol';
import { MessageType, BluetoothConnectionOptions } from './types';

export interface BluetoothProviderOptions extends BluetoothConnectionOptions {
  autoReconnect?: boolean;
  reconnectDelay?: number;
}

export class BluetoothProvider extends AbstractContextProvider {
  private connection: BluetoothConnection;
  private options: BluetoothProviderOptions;
  private reconnectTimer?: NodeJS.Timeout;

  constructor(options: BluetoothProviderOptions = {}) {
    super();
    this.options = {
      autoReconnect: true,
      reconnectDelay: 5000,
      ...options
    };
    
    this.connection = new BluetoothConnection(options);
    this.setupEventHandlers();
  }

  /**
   * Connect to a mobile device
   */
  async connect(): Promise<void> {
    if (!BluetoothConnection.isAvailable()) {
      throw new Error('Bluetooth is not available in this browser');
    }

    await this.connection.discover();
  }

  /**
   * Disconnect from the device
   */
  async disconnect(): Promise<void> {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = undefined;
    }
    await this.connection.disconnect();
  }

  async getLatestPlatformBlockHeight(): Promise<number> {
    const cached = this.getCached<number>('blockHeight');
    if (cached !== null) return cached;

    const request = BluetoothProtocol.createRequest(MessageType.GET_BLOCK_HEIGHT);
    const response = await this.connection.sendRequest(request);

    if (!response.success) {
      throw new Error(`Failed to get block height: ${response.error?.message}`);
    }

    const height = response.data.height;
    this.setCache('blockHeight', height);
    return height;
  }

  async getLatestPlatformBlockTime(): Promise<number> {
    const cached = this.getCached<number>('blockTime');
    if (cached !== null) return cached;

    const request = BluetoothProtocol.createRequest(MessageType.GET_BLOCK_TIME);
    const response = await this.connection.sendRequest(request);

    if (!response.success) {
      throw new Error(`Failed to get block time: ${response.error?.message}`);
    }

    const time = response.data.time;
    this.setCache('blockTime', time);
    return time;
  }

  async getLatestPlatformCoreChainLockedHeight(): Promise<number> {
    const cached = this.getCached<number>('coreChainLockedHeight');
    if (cached !== null) return cached;

    const request = BluetoothProtocol.createRequest(MessageType.GET_CORE_CHAIN_LOCKED_HEIGHT);
    const response = await this.connection.sendRequest(request);

    if (!response.success) {
      throw new Error(`Failed to get core chain locked height: ${response.error?.message}`);
    }

    const height = response.data.height;
    this.setCache('coreChainLockedHeight', height);
    return height;
  }

  async getLatestPlatformVersion(): Promise<string> {
    const cached = this.getCached<string>('version');
    if (cached !== null) return cached;

    const request = BluetoothProtocol.createRequest(MessageType.GET_PLATFORM_VERSION);
    const response = await this.connection.sendRequest(request);

    if (!response.success) {
      throw new Error(`Failed to get platform version: ${response.error?.message}`);
    }

    const version = response.data.version;
    this.setCache('version', version);
    return version;
  }

  async getProposerBlockCount(proposerProTxHash: string): Promise<number | null> {
    const cacheKey = `proposerBlockCount:${proposerProTxHash}`;
    const cached = this.getCached<number>(cacheKey);
    if (cached !== null) return cached;

    const request = BluetoothProtocol.createRequest(
      MessageType.GET_PROPOSER_BLOCK_COUNT,
      { proposerProTxHash }
    );
    const response = await this.connection.sendRequest(request);

    if (!response.success) {
      return null;
    }

    const count = response.data.count;
    this.setCache(cacheKey, count);
    return count;
  }

  async getTimePerBlockMillis(): Promise<number> {
    const cached = this.getCached<number>('timePerBlock');
    if (cached !== null) return cached;

    const request = BluetoothProtocol.createRequest(MessageType.GET_TIME_PER_BLOCK);
    const response = await this.connection.sendRequest(request);

    if (!response.success) {
      throw new Error(`Failed to get time per block: ${response.error?.message}`);
    }

    const time = response.data.timePerBlock;
    this.setCache('timePerBlock', time);
    return time;
  }

  async getBlockProposer(blockHeight: number): Promise<string | null> {
    const cacheKey = `blockProposer:${blockHeight}`;
    const cached = this.getCached<string>(cacheKey);
    if (cached !== null) return cached;

    const request = BluetoothProtocol.createRequest(
      MessageType.GET_BLOCK_PROPOSER,
      { blockHeight }
    );
    const response = await this.connection.sendRequest(request);

    if (!response.success) {
      return null;
    }

    const proposer = response.data.proposer;
    this.setCache(cacheKey, proposer);
    return proposer;
  }

  async isValid(): Promise<boolean> {
    if (!this.connection.isConnected()) {
      return false;
    }

    try {
      // Send ping to check connection
      const request = BluetoothProtocol.createRequest(MessageType.PING);
      const response = await this.connection.sendRequest(request);
      return response.success && response.type === MessageType.PONG;
    } catch {
      return false;
    }
  }

  /**
   * Get all platform status in one request
   */
  async getPlatformStatus(): Promise<{
    blockHeight: number;
    blockTime: number;
    coreChainLockedHeight: number;
    version: string;
    timePerBlock: number;
  }> {
    const request = BluetoothProtocol.createRequest(MessageType.GET_PLATFORM_STATUS);
    const response = await this.connection.sendRequest(request);

    if (!response.success) {
      throw new Error(`Failed to get platform status: ${response.error?.message}`);
    }

    const status = response.data;
    
    // Cache all values
    this.setCache('blockHeight', status.blockHeight);
    this.setCache('blockTime', status.blockTime);
    this.setCache('coreChainLockedHeight', status.coreChainLockedHeight);
    this.setCache('version', status.version);
    this.setCache('timePerBlock', status.timePerBlock);

    return status;
  }

  /**
   * Get connection instance for wallet operations
   */
  getConnection(): BluetoothConnection {
    return this.connection;
  }

  private setupEventHandlers(): void {
    this.connection.on('disconnected', () => {
      // Clear cache on disconnect
      this.cache.clear();
      
      // Auto-reconnect if enabled
      if (this.options.autoReconnect) {
        this.scheduleReconnect();
      }
    });

    this.connection.on('error', (error) => {
      console.error('Bluetooth provider error:', error);
    });
  }

  private scheduleReconnect(): void {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
    }

    this.reconnectTimer = setTimeout(async () => {
      try {
        console.log('Attempting to reconnect...');
        await this.connect();
      } catch (error) {
        console.error('Reconnection failed:', error);
        // Schedule another attempt
        this.scheduleReconnect();
      }
    }, this.options.reconnectDelay);
  }
}