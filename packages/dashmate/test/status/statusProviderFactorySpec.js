const statusProviderFactory = require('../../src/status/statusProviderFactory');
const mockMNSync = require('./mocks/mnSyncMock.json');
const mockGetNetworkInfo = require('./mocks/mockGetNetworkInfo.json');
const mockGetBlockchainInfo = require('./mocks/mockGetBlockchainInfo.json');
const MasternodeStateEnum = require('../../src/enums/masternodeState');

describe('statusProvider integration test', () => {
  let statusProvider;

  let dockerComposeMock;
  let mockRpcClient;
  let mockConfig;

  beforeEach(async function it() {
    dockerComposeMock = {
      isServiceRunning: this.sinon.stub(),
      docker: { getContainer: this.sinon.stub() },
      inspectService: this.sinon.stub(),
    };
    mockRpcClient = {
      mnsync: this.sinon.stub(),
      getNetworkInfo: this.sinon.stub(),
      getBlockchainInfo: this.sinon.stub(),
      masternode: this.sinon.stub(),
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

    mockRpcClient.masternode.withArgs('count').resolves({ result: { enabled: 1337 } });
    mockRpcClient.masternode.withArgs('status').resolves({
      result: {
        state: MasternodeStateEnum.READY,
        status: 'Ready',
        proTxHash: 'deadbeef',
        dmnState: { poSePenalty: 0, lastPaidHeight: 1300 },
      },
    });

    const scope = await statusProvider.getOverviewScope(mockConfig);

    expect(scope).to.exist();
  });

  // todo add test checking throws
});
