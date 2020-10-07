const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    GetIdentitiesByPublicKeyHashesResponse,
  },
} = require('@dashevo/dapi-grpc');

const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');

const getIdentitiesByPublicKeyHashesHandlerFactory = require(
  '../../../../../lib/grpcServer/handlers/platform/getIdentitiesByPublicKeyHashesHandlerFactory',
);

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

const AbciResponseError = require('../../../../../lib/errors/AbciResponseError');

describe('getIdentitiesByPublicKeyHashesHandlerFactory', () => {
  let call;
  let driveStateRepositoryMock;
  let handleAbciResponseErrorMock;
  let getIdentitiesByPublicKeyHashesHandler;
  let identity;
  let publicKeyHash;

  beforeEach(function beforeEach() {
    publicKeyHash = '556c2910d46fda2b327ef9d9bda850cc84d30db0';

    call = new GrpcCallMock(this.sinon, {
      getPublicKeyHashesList: this.sinon.stub().returns(
        [publicKeyHash],
      ),
    });

    identity = getIdentityFixture();

    handleAbciResponseErrorMock = this.sinon.stub();

    driveStateRepositoryMock = {
      fetchIdentitiesByPublicKeyHashes: this.sinon.stub().resolves([
        identity.serialize(),
      ]),
    };

    getIdentitiesByPublicKeyHashesHandler = getIdentitiesByPublicKeyHashesHandlerFactory(
      driveStateRepositoryMock,
      handleAbciResponseErrorMock,
    );
  });

  it('should return valid result', async () => {
    const result = await getIdentitiesByPublicKeyHashesHandler(call);

    expect(result).to.be.an.instanceOf(GetIdentitiesByPublicKeyHashesResponse);

    expect(result.getIdentitiesList()).to.deep.equal(
      [identity.serialize()],
    );

    expect(driveStateRepositoryMock.fetchIdentitiesByPublicKeyHashes)
      .to.be.calledOnceWith([publicKeyHash]);

    expect(handleAbciResponseErrorMock).to.not.be.called();
  });

  it('should throw an InvalidArgumentGrpcError if no hashes were submitted', async () => {
    call.request.getPublicKeyHashesList.returns([]);

    try {
      await getIdentitiesByPublicKeyHashesHandler(call);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('No public key hashes were provided');
      expect(driveStateRepositoryMock.fetchIdentitiesByPublicKeyHashes).to.not.be.called();
      expect(handleAbciResponseErrorMock).to.not.be.called();
    }
  });

  it('should throw an error when fetchIdentity throws an AbciResponseError', async () => {
    const code = 2;
    const message = 'Some error';
    const data = 42;
    const abciResponseError = new AbciResponseError(code, { message, data });
    const handleError = new InvalidArgumentGrpcError('Another error');

    driveStateRepositoryMock.fetchIdentitiesByPublicKeyHashes.throws(abciResponseError);
    handleAbciResponseErrorMock.throws(handleError);

    try {
      await getIdentitiesByPublicKeyHashesHandler(call);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.equal(handleError);
      expect(driveStateRepositoryMock.fetchIdentitiesByPublicKeyHashes)
        .to.be.calledOnceWith([publicKeyHash]);
      expect(handleAbciResponseErrorMock).to.be.calledOnceWith(abciResponseError);
    }
  });

  it('should throw an error when fetchIdentity throws unknown error', async () => {
    const error = new Error('Unknown error');

    driveStateRepositoryMock.fetchIdentitiesByPublicKeyHashes.throws(error);

    try {
      await getIdentitiesByPublicKeyHashesHandler(call);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.equal(error);
      expect(driveStateRepositoryMock.fetchIdentitiesByPublicKeyHashes)
        .to.be.calledOnceWith([publicKeyHash]);
      expect(handleAbciResponseErrorMock).to.not.be.called();
    }
  });
});
