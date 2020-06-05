const {
  PlatformPromiseClient,
  GetDataContractRequest,
  GetDataContractResponse,
} = require('@dashevo/dapi-grpc');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const grpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

const getDataContractFactory = require('../../../../lib/methods/platform/getDataContractFactory');

describe('getDataContractFactory', () => {
  let grpcTransportMock;
  let getDataContract;
  let options;
  let response;
  let dataContractFixture;

  beforeEach(function beforeEach() {
    dataContractFixture = getDataContractFixture();

    response = new GetDataContractResponse();
    response.setDataContract(dataContractFixture.serialize());

    grpcTransportMock = {
      request: this.sinon.stub().resolves(response),
    };

    options = {
      timeout: 1000,
    };

    getDataContract = getDataContractFactory(grpcTransportMock);
  });

  it('should return data contract', async () => {
    const contractId = dataContractFixture.getId();
    const result = await getDataContract(contractId, options);

    const request = new GetDataContractRequest();
    request.setId(contractId);

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getDataContract',
      request,
      options,
    );
    expect(result).to.deep.equal(dataContractFixture.serialize());
  });

  it('should return null if data contract not found', async () => {
    const error = new Error('Nothing found');
    error.code = grpcErrorCodes.NOT_FOUND;

    grpcTransportMock.request.throws(error);

    const contractId = dataContractFixture.getId();

    const result = await getDataContract(contractId, options);

    const request = new GetDataContractRequest();
    request.setId(contractId);

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getDataContract',
      request,
      options,
    );
    expect(result).to.equal(null);
  });

  it('should throw unknown error', async () => {
    const error = new Error('Unknown found');
    const contractId = dataContractFixture.getId();

    grpcTransportMock.request.throws(error);

    const request = new GetDataContractRequest();
    request.setId(contractId);

    try {
      await getDataContract(contractId, options);

      expect.fail('should throw unknown error');
    } catch (e) {
      expect(e).to.deep.equal(error);
      expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
        PlatformPromiseClient,
        'getDataContract',
        request,
        options,
      );
    }
  });
});
