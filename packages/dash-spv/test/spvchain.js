const { expect } = require('chai');
const X11 = require('../lib/x11');
const { SpvChain } = require('../index');
const { testnet } = require('./data/rawHeaders');

describe('SPVChain', () => {
  let spvChain;

  beforeEach(async () => {
    spvChain = new SpvChain('testnet', 100);
    await SpvChain.wasmX11Ready();
  });

  describe('#initialize', () => {
    it('should throw an error if wasmX11 is not initialized', function it() {
      this.sinon.stub(X11, 'ready').returns(false);
      expect(() => spvChain.initialize())
        .to.throw('X11 wasm not ready, call wasmX11Ready() first');
    });

    it('should throw an error if chain is already initialized', async () => {
      spvChain.initialize(testnet[0], 10000);
      expect(() => spvChain.initialize())
        .to.throw('Chain already initialized');
    });

    it('should initialize chain with genesis header', async () => {
      spvChain.initialize();
      expect(spvChain.startBlockHeight).to.be.equal(0);
      expect(spvChain.getLongestChain())
        .to.be.deep.equal([spvChain.genesis]);
    });

    it('should not allow initializing from arbitrary block without height', () => {
      expect(() => spvChain.initialize(testnet[0]))
        .to.throw('Initialization error, please provide both startBlock and height');
    });

    it('should initialize from arbitrary block', () => {
      spvChain.initialize(testnet[0], 10000);
      expect(spvChain.startBlockHeight).to.be.equal(10000);
      expect(spvChain.getLongestChain()[0].toBuffer().toString('hex'))
        .to.be.equal(testnet[0]);
    });
  });

  describe('#addHeaders', () => {
    it('should throw an error if wasmX11 is not initialized', function it() {
      this.sinon.stub(X11, 'ready').returns(false);
      expect(() => spvChain.addHeaders(testnet.slice(400)))
        .to.throw('X11 wasm not ready, call wasmX11Ready() first');
    });

    it('should throw an error if chain is not initialized', async () => {
      expect(() => spvChain.addHeaders(testnet.slice(400)))
        .to.throw('Chain not initialized, either call initialize() or set pendingStartBlockHeight');
    });

    it('should assemble headers chain if headers arriving out of order', async () => {
      expect(true).to.equal(true);

      spvChain.initialize(testnet[0], 10000);

      spvChain.addHeaders(testnet.slice(400), 10400);
      expect(spvChain.getOrphanChunks()).to.have.length(1);
      spvChain.addHeaders(testnet.slice(200, 300), 10200);
      expect(spvChain.getOrphanChunks()).to.have.length(2);
      spvChain.addHeaders(testnet.slice(300, 400), 10300);
      expect(spvChain.getOrphanChunks()).to.have.length(3);
      spvChain.addHeaders(testnet.slice(100, 200), 10100);
      expect(spvChain.getOrphanChunks()).to.have.length(4);
      spvChain.addHeaders(testnet.slice(0, 100), 10000);

      const longestChain = spvChain.getLongestChain({ withPruned: true });
      expect(longestChain).to.have.length(testnet.length);
      expect(spvChain.getOrphanChunks()).to.have.length(0);
    });
  });
});
