const statusProviderFactory = require('../../src/status/statusProviderFactory')

const sinon = require('sinon')

describe('Dashmate Status Provider Factory tests', () => {
  let statusProvider

  let dockerComposeMock
  let mockRpcClient

  beforeEach(async function it() {
    dockerComposeMock = {isServiceRunning: sinon.stub(), docker: {getContainer: sinon.stub()}}
    mockRpcClient = {mnsync: sinon.stub(), getNetworkInfo: sinon.stub(), getBlockchainInfo: sinon.stub()}
    const createRpcClient = () => (mockRpcClient)

    statusProvider = statusProviderFactory(dockerComposeMock, createRpcClient)
  });

  it('should basically work', async () => {
    const scope = await statusProvider.getOverviewScope()

    expect(scope).to.exist()
  });
});
