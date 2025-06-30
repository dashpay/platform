import { BluetoothProtocol } from '../../src/bluetooth/protocol';
import { MessageType } from '../../src/bluetooth/types';

describe('BluetoothProtocol', () => {
  describe('message encoding/decoding', () => {
    it('should encode and decode messages correctly', () => {
      const message = BluetoothProtocol.createRequest(
        MessageType.GET_BLOCK_HEIGHT,
        { test: 'data' }
      );

      const encoded = BluetoothProtocol.encodeMessage(message);
      const decoded = BluetoothProtocol.decodeMessage(encoded);

      expect(decoded.id).toBe(message.id);
      expect(decoded.type).toBe(message.type);
      expect(decoded.payload).toEqual(message.payload);
      expect(decoded.timestamp).toBe(message.timestamp);
    });

    it('should handle empty payload', () => {
      const message = BluetoothProtocol.createRequest(MessageType.PING);
      
      const encoded = BluetoothProtocol.encodeMessage(message);
      const decoded = BluetoothProtocol.decodeMessage(encoded);

      expect(decoded.type).toBe(MessageType.PING);
      expect(decoded.payload).toBeUndefined();
    });

    it('should reject unsupported protocol version', () => {
      const badData = new TextEncoder().encode(JSON.stringify({
        v: 999,
        id: 'test',
        type: MessageType.PING,
        timestamp: Date.now()
      }));

      expect(() => BluetoothProtocol.decodeMessage(badData))
        .toThrow('Unsupported protocol version: 999');
    });
  });

  describe('response encoding/decoding', () => {
    it('should encode and decode success responses', () => {
      const response = BluetoothProtocol.createSuccessResponse(
        'request-123',
        MessageType.GET_BLOCK_HEIGHT,
        { height: 123456 }
      );

      const encoded = BluetoothProtocol.encodeResponse(response);
      const decoded = BluetoothProtocol.decodeResponse(encoded);

      expect(decoded.id).toBe('request-123');
      expect(decoded.success).toBe(true);
      expect(decoded.data).toEqual({ height: 123456 });
      expect(decoded.error).toBeUndefined();
    });

    it('should encode and decode error responses', () => {
      const response = BluetoothProtocol.createErrorResponse(
        'request-123',
        MessageType.SIGN_STATE_TRANSITION,
        'SIGNING_FAILED',
        'Invalid key index'
      );

      const encoded = BluetoothProtocol.encodeResponse(response);
      const decoded = BluetoothProtocol.decodeResponse(encoded);

      expect(decoded.id).toBe('request-123');
      expect(decoded.success).toBe(false);
      expect(decoded.error).toEqual({
        code: 'SIGNING_FAILED',
        message: 'Invalid key index'
      });
      expect(decoded.data).toBeUndefined();
    });
  });

  describe('chunking', () => {
    it('should split large data into chunks', () => {
      const largeData = new Uint8Array(1500);
      largeData.fill(42);

      const chunks = BluetoothProtocol.createChunks(largeData);

      expect(chunks.length).toBe(3); // 512 bytes per chunk
      expect(chunks[0][0]).toBe(0); // First chunk index
      expect(chunks[0][1]).toBe(3); // Total chunks
      expect(chunks[2][0]).toBe(2); // Last chunk index
    });

    it('should reassemble chunks correctly', () => {
      const originalData = new Uint8Array(1000);
      for (let i = 0; i < originalData.length; i++) {
        originalData[i] = i % 256;
      }

      const chunks = BluetoothProtocol.createChunks(originalData);
      
      // Simulate receiving chunks
      const chunkMap = new Map<number, Uint8Array>();
      chunks.forEach((chunk, index) => {
        chunkMap.set(index, chunk);
      });

      const assembled = BluetoothProtocol.assembleChunks(chunkMap);
      expect(assembled).toEqual(originalData);
    });

    it('should return null for incomplete chunks', () => {
      const chunks = BluetoothProtocol.createChunks(new Uint8Array(1000));
      
      const chunkMap = new Map<number, Uint8Array>();
      chunkMap.set(0, chunks[0]);
      // Missing chunk 1

      const assembled = BluetoothProtocol.assembleChunks(chunkMap);
      expect(assembled).toBeNull();
    });
  });

  describe('validation', () => {
    it('should validate correct message format', () => {
      const validMessage = {
        id: 'test-123',
        type: MessageType.GET_BLOCK_HEIGHT,
        timestamp: Date.now(),
        payload: { test: true }
      };

      expect(BluetoothProtocol.validateMessage(validMessage)).toBe(true);
    });

    it('should reject invalid message format', () => {
      const invalidMessages = [
        { id: 123, type: MessageType.PING, timestamp: Date.now() }, // Wrong id type
        { id: 'test', type: 'INVALID_TYPE', timestamp: Date.now() }, // Invalid type
        { id: 'test', type: MessageType.PING }, // Missing timestamp
        null,
        undefined,
        'not an object'
      ];

      invalidMessages.forEach(msg => {
        expect(BluetoothProtocol.validateMessage(msg)).toBe(false);
      });
    });

    it('should validate correct response format', () => {
      const validResponse = {
        id: 'test-123',
        type: MessageType.PONG,
        success: true,
        timestamp: Date.now(),
        data: { result: 'ok' }
      };

      expect(BluetoothProtocol.validateResponse(validResponse)).toBe(true);
    });
  });

  describe('message creation helpers', () => {
    it('should generate unique message IDs', () => {
      const messages = Array.from({ length: 100 }, () => 
        BluetoothProtocol.createRequest(MessageType.PING)
      );

      const ids = messages.map(m => m.id);
      const uniqueIds = new Set(ids);

      expect(uniqueIds.size).toBe(100);
    });

    it('should create requests with correct structure', () => {
      const request = BluetoothProtocol.createRequest(
        MessageType.GET_ADDRESSES,
        { accountIndex: 0 }
      );

      expect(request.type).toBe(MessageType.GET_ADDRESSES);
      expect(request.payload).toEqual({ accountIndex: 0 });
      expect(request.timestamp).toBeCloseTo(Date.now(), -2);
      expect(request.id).toMatch(/^\d+-[a-z0-9]+$/);
    });
  });
});