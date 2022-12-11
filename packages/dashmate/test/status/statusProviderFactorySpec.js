const sinon = require('sinon');
const statusProviderFactory = require('../../src/status/statusProviderFactory');
const mockMNSync = require('./mocks/mnSyncMock.json');
const mockGetNetworkInfo = require('./mocks/mockGetNetworkInfo.json');
const mockGetBlockchainInfo = require('./mocks/mockGetBlockchainInfo.json');

describe('statusProvider integration test', () => {
  let statusProvider;

  let dockerComposeMock;
  let mockRpcClient;
  let mockConfig;

  beforeEach(async function it() {
    dockerComposeMock = {
      isServiceRunning: sinon.stub(),
      docker: { getContainer: sinon.stub() },
      inspectService: sinon.stub(),
    };
    mockRpcClient = {
      mnsync: sinon.stub(),
      getNetworkInfo: sinon.stub(),
      getBlockchainInfo: sinon.stub(),
    };
    mockConfig = { get: this.sinon.stub(), toEnvs: this.sinon.stub() };

    statusProvider = statusProviderFactory(dockerComposeMock, () => mockRpcClient);
  });

  it('should basically work', async () => {
    mockConfig.get.withArgs('network').returns('testnet');
    mockConfig.get.withArgs('core.masternode.enable').returns(false);
    mockConfig.get.withArgs('platform.drive.tenderdash.rpc.port').returns(8080);

    const mockCoreStatus = { State: { Status: 'running' } };

    dockerComposeMock.inspectService.resolves(mockCoreStatus);

    mockRpcClient.mnsync.resolves({ result: mockMNSync });
    mockRpcClient.getBlockchainInfo.resolves({ result: mockGetBlockchainInfo });
    mockRpcClient.getNetworkInfo.resolves({ result: mockGetNetworkInfo });

    const scope = await statusProvider.getOverviewScope(mockConfig);

    expect(scope).to.exist();
  });

  // todo add test checking throws
});
