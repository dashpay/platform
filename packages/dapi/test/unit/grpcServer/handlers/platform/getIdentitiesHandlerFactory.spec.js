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

const {
  IdentityEntry,
  IdentityValue,
  GetIdentitiesResponseV0,
  Identities,
} = GetIdentitiesResponse;

/* eslint-disable import/no-extraneous-dependencies */
const generateRandomIdentifierAsync = require('@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync');
const getIdentityFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getIdentityFixture');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

const getIdentitiesHandlerFactory = require('../../../../../lib/grpcServer/handlers/platform/getIdentitiesHandlerFactory');

describe('getIdentitiesHandlerFactory', () => {
  let call;
  let getIdentitiesHandler;
  let fetchIdentitiesMock;
  let request;
  let id;

  beforeEach(async function beforeEach() {
    id = await generateRandomIdentifierAsync();
    request = {
      getIdsList: this.sinon.stub().returns([id]),
      getProve: this.sinon.stub().returns(true),
    };

    call = new GrpcCallMock(this.sinon, request);

    fetchIdentitiesMock = this.sinon.stub();

    getIdentitiesHandler = getIdentitiesHandlerFactory({
      fetchIdentities: fetchIdentitiesMock,
    });
  });

  it('should return identities', async () => {
    const identityFixture = await getIdentityFixture();

    const identityValue = new IdentityValue();
    identityValue.setValue(identityFixture.toBuffer());

    const identityEntry = new IdentityEntry();
    identityEntry.setKey(id.toBuffer());
    identityEntry.setValue(identityValue);

    const identityEntries = [
      identityEntry,
    ];

    const identities = new Identities();
    identities.setIdentityEntriesList(identityEntries);

    const response = new GetIdentitiesResponse()
      .setV0(new GetIdentitiesResponseV0().setIdentities(identities));

    fetchIdentitiesMock.resolves(response.serializeBinary());

    const result = await getIdentitiesHandler(call);

    expect(result).to.be.an.instanceOf(GetIdentitiesResponse);

    const identityBinaries = result.getV0().getIdentities().getIdentityEntriesList();

    expect(identityBinaries).to.deep.equal(identityEntries);
  });

  it('should return proof', async () => {
    const proofFixture = {
      merkleProof: Buffer.alloc(1, 1),
    };

    const proofMock = new Proof();
    proofMock.setGrovedbProof(proofFixture.merkleProof);

    const response = new GetIdentitiesResponse()
      .setV0(new GetIdentitiesResponseV0().setProof(proofMock));

    fetchIdentitiesMock.resolves(response.serializeBinary());

    const result = await getIdentitiesHandler(call);

    expect(result).to.be.an.instanceOf(GetIdentitiesResponse);

    const proof = result.getV0().getProof();

    expect(proof).to.be.an.instanceOf(Proof);
    const merkleProof = proof.getGrovedbProof();

    expect(merkleProof).to.deep.equal(proofFixture.merkleProof);

    expect(fetchIdentitiesMock).to.be.calledOnceWith(call.request);
  });

  it('should throw InvalidArgumentGrpcError error if ids are not specified', async () => {
    request.getIdsList.returns(null);

    try {
      await getIdentitiesHandler(call);

      expect.fail('should thrown InvalidArgumentGrpcError error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('identity ids are not specified');
      expect(fetchIdentitiesMock).to.be.not.called();
    }
  });

  it('should throw error if driveStateRepository throws an error', async () => {
    const message = 'Some error';
    const abciResponseError = new Error(message);

    fetchIdentitiesMock.throws(abciResponseError);

    try {
      await getIdentitiesHandler(call);

      expect.fail('should throw error');
    } catch (e) {
      expect(e).to.equal(abciResponseError);
    }
  });
});
