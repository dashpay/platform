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

  beforeEach(function beforeEach() {
    grpcTransportMock = {
      request: this.sinon.stub(),
    };
    getStatus = getStatusFactory(grpcTransportMock);
  });

  it('should return status', async () => {
    const response = new GetStatusResponse();

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

    expect(result).to.deep.equal(response.toObject());
  });
});
