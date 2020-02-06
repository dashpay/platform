const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  GetDataContractResponse,
} = require('@dashevo/dapi-grpc');

const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

const getDataContractHandlerFactory = require(
  '../../../../../lib/grpcServer/handlers/platform/getDataContractHandlerFactory',
);

const RPCError = require('../../../../../lib/rpcServer/RPCError');

describe('getDataContractHandlerFactory', () => {
  let call;
  let getDataContractHandler;
  let driveApiMock;
  let request;
  let id;
  let dataContractFixture;
  let dppMock;

  beforeEach(function beforeEach() {
    id = 1;
    request = {
      getId: this.sinon.stub().returns(id),
    };

    call = new GrpcCallMock(this.sinon, request);

    dataContractFixture = getDataContractFixture();

    driveApiMock = {
      fetchContract: this.sinon.stub().resolves(dataContractFixture.toJSON()),
    };

    dppMock = createDPPMock(this.sinon);
    dppMock.dataContract.createFromObject.returns(dataContractFixture);

    getDataContractHandler = getDataContractHandlerFactory(driveApiMock, dppMock);
  });

  it('should return valid data', async () => {
    const result = await getDataContractHandler(call);

    expect(result).to.be.an.instanceOf(GetDataContractResponse);

    const contractBinary = result.getDataContract();
    expect(contractBinary).to.be.an.instanceOf(Buffer);

    expect(dppMock.dataContract.createFromObject).to.be.calledOnceWith(
      dataContractFixture.toJSON(),
    );

    expect(contractBinary).to.deep.equal(dataContractFixture.serialize());

    expect(driveApiMock.fetchContract).to.be.calledOnceWith(id);
  });

  it('should throw InvalidArgumentGrpcError error if id is not specified', async () => {
    id = null;
    request.getId.returns(id);

    try {
      await getDataContractHandler(call);

      expect.fail('should thrown InvalidArgumentGrpcError error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('Invalid argument: id is not specified');
      expect(driveApiMock.fetchContract).to.be.not.called();
      expect(dppMock.dataContract.createFromObject).to.be.not.called();
    }
  });

  it('should throw InvalidArgumentGrpcError if driveAPI throws RPCError with code -32602', async () => {
    const code = -32602;
    const message = 'message';
    const data = {
      data: 'some data',
    };
    const error = new RPCError(code, message, data);

    driveApiMock.fetchContract.throws(error);

    try {
      await getDataContractHandler(call);

      expect.fail('should throw InvalidArgumentGrpcError error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal(`Invalid argument: ${message}`);
      expect(e.getMetadata()).to.deep.equal(data);
      expect(driveApiMock.fetchContract).to.be.calledOnceWith(id);
      expect(dppMock.document.createFromObject).to.be.not.called();
    }
  });

  it('should throw error if driveAPI throws RPCError with code not equal -32602', async () => {
    const code = -32600;
    const message = 'message';
    const data = {
      data: 'some data',
    };
    const error = new RPCError(code, message, data);

    driveApiMock.fetchContract.throws(error);

    try {
      await getDataContractHandler(call);

      expect.fail('should throw error');
    } catch (e) {
      expect(e).to.equal(error);
      expect(driveApiMock.fetchContract).to.be.calledOnceWith(id);
      expect(dppMock.document.createFromObject).to.be.not.called();
    }
  });
});
