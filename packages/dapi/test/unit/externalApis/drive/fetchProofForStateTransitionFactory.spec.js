const { expect } = require('chai');

const {
  v0: {
    GetProofsResponse,
    Proof,
  },
} = require('@dashevo/dapi-grpc');

const generateRandomIdentifierAsync = require('@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync');

const fetchProofForStateTransitionFactory = require('../../../../lib/externalApis/drive/fetchProofForStateTransitionFactory');

describe('fetchProofForStateTransition', () => {
  let driveClientMock;
  let fetchProofForStateTransition;
  let identitiesProofResponse;
  let dataContractsProofResponse;
  let documentsProofResponse;
  let stateTransitionFixture;

  beforeEach(async function beforeEach() {
    dataContractsProofResponse = new GetProofsResponse();
    dataContractsProofResponse.setProof(new Proof([Buffer.from('data contracts proof')]));
    documentsProofResponse = new GetProofsResponse();
    documentsProofResponse.setProof(new Proof([Buffer.from('documents contracts proof')]));
    identitiesProofResponse = new GetProofsResponse();
    identitiesProofResponse.setProof(new Proof([Buffer.from('identities contracts proof')]));

    driveClientMock = {
      fetchProofs: this.sinon.stub().callsFake(async (requestProto) => {
        if (requestProto.getIdentitiesList().length > 0) {
          return identitiesProofResponse.serializeBinary();
        } if (requestProto.getDocumentsList().length > 0) {
          return documentsProofResponse.serializeBinary();
        } if (requestProto.getContractsList().length > 0) {
          return dataContractsProofResponse.serializeBinary();
        }

        return null;
      }),
    };

    stateTransitionFixture = {
      isIdentityStateTransition: this.sinon.stub(),
      isDocumentStateTransition: this.sinon.stub(),
      isDataContractStateTransition: this.sinon.stub(),
      getModifiedDataIds: this.sinon.stub().returns([
        await generateRandomIdentifierAsync(),
        await generateRandomIdentifierAsync(),
      ]),
    };

    fetchProofForStateTransition = fetchProofForStateTransitionFactory(driveClientMock);
  });

  it('should fetch identities proofs', async () => {
    stateTransitionFixture.isIdentityStateTransition.returns(true);
    const result = await fetchProofForStateTransition(stateTransitionFixture);
    expect(result.serializeBinary()).to.deep
      .equal(identitiesProofResponse.serializeBinary());
  });

  it('should fetch data contract proofs', async () => {
    stateTransitionFixture.isDataContractStateTransition.returns(true);
    const result = await fetchProofForStateTransition(stateTransitionFixture);
    expect(result.serializeBinary()).to.deep
      .equal(dataContractsProofResponse.serializeBinary());
  });

  it('should fetch documents proofs', async function it() {
    stateTransitionFixture.isDocumentStateTransition.returns(true);
    stateTransitionFixture.getTransitions = this.sinon.stub().returns([
      {
        getDataContractId: this.sinon.stub().returns(await generateRandomIdentifierAsync()),
        getType: this.sinon.stub().returns('niceDocument'),
        getId: this.sinon.stub().returns(await generateRandomIdentifierAsync()),
      },
    ]);

    const result = await fetchProofForStateTransition(stateTransitionFixture);
    expect(result.serializeBinary()).to.deep
      .equal(documentsProofResponse.serializeBinary());
  });
});
