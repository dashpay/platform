const {
  v0: {
    GetStatusRequest,
    GetStatusResponse,
    CorePromiseClient,
  },
} = require('@dashevo/dapi-grpc');

const getStatusFactory = require('../../../../lib/methods/core/getStatusFactory');

describe('getStatusFactory', () => {
  let getStatus;
  let grpcTransportMock;
  let status;

  beforeEach(function beforeEach() {
    status = {
      coreVersion: 1,
      protocolVersion: 2,
      blocks: 3,
      timeOffset: 4,
      connections: 5,
      proxy: 'proxy',
      difficulty: 0.4344343,
      testnet: true,
      relayFee: 0.1321321,
      errors: '',
      network: 'mainnet',
    };
    grpcTransportMock = {
      request: this.sinon.stub(),
    };
    getStatus = getStatusFactory(grpcTransportMock);
  });

  it('should return status', async () => {
    const response = new GetStatusResponse();
    response.setCoreVersion(status.coreVersion);
    response.setProtocolVersion(status.protocolVersion);
    response.setBlocks(status.blocks);
    response.setTimeOffset(status.timeOffset);
    response.setConnections(status.connections);
    response.setProxy(status.proxy);
    response.setDifficulty(status.difficulty);
    response.setTestnet(status.testnet);
    response.setRelayFee(status.relayFee);
    response.setErrors(status.errors);
    response.setNetwork(status.network);
    grpcTransportMock.request.resolves(response);

    const options = {
      timeout: 1000,
    };

    const result = await getStatus(
      options,
    );

    const request = new GetStatusRequest();

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      CorePromiseClient,
      'getStatus',
      request,
      options,
    );
    expect(result).to.be.deep.equal(status);
  });
});
