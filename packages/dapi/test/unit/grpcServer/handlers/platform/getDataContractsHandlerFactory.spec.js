const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    GetDataContractsResponse,
    Proof,
  },
} = require('@dashevo/dapi-grpc');

/* eslint-disable import/no-extraneous-dependencies */
const generateRandomIdentifierAsync = require('@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync');
const getDataContractFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getDataContractFixture');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

const getDataContractsHandlerFactory = require('../../../../../lib/grpcServer/handlers/platform/getDataContractsHandlerFactory');

describe('getDataContractsHandlerFactory', () => {
  let call;
  let getDataContractsHandler;
  let driveClientMock;
  let request;
  let id;
  let dataContractFixture;
  let proofFixture;
  let proofMock;
  let response;
  let proofResponse;
  let dataContractEntries;

  beforeEach(async function beforeEach() {
    id = await generateRandomIdentifierAsync();
    request = {
      getIdsList: this.sinon.stub().returns([id]),
      getProve: this.sinon.stub().returns(true),
    };

    call = new GrpcCallMock(this.sinon, request);

    dataContractFixture = await getDataContractFixture();
    proofFixture = {
      merkleProof: Buffer.alloc(1, 1),
    };

    proofMock = new Proof();
    proofMock.setGrovedbProof(proofFixture.merkleProof);

    response = new GetDataContractsResponse();

    const dataContractEntry = new GetDataContractsResponse.DataContractEntry();
    dataContractEntry.setKey(id.toBuffer());
    const dataContractValue = new GetDataContractsResponse.DataContractValue();
    dataContractValue.setValue(dataContractFixture.toBuffer());
    dataContractEntry.setValue(dataContractValue);

    dataContractEntries = [
      dataContractEntry,
    ];

    const dataContracts = new GetDataContractsResponse.DataContracts();
    dataContracts.setDataContractEntriesList(dataContractEntries);

    response.setDataContracts(dataContracts);

    proofResponse = new GetDataContractsResponse();
    proofResponse.setProof(proofMock);

    driveClientMock = {
      fetchDataContracts: this.sinon.stub().resolves(response.serializeBinary()),
    };

    getDataContractsHandler = getDataContractsHandlerFactory(
      driveClientMock,
    );
  });

  it('should return data contracts', async () => {
    const result = await getDataContractsHandler(call);

    expect(result).to.be.an.instanceOf(GetDataContractsResponse);

    const contractBinaries = result.getDataContracts().getDataContractEntriesList();

    expect(contractBinaries).to.deep.equal(dataContractEntries);

    const proof = result.getProof();

    expect(proof).to.be.undefined();
  });

  it('should return proof', async function it() {
    driveClientMock = {
      fetchDataContracts: this.sinon.stub().resolves(proofResponse.serializeBinary()),
    };

    getDataContractsHandler = getDataContractsHandlerFactory(
      driveClientMock,
    );

    const result = await getDataContractsHandler(call);

    expect(result).to.be.an.instanceOf(GetDataContractsResponse);

    const contractsBinary = result.getDataContracts();
    expect(contractsBinary).to.be.undefined();

    const proof = result.getProof();

    expect(proof).to.be.an.instanceOf(Proof);
    const merkleProof = proof.getGrovedbProof();

    expect(merkleProof).to.deep.equal(proofFixture.merkleProof);

    expect(driveClientMock.fetchDataContracts).to.be.calledOnceWith(call.request);
  });

  it('should throw InvalidArgumentGrpcError error if ids are not specified', async () => {
    request.getIdsList.returns(null);

    try {
      await getDataContractsHandler(call);

      expect.fail('should thrown InvalidArgumentGrpcError error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('data contract ids are not specified');
      expect(driveClientMock.fetchDataContracts).to.be.not.called();
    }
  });

  it('should throw error if driveStateRepository throws an error', async () => {
    const message = 'Some error';
    const abciResponseError = new Error(message);

    driveClientMock.fetchDataContracts.throws(abciResponseError);

    try {
      await getDataContractsHandler(call);

      expect.fail('should throw error');
    } catch (e) {
      expect(e).to.equal(abciResponseError);
    }
  });
});
