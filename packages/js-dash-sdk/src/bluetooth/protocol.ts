/**
 * Bluetooth protocol implementation for message encoding/decoding
 */

import { BluetoothMessage, BluetoothResponse, MessageType } from './types';

export class BluetoothProtocol {
  private static readonly PROTOCOL_VERSION = 1;
  private static readonly MAX_CHUNK_SIZE = 512; // BLE MTU limit

  /**
   * Encode a message for transmission
   */
  static encodeMessage(message: BluetoothMessage): Uint8Array {
    const json = JSON.stringify({
      v: this.PROTOCOL_VERSION,
      ...message
    });
    
    return new TextEncoder().encode(json);
  }

  /**
   * Decode a received message
   */
  static decodeMessage(data: Uint8Array): BluetoothMessage {
    const json = new TextDecoder().decode(data);
    const parsed = JSON.parse(json);
    
    if (parsed.v !== this.PROTOCOL_VERSION) {
      throw new Error(`Unsupported protocol version: ${parsed.v}`);
    }
    
    return {
      id: parsed.id,
      type: parsed.type,
      payload: parsed.payload,
      timestamp: parsed.timestamp,
      signature: parsed.signature
    };
  }

  /**
   * Encode a response for transmission
   */
  static encodeResponse(response: BluetoothResponse): Uint8Array {
    const json = JSON.stringify({
      v: this.PROTOCOL_VERSION,
      ...response
    });
    
    return new TextEncoder().encode(json);
  }

  /**
   * Decode a received response
   */
  static decodeResponse(data: Uint8Array): BluetoothResponse {
    const json = new TextDecoder().decode(data);
    const parsed = JSON.parse(json);
    
    if (parsed.v !== this.PROTOCOL_VERSION) {
      throw new Error(`Unsupported protocol version: ${parsed.v}`);
    }
    
    return {
      id: parsed.id,
      type: parsed.type,
      success: parsed.success,
      data: parsed.data,
      error: parsed.error,
      timestamp: parsed.timestamp
    };
  }

  /**
   * Split data into chunks for BLE transmission
   */
  static createChunks(data: Uint8Array): Uint8Array[] {
    const chunks: Uint8Array[] = [];
    const totalChunks = Math.ceil(data.length / this.MAX_CHUNK_SIZE);
    
    for (let i = 0; i < totalChunks; i++) {
      const start = i * this.MAX_CHUNK_SIZE;
      const end = Math.min(start + this.MAX_CHUNK_SIZE, data.length);
      
      // Add header: [chunk_index, total_chunks, ...data]
      const chunk = new Uint8Array(end - start + 2);
      chunk[0] = i;
      chunk[1] = totalChunks;
      chunk.set(data.slice(start, end), 2);
      
      chunks.push(chunk);
    }
    
    return chunks;
  }

  /**
   * Reassemble chunks into complete data
   */
  static assembleChunks(chunks: Map<number, Uint8Array>): Uint8Array | null {
    if (chunks.size === 0) return null;
    
    // Get total chunks from first chunk header
    const firstChunk = chunks.get(0);
    if (!firstChunk) return null;
    
    const totalChunks = firstChunk[1];
    
    // Check if we have all chunks
    if (chunks.size !== totalChunks) return null;
    
    // Calculate total size
    let totalSize = 0;
    for (let i = 0; i < totalChunks; i++) {
      const chunk = chunks.get(i);
      if (!chunk) return null;
      totalSize += chunk.length - 2; // Subtract header size
    }
    
    // Assemble data
    const assembled = new Uint8Array(totalSize);
    let offset = 0;
    
    for (let i = 0; i < totalChunks; i++) {
      const chunk = chunks.get(i)!;
      const data = chunk.slice(2); // Skip header
      assembled.set(data, offset);
      offset += data.length;
    }
    
    return assembled;
  }

  /**
   * Create a request message
   */
  static createRequest(type: MessageType, payload?: any): BluetoothMessage {
    return {
      id: this.generateMessageId(),
      type,
      payload,
      timestamp: Date.now()
    };
  }

  /**
   * Create a success response
   */
  static createSuccessResponse(
    requestId: string, 
    type: MessageType, 
    data?: any
  ): BluetoothResponse {
    return {
      id: requestId,
      type,
      success: true,
      data,
      timestamp: Date.now()
    };
  }

  /**
   * Create an error response
   */
  static createErrorResponse(
    requestId: string,
    type: MessageType,
    code: string,
    message: string
  ): BluetoothResponse {
    return {
      id: requestId,
      type,
      success: false,
      error: { code, message },
      timestamp: Date.now()
    };
  }

  /**
   * Generate a unique message ID
   */
  private static generateMessageId(): string {
    return `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
  }

  /**
   * Validate message format
   */
  static validateMessage(message: any): message is BluetoothMessage {
    return (
      typeof message === 'object' &&
      typeof message.id === 'string' &&
      typeof message.type === 'string' &&
      typeof message.timestamp === 'number' &&
      Object.values(MessageType).includes(message.type as MessageType)
    );
  }

  /**
   * Validate response format
   */
  static validateResponse(response: any): response is BluetoothResponse {
    return (
      typeof response === 'object' &&
      typeof response.id === 'string' &&
      typeof response.type === 'string' &&
      typeof response.success === 'boolean' &&
      typeof response.timestamp === 'number' &&
      Object.values(MessageType).includes(response.type as MessageType)
    );
  }
}