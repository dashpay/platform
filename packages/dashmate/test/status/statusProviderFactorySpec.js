const statusProviderFactory = require('../../src/status/statusProviderFactory')
const sinon = require('sinon')
const mockMNSync = require('./mocks/mnSyncMock.json')
const mockGetNetworkInfo = require('./mocks/mockGetNetworkInfo.json')
const mockGetBlockchainInfo = require('./mocks/mockGetBlockchainInfo.json')

describe('Dashmate Status Provider Factory tests', () => {
  let statusProvider

  let dockerComposeMock
  let mockRpcClient

  beforeEach(async function it() {
    dockerComposeMock = {
      isServiceRunning: sinon.stub(),
      docker: {getContainer: sinon.stub()},
      inspectService: sinon.stub()
    }
    mockRpcClient = {mnsync: sinon.stub(), getNetworkInfo: sinon.stub(), getBlockchainInfo: sinon.stub()}

    statusProvider = statusProviderFactory(dockerComposeMock, () => mockRpcClient)
  });

  it('should basically work', async () => {
    const mockConfig = {network: 'test', 'core.masternode.enable': false, 'platform.drive.tenderdash.rpc.port': 8080}

    const mockCoreStatus = {State: {Status: "running"}}

    dockerComposeMock.inspectService.resolves(mockCoreStatus)

    mockRpcClient.mnsync.resolves({result: mockMNSync})
    mockRpcClient.getBlockchainInfo.resolves({result: mockGetBlockchainInfo})
    mockRpcClient.getNetworkInfo.resolves({result: mockGetNetworkInfo})

    const config = {get: (path) => mockConfig[path], toEnvs: sinon.stub()}

    const scope = await statusProvider.getOverviewScope(config)

    expect(scope).to.exist()
  });
});
