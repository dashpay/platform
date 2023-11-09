const { BytesValue } = require('google-protobuf/google/protobuf/wrappers_pb');

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

const {
  DataContractEntry,
  DataContracts,
  GetDataContractsResponseV0,
} = GetDataContractsResponse;

describe('getDataContractsHandlerFactory', () => {
  let call;
  let getDataContractsHandler;
  let request;
  let id;
  let dataContractEntries;
  let fetchDataContractsMock;

  beforeEach(async function beforeEach() {
    id = await generateRandomIdentifierAsync();
    request = {
      getIdsList: this.sinon.stub().returns([id]),
      getProve: this.sinon.stub().returns(true),
    };

    call = new GrpcCallMock(this.sinon, request);

    fetchDataContractsMock = this.sinon.stub();

    getDataContractsHandler = getDataContractsHandlerFactory({
      fetchDataContracts: fetchDataContractsMock,
    });
  });

  it('should return data contracts', async () => {
    const dataContractFixture = await getDataContractFixture();

    const dataContractEntry = new DataContractEntry();
    dataContractEntry.setIdentifier(id.toBuffer());
    dataContractEntry.setDataContract(new BytesValue().setValue(dataContractFixture.toBuffer()));

    dataContractEntries = [
      dataContractEntry,
    ];

    const dataContracts = new DataContracts();
    dataContracts.setDataContractEntriesList(dataContractEntries);

    const response = new GetDataContractsResponse().setV0(
      new GetDataContractsResponseV0().setDataContracts(dataContracts),
    );

    fetchDataContractsMock.resolves(response.serializeBinary());

    const result = await getDataContractsHandler(call);

    expect(result).to.be.an.instanceOf(GetDataContractsResponse);

    const contractBinaries = result.getV0().getDataContracts().getDataContractEntriesList();

    expect(contractBinaries).to.deep.equal(dataContractEntries);

    expect(fetchDataContractsMock).to.be.calledOnceWith(call.request);
  });

  it('should return proof', async () => {
    const proofFixture = {
      merkleProof: Buffer.alloc(1, 1),
    };

    const proofMock = new Proof();
    proofMock.setGrovedbProof(proofFixture.merkleProof);

    const response = new GetDataContractsResponse()
      .setV0(new GetDataContractsResponseV0().setProof(proofMock));

    fetchDataContractsMock.resolves(response.serializeBinary());

    const result = await getDataContractsHandler(call);

    expect(result).to.be.an.instanceOf(GetDataContractsResponse);

    const proof = result.getV0().getProof();

    expect(proof).to.be.an.instanceOf(Proof);
    const merkleProof = proof.getGrovedbProof();

    expect(merkleProof).to.deep.equal(proofFixture.merkleProof);

    expect(fetchDataContractsMock).to.be.calledOnceWith(call.request);
  });

  it('should throw InvalidArgumentGrpcError error if ids are not specified', async () => {
    request.getIdsList.returns(null);

    try {
      await getDataContractsHandler(call);

      expect.fail('should thrown InvalidArgumentGrpcError error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('data contract ids are not specified');
      expect(fetchDataContractsMock).to.be.not.called();
    }
  });

  it('should throw error if driveStateRepository throws an error', async () => {
    const message = 'Some error';
    const abciResponseError = new Error(message);

    fetchDataContractsMock.throws(abciResponseError);

    try {
      await getDataContractsHandler(call);

      expect.fail('should throw error');
    } catch (e) {
      expect(e).to.equal(abciResponseError);
    }
  });
});
