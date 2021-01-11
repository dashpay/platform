const {
  tendermint: {
    abci: {
      ResponseQuery,
    },
  },
} = require('@dashevo/abci/types');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');

const getProofsQueryHandlerFactory = require('../../../../../lib/abci/handlers/query/getProofsQueryHandlerFactory');

describe('getProofsQueryHandlerFactory', () => {
  let getProofsQueryHandler;
  let previousDataContractRepositoryMock;
  let dataContract;
  let identity;
  let documents;
  let dataContractData;
  let documentsData;
  let identityData;
  let previousRootTreeMock;
  let previousDataContractsStoreRootTreeLeafMock;
  let previousIdentitiesStoreRootTreeLeafMock;
  let previousDocumentsStoreRootTreeLeafMock;

  beforeEach(function beforeEach() {
    dataContract = getDataContractFixture();
    identity = getIdentityFixture();
    documents = getDocumentsFixture();

    previousDataContractRepositoryMock = {
      fetch: this.sinon.stub(),
    };

    previousRootTreeMock = {
      getFullProof: this.sinon.stub(),
    };

    previousDataContractsStoreRootTreeLeafMock = this.sinon.stub();
    previousDocumentsStoreRootTreeLeafMock = this.sinon.stub();
    previousIdentitiesStoreRootTreeLeafMock = this.sinon.stub();

    getProofsQueryHandler = getProofsQueryHandlerFactory(
      previousRootTreeMock,
      previousDocumentsStoreRootTreeLeafMock,
      previousIdentitiesStoreRootTreeLeafMock,
      previousDataContractsStoreRootTreeLeafMock,
    );

    dataContractData = {
      id: dataContract.getId(),
    };
    identityData = {
      id: identity.getId(),
    };
    documentsData = {
      ids: documents.map((doc) => doc.getId()),
    };
  });

  it('should return proof for passed data contract ids', async () => {
    const expectedProof = {
      rootTreeProof: Buffer.from('0100000001f0faf5f55674905a68eba1be2f946e667c1cb5010101', 'hex'),
      storeTreeProof: Buffer.from('03046b657931060076616c75653103046b657932060076616c75653210', 'hex'),
    };

    previousDataContractRepositoryMock.fetch.resolves(dataContract);
    previousRootTreeMock.getFullProof.returns(expectedProof);

    const result = await getProofsQueryHandler({
      dataContractIds: [dataContractData.id],
      identityIds: [identityData.id],
      documentIds: documentsData.ids,
    });

    expect(previousRootTreeMock.getFullProof).to.be.calledThrice();
    expect(previousRootTreeMock.getFullProof.getCall(0).args).to.be.deep.equal([
      previousDocumentsStoreRootTreeLeafMock,
      documentsData.ids,
    ]);
    expect(previousRootTreeMock.getFullProof.getCall(1).args).to.be.deep.equal([
      previousIdentitiesStoreRootTreeLeafMock,
      [identity.getId()],
    ]);
    expect(previousRootTreeMock.getFullProof.getCall(2).args).to.be.deep.equal([
      previousDataContractsStoreRootTreeLeafMock,
      [dataContract.getId()],
    ]);

    expect(result).to.be.deep.equal(new ResponseQuery({
      documentsProof: expectedProof,
      identitiesProof: expectedProof,
      dataContractsProof: expectedProof,
    }));
  });

  it('should return no proofs if no data contract ids were passed', async () => {
    const expectedProof = {
      rootTreeProof: Buffer.from('0100000001f0faf5f55674905a68eba1be2f946e667c1cb5010101', 'hex'),
      storeTreeProof: Buffer.from('03046b657931060076616c75653103046b657932060076616c75653210', 'hex'),
    };

    previousDataContractRepositoryMock.fetch.resolves(dataContract);
    previousRootTreeMock.getFullProof.returns(expectedProof);

    const result = await getProofsQueryHandler();

    expect(previousRootTreeMock.getFullProof).to.not.be.called();

    expect(result).to.be.deep.equal(new ResponseQuery({
      documentsProof: null,
      identitiesProof: null,
      dataContractsProof: null,
    }));
  });

  it('should return no proofs if an empty array of ids was passed', async () => {
    const expectedProof = {
      rootTreeProof: Buffer.from('0100000001f0faf5f55674905a68eba1be2f946e667c1cb5010101', 'hex'),
      storeTreeProof: Buffer.from('03046b657931060076616c75653103046b657932060076616c75653210', 'hex'),
    };

    previousDataContractRepositoryMock.fetch.resolves(dataContract);
    previousRootTreeMock.getFullProof.returns(expectedProof);

    const result = await getProofsQueryHandler({
      dataContractIds: [],
      identityIds: [],
      documentIds: [],
    });

    expect(previousRootTreeMock.getFullProof).to.not.be.called();

    expect(result).to.be.deep.equal(new ResponseQuery({
      documentsProof: null,
      identitiesProof: null,
      dataContractsProof: null,
    }));
  });
});
