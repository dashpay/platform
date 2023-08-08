const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    GetIdentitiesResponse,
    Proof,
  },
} = require('@dashevo/dapi-grpc');

/* eslint-disable import/no-extraneous-dependencies */
const generateRandomIdentifierAsync = require('@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync');
const getIdentityFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getIdentityFixture');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

const getIdentitiesHandlerFactory = require('../../../../../lib/grpcServer/handlers/platform/getIdentitiesHandlerFactory');

describe('getDataContractsHandlerFactory', () => {
  let call;
  let getIdentitiesHandler;
  let driveClientMock;
  let request;
  let id;
  let identityFixture;
  let proofFixture;
  let proofMock;
  let response;
  let proofResponse;
  let identityEntries;

  beforeEach(async function beforeEach() {
    id = await generateRandomIdentifierAsync();
    request = {
      getIdsList: this.sinon.stub().returns([id]),
      getProve: this.sinon.stub().returns(true),
    };

    call = new GrpcCallMock(this.sinon, request);

    identityFixture = await getIdentityFixture();
    proofFixture = {
      merkleProof: Buffer.alloc(1, 1),
    };

    proofMock = new Proof();
    proofMock.setGrovedbProof(proofFixture.merkleProof);

    response = new GetIdentitiesResponse();

    const identityEntry = new GetIdentitiesResponse.IdentityEntry();
    identityEntry.setKey(id.toBuffer());
    const identityValue = new GetIdentitiesResponse.IdentityValue();
    identityValue.setValue(identityFixture.toBuffer());
    identityEntry.setValue(identityValue);

    identityEntries = [
      identityEntry,
    ];

    const identities = new GetIdentitiesResponse.Identities();
    identities.setIdentityEntriesList(identityEntries);

    response.setIdentities(identities);

    proofResponse = new GetIdentitiesResponse();
    proofResponse.setProof(proofMock);

    driveClientMock = {
      fetchIdentities: this.sinon.stub().resolves(response.serializeBinary()),
    };

    getIdentitiesHandler = getIdentitiesHandlerFactory(
      driveClientMock,
    );
  });

  it('should return identities', async () => {
    const result = await getIdentitiesHandler(call);

    expect(result).to.be.an.instanceOf(GetIdentitiesResponse);

    const identityBinaries = result.getIdentities().getIdentityEntriesList();

    expect(identityBinaries).to.deep.equal(identityEntries);

    const proof = result.getProof();

    expect(proof).to.be.undefined();
  });

  it('should return proof', async function it() {
    driveClientMock = {
      fetchIdentities: this.sinon.stub().resolves(proofResponse.serializeBinary()),
    };

    getIdentitiesHandler = getIdentitiesHandlerFactory(
      driveClientMock,
    );

    const result = await getIdentitiesHandler(call);

    expect(result).to.be.an.instanceOf(GetIdentitiesResponse);

    const contractsBinary = result.getIdentities();
    expect(contractsBinary).to.be.undefined();

    const proof = result.getProof();

    expect(proof).to.be.an.instanceOf(Proof);
    const merkleProof = proof.getGrovedbProof();

    expect(merkleProof).to.deep.equal(proofFixture.merkleProof);

    expect(driveClientMock.fetchIdentities).to.be.calledOnceWith(call.request);
  });

  it('should throw InvalidArgumentGrpcError error if ids are not specified', async () => {
    request.getIdsList.returns(null);

    try {
      await getIdentitiesHandler(call);

      expect.fail('should thrown InvalidArgumentGrpcError error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('identity ids are not specified');
      expect(driveClientMock.fetchIdentities).to.be.not.called();
    }
  });

  it('should throw error if driveStateRepository throws an error', async () => {
    const message = 'Some error';
    const abciResponseError = new Error(message);

    driveClientMock.fetchIdentities.throws(abciResponseError);

    try {
      await getIdentitiesHandler(call);

      expect.fail('should throw error');
    } catch (e) {
      expect(e).to.equal(abciResponseError);
    }
  });
});
