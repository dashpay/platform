const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    GetIdentityByFirstPublicKeyResponse,
  },
} = require('@dashevo/dapi-grpc');

const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');

const getIdentityByFirstPublicKeyHandlerFactory = require(
  '../../../../../lib/grpcServer/handlers/platform/getIdentityByFirstPublicKeyHandlerFactory',
);

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

const AbciResponseError = require('../../../../../lib/errors/AbciResponseError');

describe('getIdentityByFirstPublicKeyHandlerFactory', () => {
  let call;
  let driveStateRepositoryMock;
  let handleAbciResponseErrorMock;
  let getIdentityByFirstPublicKeyHandler;
  let identity;
  let publicKeyHash;

  beforeEach(function beforeEach() {
    publicKeyHash = '556c2910d46fda2b327ef9d9bda850cc84d30db0';

    call = new GrpcCallMock(this.sinon, {
      getPublicKeyHash: this.sinon.stub().returns(
        Buffer.from(publicKeyHash, 'hex'),
      ),
    });

    identity = getIdentityFixture();

    handleAbciResponseErrorMock = this.sinon.stub();

    driveStateRepositoryMock = {
      fetchIdentityByFirstPublicKey: this.sinon.stub().resolves(identity.serialize()),
    };

    getIdentityByFirstPublicKeyHandler = getIdentityByFirstPublicKeyHandlerFactory(
      driveStateRepositoryMock,
      handleAbciResponseErrorMock,
    );
  });

  it('should return valid result', async () => {
    const result = await getIdentityByFirstPublicKeyHandler(call);

    expect(result).to.be.an.instanceOf(GetIdentityByFirstPublicKeyResponse);
    expect(result.getIdentity()).to.deep.equal(identity.serialize());
    expect(driveStateRepositoryMock.fetchIdentityByFirstPublicKey)
      .to.be.calledOnceWith(publicKeyHash);
    expect(handleAbciResponseErrorMock).to.not.be.called();
  });

  it('should throw an InvalidArgumentGrpcError if id is not specified', async () => {
    call.request.getPublicKeyHash.returns(null);

    try {
      await getIdentityByFirstPublicKeyHandler(call);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('Public key hash is not specified');
      expect(driveStateRepositoryMock.fetchIdentityByFirstPublicKey).to.not.be.called();
      expect(handleAbciResponseErrorMock).to.not.be.called();
    }
  });

  it('should throw an error when fetchIdentity throws an AbciResponseError', async () => {
    const code = 2;
    const message = 'Some error';
    const data = 42;
    const abciResponseError = new AbciResponseError(code, { message, data });
    const handleError = new InvalidArgumentGrpcError('Another error');

    driveStateRepositoryMock.fetchIdentityByFirstPublicKey.throws(abciResponseError);
    handleAbciResponseErrorMock.throws(handleError);

    try {
      await getIdentityByFirstPublicKeyHandler(call);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.equal(handleError);
      expect(driveStateRepositoryMock.fetchIdentityByFirstPublicKey)
        .to.be.calledOnceWith(publicKeyHash);
      expect(handleAbciResponseErrorMock).to.be.calledOnceWith(abciResponseError);
    }
  });

  it('should throw an error when fetchIdentity throws unknown error', async () => {
    const error = new Error('Unknown error');

    driveStateRepositoryMock.fetchIdentityByFirstPublicKey.throws(error);

    try {
      await getIdentityByFirstPublicKeyHandler(call);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.equal(error);
      expect(driveStateRepositoryMock.fetchIdentityByFirstPublicKey)
        .to.be.calledOnceWith(publicKeyHash);
      expect(handleAbciResponseErrorMock).to.not.be.called();
    }
  });
});
