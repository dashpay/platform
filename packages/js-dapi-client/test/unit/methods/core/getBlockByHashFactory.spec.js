const {
  v0: {
    GetBlockRequest,
    GetBlockResponse,
    CorePromiseClient,
  },
} = require('@dashevo/dapi-grpc');

const grpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

const getBlockByHashFactory = require('../../../../lib/methods/core/getBlockByHashFactory');

describe('getBlockByHashFactory', () => {
  let getBlockByHash;
  let grpcTransportMock;
  let block;

  beforeEach(function beforeEach() {
    block = Buffer.from('block');
    const response = new GetBlockResponse();
    response.setBlock(block);

    grpcTransportMock = {
      request: this.sinon.stub().resolves(response),
    };
    getBlockByHash = getBlockByHashFactory(grpcTransportMock);
  });

  it('should return block by hash', async () => {
    const options = {
      timeout: 1000,
    };

    const hash = '4f46066bd50cc2684484407696b7949e82bd906ea92c040f59a97cba47ed8176';

    const result = await getBlockByHash(hash, options);

    const request = new GetBlockRequest();
    request.setHash(hash);

    expect(result).to.be.instanceof(Buffer);
    expect(result).to.deep.equal(block);
    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      CorePromiseClient,
      'getBlock',
      request,
      options,
    );
  });

  it('should return null if block is not found', async () => {
    const error = new Error('Nothing found');
    error.code = grpcErrorCodes.NOT_FOUND;

    grpcTransportMock.request.throws(error);

    const hash = '4f46066bd50cc2684484407696b7949e82bd906ea92c040f59a97cba47ed8176';

    const result = await getBlockByHash(hash);

    const request = new GetBlockRequest();
    request.setHash(hash);

    expect(result).to.equal(null);
    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      CorePromiseClient,
      'getBlock',
      request,
      {},
    );
  });

  it('should throw unknown error', async () => {
    const error = new Error('Unknown found');

    grpcTransportMock.request.throws(error);

    const hash = '4f46066bd50cc2684484407696b7949e82bd906ea92c040f59a97cba47ed8176';

    const request = new GetBlockRequest();
    request.setHash(hash);

    try {
      await getBlockByHash(hash);

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
