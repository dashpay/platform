const {
  v0: {
    GetBlockchainStatusRequest,
    GetBlockchainStatusResponse,
    CorePromiseClient,
  },
} = require('@dashevo/dapi-grpc');

const getBlockchainStatusFactory = require('../../../../lib/methods/core/getBlockchainStatusFactory');
const { base64ToBytes } = require('../../../../lib/utils/bytes');

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
    chain.setBestBlockHash(new TextEncoder().encode('bestBlockHash'));

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

    // toObject returns bestBlockHash as base64 string; production code converts to Uint8Array
    expectedResult.chain.bestBlockHash = base64ToBytes(expectedResult.chain.bestBlockHash);

    expect(result).to.deep.equal(expectedResult);
  });
});
