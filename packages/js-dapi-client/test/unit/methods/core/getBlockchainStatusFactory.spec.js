const {
  v0: {
    GetBlockchainStatusRequest,
    GetBlockchainStatusResponse,
    CorePromiseClient,
  },
} = require('@dashevo/dapi-grpc');

const getBlockchainStatusFactory = require('../../../../lib/methods/core/getBlockchainStatusFactory');

describe('getBlockchainStatusFactory', () => {
  let getBlockchainStatus;
  let grpcTransportMock;

  beforeEach(function beforeEach() {
    grpcTransportMock = {
      request: this.sinon.stub(),
    };
    getBlockchainStatus = getBlockchainStatusFactory(grpcTransportMock);
  });

  it('should return status', async () => {
    const response = new GetBlockchainStatusResponse();

    response.setStatus(GetBlockchainStatusResponse.Status.READY);

    const chain = new GetBlockchainStatusResponse.Chain();
    chain.setBestBlockHash(Buffer.from('bestBlockHash'));

    response.setChain(chain);

    grpcTransportMock.request.resolves(response);

    const options = {
      timeout: 1000,
    };

    const result = await getBlockchainStatus(
      options,
    );

    const request = new GetBlockchainStatusRequest();

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      CorePromiseClient,
      'getBlockchainStatus',
      request,
      options,
    );

    const expectedResult = {
      ...response.toObject(),
      status: 'READY',
    };

    expectedResult.chain.bestBlockHash = Buffer.from(expectedResult.chain.bestBlockHash, 'base64');

    expect(result).to.deep.equal(expectedResult);
  });
});
