const detectStandaloneRegtestModeFactory = require('../../../lib/core/detectStandaloneRegtestModeFactory');

describe('detectStandaloneRegtestModeFactory', function describe() {
  this.timeout(6000);

  let detectStandaloneRegtestMode;
  let coreRpcClientMock;

  beforeEach(function beforeEach() {
    coreRpcClientMock = {
      getBlockchainInfo: this.sinon.stub(),
      getPeerInfo: this.sinon.stub(),
    };

    detectStandaloneRegtestMode = detectStandaloneRegtestModeFactory(coreRpcClientMock);
  });

  it('should return true if chain is regtest and has no peers', async () => {
    coreRpcClientMock.getBlockchainInfo.resolves({
      result: {
        chain: 'regtest',
      },
    });

    coreRpcClientMock.getPeerInfo.resolves({
      result: [],
    });

    const isStandaloneRegtestMode = await detectStandaloneRegtestMode();

    expect(isStandaloneRegtestMode).to.be.true();
    expect(coreRpcClientMock.getBlockchainInfo).to.calledOnce();
    expect(coreRpcClientMock.getPeerInfo).to.calledOnce();
  });

  it('should return false if chain is not regtest', async () => {
    coreRpcClientMock.getBlockchainInfo.resolves({
      result: {
        chain: 'mainnet',
      },
    });

    coreRpcClientMock.getPeerInfo.resolves({
      result: [],
    });

    const isStandaloneRegtestMode = await detectStandaloneRegtestMode();

    expect(isStandaloneRegtestMode).to.be.false();
    expect(coreRpcClientMock.getBlockchainInfo).to.calledOnce();
    expect(coreRpcClientMock.getPeerInfo).to.be.not.called();
  });

  it('should return false if peers count > 0', async () => {
    coreRpcClientMock.getBlockchainInfo.resolves({
      result: {
        chain: 'regtest',
      },
    });

    coreRpcClientMock.getPeerInfo.resolves({
      result: ['peer'],
    });

    const isStandaloneRegtestMode = await detectStandaloneRegtestMode();

    expect(isStandaloneRegtestMode).to.be.false();
    expect(coreRpcClientMock.getBlockchainInfo).to.calledOnce();
    expect(coreRpcClientMock.getPeerInfo).to.calledOnce();
  });
});
