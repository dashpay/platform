import init, {
  StateTransitionWASM,
  ConsensusErrorWASM,
} from '../../dist/dpp.compressed.js';

describe('wasm-dpp2 module', () => {
  before(async () => {
    await init();
  });

  it('exposes state transition bindings', () => {
    expect(StateTransitionWASM).to.be.a('function');
  });

  it('exposes consensus error helpers', () => {
    expect(ConsensusErrorWASM).to.be.a('function');
    expect(ConsensusErrorWASM.deserialize).to.be.a('function');
  });
});
