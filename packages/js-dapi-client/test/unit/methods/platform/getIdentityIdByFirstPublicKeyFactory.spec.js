const {
  PlatformPromiseClient,
  GetIdentityIdByFirstPublicKeyRequest,
  GetIdentityIdByFirstPublicKeyResponse,
} = require('@dashevo/dapi-grpc');
const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const grpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

const getIdentityIdByFirstPublicKeyFactory = require(
  '../../../../lib/methods/platform/getIdentityIdByFirstPublicKeyFactory',
);

describe('getIdentityIdByFirstPublicKeyFactory', () => {
  let grpcTransportMock;
  let getIdentityIdByFirstPublicKey;
  let options;
  let response;
  let identityFixture;
  let publicKeyHash;

  beforeEach(function beforeEach() {
    identityFixture = getIdentityFixture();

    response = new GetIdentityIdByFirstPublicKeyResponse();
    response.setId(identityFixture.getId());

    publicKeyHash = '556c2910d46fda2b327ef9d9bda850cc84d30db0';

    grpcTransportMock = {
      request: this.sinon.stub().resolves(response),
    };

    options = {
      timeout: 1000,
    };

    getIdentityIdByFirstPublicKey = getIdentityIdByFirstPublicKeyFactory(grpcTransportMock);
  });

  it('should return identity', async () => {
    const result = await getIdentityIdByFirstPublicKey(publicKeyHash, options);

    const request = new GetIdentityIdByFirstPublicKeyRequest();
    request.setPublicKeyHash(Buffer.from(publicKeyHash, 'hex'));

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getIdentityIdByFirstPublicKey',
      request,
      options,
    );
    expect(result).to.deep.equal(identityFixture.getId());
  });

  it('should return null if identity not found', async () => {
    const error = new Error('Nothing found');
    error.code = grpcErrorCodes.NOT_FOUND;

    grpcTransportMock.request.throws(error);

    const result = await getIdentityIdByFirstPublicKey(publicKeyHash, options);

    const request = new GetIdentityIdByFirstPublicKeyRequest();
    request.setPublicKeyHash(Buffer.from(publicKeyHash, 'hex'));

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getIdentityIdByFirstPublicKey',
      request,
      options,
    );
    expect(result).to.equal(null);
  });

  it('should throw unknown error', async () => {
    const error = new Error('Unknown found');

    grpcTransportMock.request.throws(error);

    const request = new GetIdentityIdByFirstPublicKeyRequest();
    request.setPublicKeyHash(Buffer.from(publicKeyHash, 'hex'));

    try {
      await getIdentityIdByFirstPublicKey(publicKeyHash, options);

      expect.fail('should throw unknown error');
    } catch (e) {
      expect(e).to.deep.equal(error);
      expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
        PlatformPromiseClient,
        'getIdentityIdByFirstPublicKey',
        request,
        options,
      );
    }
  });
});
