const {
  v0: {
    GetBlockRequest,
    GetBlockResponse,
    CorePromiseClient,
  },
} = require('@dashevo/dapi-grpc');

const getBlockByHeightFactory = require('../../../../lib/methods/core/getBlockByHeightFactory');

describe('getBlockByHeightFactory', () => {
  let getBlockByHeight;
  let grpcTransportMock;
  let block;

  beforeEach(function beforeEach() {
    block = Buffer.from('block');
    const response = new GetBlockResponse();
    response.setBlock(block);

    grpcTransportMock = {
      request: this.sinon.stub().resolves(response),
    };
    getBlockByHeight = getBlockByHeightFactory(grpcTransportMock);
  });

  it('should return block by hash', async () => {
    const options = {
      timeout: 1000,
    };

    const height = 1;

    const result = await getBlockByHeight(height, options);

    const request = new GetBlockRequest();
    request.setHeight(height);

    expect(result).to.be.instanceof(Buffer);
    expect(result).to.deep.equal(block);
    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      CorePromiseClient,
      'getBlock',
      request,
      options,
    );
  });

  it('should throw unknown error', async () => {
    const error = new Error('Unknown found');

    grpcTransportMock.request.throws(error);

    const height = 1;

    const request = new GetBlockRequest();
    request.setHeight(height);

    try {
      await getBlockByHeight(height);

      expect.fail('should throw unknown error');
    } catch (e) {
      expect(e).to.deep.equal(error);
      expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
        CorePromiseClient,
        'getBlock',
        request,
        {},
      );
    }
  });
});
