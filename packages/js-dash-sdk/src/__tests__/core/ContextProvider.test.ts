import { ContextProvider, BlockHeight, StateTransition } from '../../core/types';

// Mock implementation for testing
class MockContextProvider implements ContextProvider {
  async getBlockHash(height: BlockHeight): Promise<string> {
    return `hash-${height}`;
  }

  async getDataContract(identifier: string): Promise<any> {
    return {
      id: identifier,
      schema: {},
      version: 1,
    };
  }

  async waitForStateTransitionResult(
    stHash: string,
    prove: boolean
  ): Promise<any> {
    return {
      hash: stHash,
      proved: prove,
      result: 'success',
    };
  }

  async broadcastStateTransition(
    stateTransition: StateTransition
  ): Promise<string> {
    return 'broadcast-hash-123';
  }

  async getProtocolVersion(): Promise<number> {
    return 1;
  }
}

describe('ContextProvider Interface', () => {
  let provider: ContextProvider;

  beforeEach(() => {
    provider = new MockContextProvider();
  });

  describe('getBlockHash', () => {
    it('should return block hash for given height', async () => {
      const height = 12345;
      const hash = await provider.getBlockHash(height);
      
      expect(hash).toBe(`hash-${height}`);
    });

    it('should handle edge case heights', async () => {
      const cases = [0, 1, Number.MAX_SAFE_INTEGER];
      
      for (const height of cases) {
        const hash = await provider.getBlockHash(height);
        expect(hash).toBeTruthy();
        expect(typeof hash).toBe('string');
      }
    });
  });

  describe('getDataContract', () => {
    it('should fetch data contract by identifier', async () => {
      const contractId = 'testContract123';
      const contract = await provider.getDataContract(contractId);
      
      expect(contract).toMatchObject({
        id: contractId,
        schema: expect.any(Object),
        version: expect.any(Number),
      });
    });
  });

  describe('waitForStateTransitionResult', () => {
    it('should wait for state transition with proof', async () => {
      const stHash = 'transition-hash-123';
      const result = await provider.waitForStateTransitionResult(stHash, true);
      
      expect(result).toMatchObject({
        hash: stHash,
        proved: true,
        result: 'success',
      });
    });

    it('should wait for state transition without proof', async () => {
      const stHash = 'transition-hash-456';
      const result = await provider.waitForStateTransitionResult(stHash, false);
      
      expect(result).toMatchObject({
        hash: stHash,
        proved: false,
        result: 'success',
      });
    });

    it('should always use proved mode for production', async () => {
      // In production, we should always use proved mode
      const stHash = 'production-transition';
      const result = await provider.waitForStateTransitionResult(stHash, true);
      
      expect(result.proved).toBe(true);
    });
  });

  describe('broadcastStateTransition', () => {
    it('should broadcast state transition', async () => {
      const stateTransition: StateTransition = {
        toBuffer: () => Buffer.from('test-transition'),
      };
      
      const hash = await provider.broadcastStateTransition(stateTransition);
      
      expect(hash).toBeTruthy();
      expect(typeof hash).toBe('string');
    });
  });

  describe('getProtocolVersion', () => {
    it('should return protocol version', async () => {
      const version = await provider.getProtocolVersion();
      
      expect(version).toBeGreaterThan(0);
      expect(Number.isInteger(version)).toBe(true);
    });
  });
});