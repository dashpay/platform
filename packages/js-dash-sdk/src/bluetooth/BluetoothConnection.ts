/**
 * Bluetooth connection management
 */

import { EventEmitter } from 'eventemitter3';
import { 
  BluetoothConnectionOptions,
  BluetoothDeviceInfo,
  BluetoothEvents,
  BluetoothMessage,
  BluetoothResponse,
  DASH_BLUETOOTH_SERVICE_UUID,
  COMMAND_CHARACTERISTIC_UUID,
  RESPONSE_CHARACTERISTIC_UUID,
  STATUS_CHARACTERISTIC_UUID,
  MessageType
} from './types';
import { BluetoothProtocol } from './protocol';

export class BluetoothConnection extends EventEmitter<BluetoothEvents> {
  private device: BluetoothDevice | null = null;
  private server: BluetoothRemoteGATTServer | null = null;
  private service: BluetoothRemoteGATTService | null = null;
  
  private commandChar: BluetoothRemoteGATTCharacteristic | null = null;
  private responseChar: BluetoothRemoteGATTCharacteristic | null = null;
  private statusChar: BluetoothRemoteGATTCharacteristic | null = null;
  
  private pendingRequests = new Map<string, {
    resolve: (response: BluetoothResponse) => void;
    reject: (error: Error) => void;
    timeout: NodeJS.Timeout;
  }>();
  
  private receiveChunks = new Map<number, Uint8Array>();
  private connected = false;
  private authenticated = false;
  
  constructor(private options: BluetoothConnectionOptions = {}) {
    super();
    this.options = {
      timeout: 30000,
      retries: 3,
      requireAuthentication: true,
      ...options
    };
  }

  /**
   * Check if Web Bluetooth is available
   */
  static isAvailable(): boolean {
    return 'bluetooth' in navigator;
  }

  /**
   * Discover and connect to a Dash wallet device
   */
  async discover(): Promise<BluetoothDeviceInfo[]> {
    if (!BluetoothConnection.isAvailable()) {
      throw new Error('Web Bluetooth is not available in this browser');
    }

    try {
      // Request device with Dash service
      const device = await navigator.bluetooth.requestDevice({
        filters: [
          { services: [DASH_BLUETOOTH_SERVICE_UUID] }
        ],
        optionalServices: [DASH_BLUETOOTH_SERVICE_UUID]
      });

      // Connect to the device
      await this.connect(device);

      return [{
        id: device.id,
        name: device.name || 'Dash Wallet',
        paired: true,
        authenticated: false
      }];
    } catch (error: any) {
      throw new Error(`Device discovery failed: ${error.message}`);
    }
  }

  /**
   * Connect to a specific device
   */
  async connect(device: BluetoothDevice): Promise<void> {
    try {
      this.device = device;
      
      // Add disconnect listener
      device.addEventListener('gattserverdisconnected', () => {
        this.handleDisconnect();
      });

      // Connect to GATT server
      console.log('Connecting to GATT server...');
      this.server = await device.gatt!.connect();
      
      // Get the Dash service
      console.log('Getting Dash service...');
      this.service = await this.server.getPrimaryService(DASH_BLUETOOTH_SERVICE_UUID);
      
      // Get characteristics
      console.log('Getting characteristics...');
      this.commandChar = await this.service.getCharacteristic(COMMAND_CHARACTERISTIC_UUID);
      this.responseChar = await this.service.getCharacteristic(RESPONSE_CHARACTERISTIC_UUID);
      this.statusChar = await this.service.getCharacteristic(STATUS_CHARACTERISTIC_UUID);
      
      // Subscribe to responses
      await this.responseChar.startNotifications();
      this.responseChar.addEventListener('characteristicvaluechanged', (event) => {
        this.handleResponse(event);
      });
      
      // Subscribe to status updates
      await this.statusChar.startNotifications();
      this.statusChar.addEventListener('characteristicvaluechanged', (event) => {
        this.handleStatusUpdate(event);
      });
      
      this.connected = true;
      
      // Emit connected event
      this.emit('connected', {
        id: device.id,
        name: device.name || 'Dash Wallet',
        paired: true,
        authenticated: false
      });
      
      // Authenticate if required
      if (this.options.requireAuthentication) {
        await this.authenticate();
      }
    } catch (error: any) {
      this.connected = false;
      throw new Error(`Connection failed: ${error.message}`);
    }
  }

  /**
   * Disconnect from the device
   */
  async disconnect(): Promise<void> {
    if (this.server && this.server.connected) {
      this.server.disconnect();
    }
    this.handleDisconnect();
  }

  /**
   * Send a request to the device
   */
  async sendRequest(message: BluetoothMessage): Promise<BluetoothResponse> {
    if (!this.connected) {
      throw new Error('Not connected to device');
    }
    
    if (this.options.requireAuthentication && !this.authenticated) {
      throw new Error('Not authenticated');
    }

    return new Promise((resolve, reject) => {
      // Set timeout
      const timeout = setTimeout(() => {
        this.pendingRequests.delete(message.id);
        reject(new Error(`Request timeout: ${message.type}`));
      }, this.options.timeout!);
      
      // Store pending request
      this.pendingRequests.set(message.id, { resolve, reject, timeout });
      
      // Send message
      this.sendMessage(message).catch((error) => {
        this.pendingRequests.delete(message.id);
        clearTimeout(timeout);
        reject(error);
      });
    });
  }

  /**
   * Check if connected
   */
  isConnected(): boolean {
    return this.connected;
  }

  /**
   * Check if authenticated
   */
  isAuthenticated(): boolean {
    return this.authenticated;
  }

  /**
   * Get device info
   */
  getDeviceInfo(): BluetoothDeviceInfo | null {
    if (!this.device) return null;
    
    return {
      id: this.device.id,
      name: this.device.name || 'Dash Wallet',
      paired: true,
      authenticated: this.authenticated
    };
  }

  /**
   * Send a message to the device
   */
  private async sendMessage(message: BluetoothMessage): Promise<void> {
    if (!this.commandChar) {
      throw new Error('Command characteristic not available');
    }

    // Encode message
    const encoded = BluetoothProtocol.encodeMessage(message);
    
    // Split into chunks if needed
    const chunks = BluetoothProtocol.createChunks(encoded);
    
    // Send each chunk
    for (const chunk of chunks) {
      await this.commandChar.writeValueWithResponse(chunk);
      // Small delay between chunks
      await new Promise(resolve => setTimeout(resolve, 50));
    }
    
    this.emit('message', message);
  }

  /**
   * Handle response from device
   */
  private handleResponse(event: Event): void {
    const target = event.target as BluetoothRemoteGATTCharacteristic;
    if (!target.value) return;
    
    const chunk = new Uint8Array(target.value.buffer);
    const chunkIndex = chunk[0];
    const totalChunks = chunk[1];
    
    // Store chunk
    this.receiveChunks.set(chunkIndex, chunk);
    
    // Check if we have all chunks
    if (this.receiveChunks.size === totalChunks) {
      // Assemble message
      const assembled = BluetoothProtocol.assembleChunks(this.receiveChunks);
      if (assembled) {
        try {
          const response = BluetoothProtocol.decodeResponse(assembled);
          this.emit('response', response);
          
          // Handle pending request
          const pending = this.pendingRequests.get(response.id);
          if (pending) {
            clearTimeout(pending.timeout);
            this.pendingRequests.delete(response.id);
            pending.resolve(response);
          }
        } catch (error: any) {
          console.error('Failed to decode response:', error);
          this.emit('error', error);
        }
      }
      
      // Clear chunks
      this.receiveChunks.clear();
    }
  }

  /**
   * Handle status updates
   */
  private handleStatusUpdate(event: Event): void {
    const target = event.target as BluetoothRemoteGATTCharacteristic;
    if (!target.value) return;
    
    try {
      const status = new TextDecoder().decode(target.value);
      const parsed = JSON.parse(status);
      
      if (parsed.authenticated !== undefined) {
        this.authenticated = parsed.authenticated;
        if (this.authenticated) {
          this.emit('authenticated', this.getDeviceInfo()!);
        }
      }
    } catch (error) {
      console.error('Failed to parse status update:', error);
    }
  }

  /**
   * Handle disconnect
   */
  private handleDisconnect(): void {
    this.connected = false;
    this.authenticated = false;
    
    // Clean up pending requests
    for (const [id, pending] of this.pendingRequests) {
      clearTimeout(pending.timeout);
      pending.reject(new Error('Disconnected'));
    }
    this.pendingRequests.clear();
    
    // Clear state
    this.device = null;
    this.server = null;
    this.service = null;
    this.commandChar = null;
    this.responseChar = null;
    this.statusChar = null;
    this.receiveChunks.clear();
    
    this.emit('disconnected');
  }

  /**
   * Authenticate with the device
   */
  private async authenticate(): Promise<void> {
    // Send auth challenge
    const challenge = crypto.getRandomValues(new Uint8Array(32));
    const request = BluetoothProtocol.createRequest(
      MessageType.AUTH_CHALLENGE,
      { challenge: Array.from(challenge) }
    );
    
    const response = await this.sendRequest(request);
    
    if (!response.success) {
      throw new Error('Authentication failed');
    }
    
    this.authenticated = true;
  }
}