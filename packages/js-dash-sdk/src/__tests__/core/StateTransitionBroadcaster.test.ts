import { StateTransitionBroadcaster } from '../../core/StateTransitionBroadcaster';
import { ContextProvider, StateTransition } from '../../core/types';

describe('StateTransitionBroadcaster', () => {
  let broadcaster: StateTransitionBroadcaster;
  let mockProvider: jest.Mocked<ContextProvider>;
  
  const mockStateTransition: StateTransition = {
    toBuffer: jest.fn().mockReturnValue(Buffer.from('test-transition-data')),
  };

  beforeEach(() => {
    mockProvider = {
      getBlockHash: jest.fn(),
      getDataContract: jest.fn(),
      waitForStateTransitionResult: jest.fn(),
      broadcastStateTransition: jest.fn(),
      getProtocolVersion: jest.fn(),
    };
    
    broadcaster = new StateTransitionBroadcaster(mockProvider);
  });

  describe('broadcast', () => {
    it('should broadcast state transition successfully', async () => {
      const transitionHash = 'transition-hash-123';
      mockProvider.broadcastStateTransition.mockResolvedValue(transitionHash);

      const result = await broadcaster.broadcast(mockStateTransition);

      expect(mockProvider.broadcastStateTransition).toHaveBeenCalledWith(
        mockStateTransition
      );
      expect(result).toBe(transitionHash);
    });

    it('should handle broadcast failure', async () => {
      mockProvider.broadcastStateTransition.mockRejectedValue(
        new Error('Network error')
      );

      await expect(
        broadcaster.broadcast(mockStateTransition)
      ).rejects.toThrow('Network error');
    });

    it('should validate state transition before broadcast', async () => {
      const invalidTransition: StateTransition = {
        toBuffer: jest.fn().mockReturnValue(Buffer.alloc(0)), // Empty buffer
      };

      await expect(
        broadcaster.broadcast(invalidTransition)
      ).rejects.toThrow();
    });
  });

  describe('broadcastAndWait', () => {
    const transitionHash = 'transition-hash-456';
    const transitionResult = {
      hash: transitionHash,
      proved: true,
      result: 'success',
      data: {},
    };

    it('should broadcast and wait for result with proof', async () => {
      mockProvider.broadcastStateTransition.mockResolvedValue(transitionHash);
      mockProvider.waitForStateTransitionResult.mockResolvedValue(transitionResult);

      const result = await broadcaster.broadcastAndWait(mockStateTransition);

      expect(mockProvider.broadcastStateTransition).toHaveBeenCalledWith(
        mockStateTransition
      );
      expect(mockProvider.waitForStateTransitionResult).toHaveBeenCalledWith(
        transitionHash,
        true // Always use proved mode
      );
      expect(result).toEqual(transitionResult);
    });

    it('should always use proved mode even if specified false', async () => {
      mockProvider.broadcastStateTransition.mockResolvedValue(transitionHash);
      mockProvider.waitForStateTransitionResult.mockResolvedValue(transitionResult);

      await broadcaster.broadcastAndWait(mockStateTransition, false);

      // Should still use proved mode
      expect(mockProvider.waitForStateTransitionResult).toHaveBeenCalledWith(
        transitionHash,
        true
      );
    });

    it('should handle timeout while waiting', async () => {
      mockProvider.broadcastStateTransition.mockResolvedValue(transitionHash);
      mockProvider.waitForStateTransitionResult.mockImplementation(
        () => new Promise((resolve) => setTimeout(resolve, 60000))
      );

      const timeoutPromise = broadcaster.broadcastAndWait(
        mockStateTransition,
        true,
        5000 // 5 second timeout
      );

      await expect(timeoutPromise).rejects.toThrow('Timeout');
    });

    it('should handle broadcast success but wait failure', async () => {
      mockProvider.broadcastStateTransition.mockResolvedValue(transitionHash);
      mockProvider.waitForStateTransitionResult.mockRejectedValue(
        new Error('State transition failed')
      );

      await expect(
        broadcaster.broadcastAndWait(mockStateTransition)
      ).rejects.toThrow('State transition failed');

      // Verify broadcast was called
      expect(mockProvider.broadcastStateTransition).toHaveBeenCalled();
    });
  });

  describe('retry logic', () => {
    it('should retry on temporary failures', async () => {
      const transitionHash = 'retry-hash';
      
      // First two calls fail, third succeeds
      mockProvider.broadcastStateTransition
        .mockRejectedValueOnce(new Error('Network timeout'))
        .mockRejectedValueOnce(new Error('Network timeout'))
        .mockResolvedValueOnce(transitionHash);

      const result = await broadcaster.broadcast(
        mockStateTransition,
        { maxRetries: 3 }
      );

      expect(mockProvider.broadcastStateTransition).toHaveBeenCalledTimes(3);
      expect(result).toBe(transitionHash);
    });

    it('should not retry on permanent failures', async () => {
      mockProvider.broadcastStateTransition.mockRejectedValue(
        new Error('Invalid state transition')
      );

      await expect(
        broadcaster.broadcast(mockStateTransition, { maxRetries: 3 })
      ).rejects.toThrow('Invalid state transition');

      // Should only try once for permanent errors
      expect(mockProvider.broadcastStateTransition).toHaveBeenCalledTimes(1);
    });

    it('should apply exponential backoff between retries', async () => {
      const startTime = Date.now();
      
      mockProvider.broadcastStateTransition
        .mockRejectedValueOnce(new Error('Network timeout'))
        .mockRejectedValueOnce(new Error('Network timeout'))
        .mockResolvedValueOnce('success-hash');

      await broadcaster.broadcast(
        mockStateTransition,
        { maxRetries: 3, backoffMs: 100 }
      );

      const duration = Date.now() - startTime;
      
      // Should have delays between retries
      expect(duration).toBeGreaterThan(200); // At least 2 backoff periods
    });
  });

  describe('batch broadcasting', () => {
    it('should broadcast multiple transitions efficiently', async () => {
      const transitions = [
        { toBuffer: () => Buffer.from('transition-1') },
        { toBuffer: () => Buffer.from('transition-2') },
        { toBuffer: () => Buffer.from('transition-3') },
      ] as StateTransition[];

      const hashes = ['hash-1', 'hash-2', 'hash-3'];
      
      mockProvider.broadcastStateTransition
        .mockResolvedValueOnce(hashes[0])
        .mockResolvedValueOnce(hashes[1])
        .mockResolvedValueOnce(hashes[2]);

      const results = await broadcaster.broadcastBatch(transitions);

      expect(results).toEqual(hashes);
      expect(mockProvider.broadcastStateTransition).toHaveBeenCalledTimes(3);
    });

    it('should handle partial batch failure', async () => {
      const transitions = [
        { toBuffer: () => Buffer.from('transition-1') },
        { toBuffer: () => Buffer.from('transition-2') },
      ] as StateTransition[];

      mockProvider.broadcastStateTransition
        .mockResolvedValueOnce('hash-1')
        .mockRejectedValueOnce(new Error('Failed'));

      const results = await broadcaster.broadcastBatch(
        transitions,
        { continueOnError: true }
      );

      expect(results).toHaveLength(2);
      expect(results[0]).toBe('hash-1');
      expect(results[1]).toBeInstanceOf(Error);
    });
  });

  describe('validation', () => {
    it('should validate transition size', async () => {
      const largeTransition: StateTransition = {
        toBuffer: () => Buffer.alloc(1024 * 1024 * 10), // 10MB
      };

      await expect(
        broadcaster.broadcast(largeTransition)
      ).rejects.toThrow('State transition too large');
    });

    it('should validate transition signature', async () => {
      const unsignedTransition: StateTransition = {
        toBuffer: () => Buffer.from('unsigned-data'),
        signature: undefined,
      } as any;

      await expect(
        broadcaster.broadcast(unsignedTransition)
      ).rejects.toThrow('signature');
    });
  });

  describe('event emission', () => {
    it('should emit events on broadcast lifecycle', async () => {
      const events: string[] = [];
      
      broadcaster.on('broadcast:start', () => events.push('start'));
      broadcaster.on('broadcast:success', () => events.push('success'));
      broadcaster.on('broadcast:error', () => events.push('error'));

      mockProvider.broadcastStateTransition.mockResolvedValue('hash-123');

      await broadcaster.broadcast(mockStateTransition);

      expect(events).toEqual(['start', 'success']);
    });

    it('should emit error event on failure', async () => {
      const errorHandler = jest.fn();
      broadcaster.on('broadcast:error', errorHandler);

      mockProvider.broadcastStateTransition.mockRejectedValue(
        new Error('Broadcast failed')
      );

      try {
        await broadcaster.broadcast(mockStateTransition);
      } catch (e) {
        // Expected
      }

      expect(errorHandler).toHaveBeenCalledWith(
        expect.objectContaining({
          error: expect.any(Error),
          transition: mockStateTransition,
        })
      );
    });
  });

  describe('monitoring and metrics', () => {
    it('should track broadcast performance', async () => {
      mockProvider.broadcastStateTransition.mockResolvedValue('hash-123');

      const startHandler = jest.fn();
      const successHandler = jest.fn();
      
      broadcaster.on('broadcast:start', startHandler);
      broadcaster.on('broadcast:success', successHandler);

      await broadcaster.broadcast(mockStateTransition);

      const successData = successHandler.mock.calls[0][0];
      expect(successData).toHaveProperty('duration');
      expect(successData.duration).toBeGreaterThan(0);
    });

    it('should maintain broadcast history', async () => {
      mockProvider.broadcastStateTransition
        .mockResolvedValueOnce('hash-1')
        .mockResolvedValueOnce('hash-2')
        .mockRejectedValueOnce(new Error('Failed'));

      await broadcaster.broadcast(mockStateTransition);
      await broadcaster.broadcast(mockStateTransition);
      
      try {
        await broadcaster.broadcast(mockStateTransition);
      } catch (e) {
        // Expected
      }

      const stats = broadcaster.getStats();
      expect(stats.total).toBe(3);
      expect(stats.successful).toBe(2);
      expect(stats.failed).toBe(1);
    });
  });

  describe('integration with testnet', () => {
    it('should use correct network parameters', async () => {
      const testnetBroadcaster = new StateTransitionBroadcaster(
        mockProvider,
        { network: 'testnet' }
      );

      mockProvider.broadcastStateTransition.mockResolvedValue('testnet-hash');

      await testnetBroadcaster.broadcast(mockStateTransition);

      // Verify testnet-specific behavior
      expect(mockProvider.broadcastStateTransition).toHaveBeenCalled();
    });

    it('should handle testnet-specific errors', async () => {
      mockProvider.broadcastStateTransition.mockRejectedValue(
        new Error('Testnet rate limit exceeded')
      );

      await expect(
        broadcaster.broadcast(mockStateTransition)
      ).rejects.toThrow('rate limit');
    });
  });
});