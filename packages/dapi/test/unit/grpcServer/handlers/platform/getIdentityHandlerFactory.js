const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    GetIdentityResponse,
  },
} = require('@dashevo/dapi-grpc');

const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');

const getIdentityHandlerFactory = require('../../../../../lib/grpcServer/handlers/platform/getIdentityHandlerFactory');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

const AbciResponseError = require('../../../../../lib/errors/AbciResponseError');


describe('getIdentityHandlerFactory', () => {
  let call;
  let driveStateRepositoryMock;
  let id;
  let handleAbciResponseErrorMock;
  let getIdentityHandler;
  let identity;

  beforeEach(function beforeEach() {
    id = '5poV8Vdi27VksX2RAzAgXmjAh14y87JN2zLvyAwmepRK';
    call = new GrpcCallMock(this.sinon, {
      getId: this.sinon.stub().returns(id),
    });

    identity = getIdentityFixture();

    handleAbciResponseErrorMock = this.sinon.stub();

    driveStateRepositoryMock = {
      fetchIdentity: this.sinon.stub().resolves(identity.serialize()),
    };

    getIdentityHandler = getIdentityHandlerFactory(
      driveStateRepositoryMock,
      handleAbciResponseErrorMock,
    );
  });

  it('should return valid result', async () => {
    const result = await getIdentityHandler(call);

    expect(result).to.be.an.instanceOf(GetIdentityResponse);
    expect(result.getIdentity()).to.deep.equal(identity.serialize());
    expect(driveStateRepositoryMock.fetchIdentity).to.be.calledOnceWith(id);
    expect(handleAbciResponseErrorMock).to.not.be.called();
  });

  it('should throw an InvalidArgumentGrpcError if id is not specified', async () => {
    call.request.getId.returns(null);

    try {
      await getIdentityHandler(call);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('id is not specified');
      expect(driveStateRepositoryMock.fetchIdentity).to.not.be.called();
      expect(handleAbciResponseErrorMock).to.not.be.called();
    }
  });

  it('should throw an error when fetchIdentity throws an AbciResponseError', async () => {
    const code = 2;
    const message = 'Some error';
    const data = 42;
    const abciResponseError = new AbciResponseError(code, { message, data });
    const handleError = new InvalidArgumentGrpcError('Another error');

    driveStateRepositoryMock.fetchIdentity.throws(abciResponseError);
    handleAbciResponseErrorMock.throws(handleError);

    try {
      await getIdentityHandler(call);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.equal(handleError);
      expect(driveStateRepositoryMock.fetchIdentity).to.be.calledOnceWith(id);
      expect(handleAbciResponseErrorMock).to.be.calledOnceWith(abciResponseError);
    }
  });

  it('should throw an error when fetchIdentity throws unknown error', async () => {
    const error = new Error('Unknown error');

    driveStateRepositoryMock.fetchIdentity.throws(error);

    try {
      await getIdentityHandler(call);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.equal(error);
      expect(driveStateRepositoryMock.fetchIdentity).to.be.calledOnceWith(id);
      expect(handleAbciResponseErrorMock).to.not.be.called();
    }
  });
});
