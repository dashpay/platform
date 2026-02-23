import { expect } from './helpers/chai.ts';
import init, { StateTransition, ConsensusError } from '../../dist/dpp.compressed.js';

before(async () => {
  await init();
});

describe('wasm-dpp2 module', () => {
  describe('StateTransition', () => {
    it('should be exposed as a function', () => {
      expect(StateTransition).to.be.a('function');
    });
  });

  describe('ConsensusError', () => {
    it('should be exposed as a function', () => {
      expect(ConsensusError).to.be.a('function');
    });

    describe('deserialize()', () => {
      it('should be exposed as a function', () => {
        expect(ConsensusError.deserialize).to.be.a('function');
      });
    });
  });
});
