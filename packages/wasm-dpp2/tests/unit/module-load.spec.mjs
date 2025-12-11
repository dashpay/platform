import init, {
  StateTransition,
  ConsensusError,
} from '../../dist/dpp.compressed.js';

describe('wasm-dpp2 module', () => {
  before(async () => {
    await init();
  });

  it('exposes state transition bindings', () => {
    expect(StateTransition).to.be.a('function');
  });

  it('exposes consensus error helpers', () => {
    expect(ConsensusError).to.be.a('function');
    expect(ConsensusError.deserialize).to.be.a('function');
  });
});
