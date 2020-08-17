const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    GetDataContractResponse,
  },
} = require('@dashevo/dapi-grpc');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

const getDataContractHandlerFactory = require(
  '../../../../../lib/grpcServer/handlers/platform/getDataContractHandlerFactory',
);

const AbciResponseError = require('../../../../../lib/errors/AbciResponseError');

describe('getDataContractHandlerFactory', () => {
  let call;
  let getDataContractHandler;
  let driveStateRepositoryMock;
  let request;
  let id;
  let dataContractFixture;
  let handleAbciResponseErrorMock;

  beforeEach(function beforeEach() {
    id = 1;
    request = {
      getId: this.sinon.stub().returns(id),
    };

    call = new GrpcCallMock(this.sinon, request);

    dataContractFixture = getDataContractFixture();

    driveStateRepositoryMock = {
      fetchDataContract: this.sinon.stub().resolves(dataContractFixture.serialize()),
    };

    handleAbciResponseErrorMock = this.sinon.stub();

    getDataContractHandler = getDataContractHandlerFactory(
      driveStateRepositoryMock,
      handleAbciResponseErrorMock,
    );
  });

  it('should return valid data', async () => {
    const result = await getDataContractHandler(call);

    expect(result).to.be.an.instanceOf(GetDataContractResponse);

    const contractBinary = result.getDataContract();
    expect(contractBinary).to.be.an.instanceOf(Buffer);

    expect(handleAbciResponseErrorMock).to.not.be.called();

    expect(contractBinary).to.deep.equal(dataContractFixture.serialize());

    expect(driveStateRepositoryMock.fetchDataContract).to.be.calledOnceWith(id);
  });

  it('should throw InvalidArgumentGrpcError error if id is not specified', async () => {
    id = null;
    request.getId.returns(id);

    try {
      await getDataContractHandler(call);

      expect.fail('should thrown InvalidArgumentGrpcError error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('id is not specified');
      expect(driveStateRepositoryMock.fetchDataContract).to.be.not.called();
      expect(handleAbciResponseErrorMock).to.be.not.called();
    }
  });

  it('should throw InvalidArgumentGrpcError if driveStateRepository throws AbciResponseError', async () => {
    const code = 2;
    const message = 'Some error';
    const data = 42;
    const abciResponseError = new AbciResponseError(code, { message, data });

    const handleError = new InvalidArgumentGrpcError('Another error');

    handleAbciResponseErrorMock.throws(handleError);

    driveStateRepositoryMock.fetchDataContract.throws(abciResponseError);

    try {
      await getDataContractHandler(call);

      expect.fail('should throw InvalidArgumentGrpcError');
    } catch (e) {
      expect(e).to.equal(handleError);
      expect(handleAbciResponseErrorMock).to.be.calledOnceWith(abciResponseError);
    }
  });

  it('should throw error if driveStateRepository throws unknown error', async () => {
    const message = 'Some error';
    const abciResponseError = new Error(message);

    driveStateRepositoryMock.fetchDataContract.throws(abciResponseError);

    try {
      await getDataContractHandler(call);

      expect.fail('should throw error');
    } catch (e) {
      expect(e).to.equal(abciResponseError);
      expect(handleAbciResponseErrorMock).to.be.not.called();
    }
  });
});
