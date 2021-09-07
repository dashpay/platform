const {
  tendermint: {
    abci: {
      ResponseQuery,
    },
  },
} = require('@dashevo/abci/types');

const Long = require('long');
const cbor = require('cbor');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');

const getProofsQueryHandlerFactory = require('../../../../../lib/abci/handlers/query/getProofsQueryHandlerFactory');
const BlockExecutionContextMock = require('../../../../../lib/test/mock/BlockExecutionContextMock');

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
  let blockExecutionContextMock;
  let previousBlockExecutionContextMock;

  beforeEach(function beforeEach() {
    dataContract = getDataContractFixture();
    identity = getIdentityFixture();
    documents = getDocumentsFixture();

    previousDataContractRepositoryMock = {
      fetch: this.sinon.stub(),
    };

    previousRootTreeMock = {
      getFullProofForOneLeaf: this.sinon.stub(),
    };

    previousDataContractsStoreRootTreeLeafMock = this.sinon.stub();
    previousDocumentsStoreRootTreeLeafMock = this.sinon.stub();
    previousIdentitiesStoreRootTreeLeafMock = this.sinon.stub();

    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);
    previousBlockExecutionContextMock = new BlockExecutionContextMock(this.sinon);

    blockExecutionContextMock.isEmpty.returns(false);
    previousBlockExecutionContextMock.isEmpty.returns(false);

    previousBlockExecutionContextMock.getHeader.returns({
      height: new Long(42),
      coreChainLockedHeight: 41,
    });

    blockExecutionContextMock.getLastCommitInfo.returns({
      quorumHash: Buffer.alloc(32, 1),
      signature: Buffer.alloc(32, 1),
    });

    getProofsQueryHandler = getProofsQueryHandlerFactory(
      previousRootTreeMock,
      previousDocumentsStoreRootTreeLeafMock,
      previousIdentitiesStoreRootTreeLeafMock,
      previousDataContractsStoreRootTreeLeafMock,
      blockExecutionContextMock,
      previousBlockExecutionContextMock,
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

  it('should return empty response if blockExecutionContext is empty', async () => {
    blockExecutionContextMock.isEmpty.returns(true);

    const result = await getProofsQueryHandler({}, {}, {});

    expect(previousRootTreeMock.getFullProofForOneLeaf).to.have.not.been.called();
    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);

    const emptyValue = cbor.encode(
      {
        documentsProof: null,
        identitiesProof: null,
        dataContractsProof: null,
        metadata: {
          height: 0,
          coreChainLockedHeight: 0,
        },
      },
    );

    expect(result.value).to.deep.equal(emptyValue);
  });

  it('should return empty response if previousBlockExecutionContext is empty', async () => {
    previousBlockExecutionContextMock.isEmpty.returns(true);

    const result = await getProofsQueryHandler({}, {}, {});

    expect(previousRootTreeMock.getFullProofForOneLeaf).to.have.not.been.called();
    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);

    const emptyValue = cbor.encode(
      {
        documentsProof: null,
        identitiesProof: null,
        dataContractsProof: null,
        metadata: {
          height: 0,
          coreChainLockedHeight: 0,
        },
      },
    );

    expect(result.value).to.deep.equal(emptyValue);
  });

  it('should return proof for passed data contract ids', async () => {
    const expectedProof = {
      signatureLlmqHash: Buffer.alloc(32, 1),
      signature: Buffer.alloc(32, 1),
      rootTreeProof: Buffer.from('0100000001f0faf5f55674905a68eba1be2f946e667c1cb5010101', 'hex'),
      storeTreeProof: Buffer.from('03046b657931060076616c75653103046b657932060076616c75653210', 'hex'),
    };

    previousDataContractRepositoryMock.fetch.resolves(dataContract);
    previousRootTreeMock.getFullProofForOneLeaf.returns(expectedProof);

    const result = await getProofsQueryHandler({}, {
      dataContractIds: [dataContractData.id],
      identityIds: [identityData.id],
      documentIds: documentsData.ids,
    });

    expect(previousRootTreeMock.getFullProofForOneLeaf).to.be.calledThrice();
    expect(previousRootTreeMock.getFullProofForOneLeaf.getCall(0).args).to.be.deep.equal([
      previousDocumentsStoreRootTreeLeafMock,
      documentsData.ids,
    ]);
    expect(previousRootTreeMock.getFullProofForOneLeaf.getCall(1).args).to.be.deep.equal([
      previousIdentitiesStoreRootTreeLeafMock,
      [identity.getId()],
    ]);
    expect(previousRootTreeMock.getFullProofForOneLeaf.getCall(2).args).to.be.deep.equal([
      previousDataContractsStoreRootTreeLeafMock,
      [dataContract.getId()],
    ]);

    const expectedResult = new ResponseQuery({
      value: cbor.encode(
        {
          documentsProof: expectedProof,
          identitiesProof: expectedProof,
          dataContractsProof: expectedProof,
          metadata: {
            height: 42,
            coreChainLockedHeight: 41,
          },
        },
      ),
    });

    expect(result).to.be.deep.equal(expectedResult);
  });

  it('should return no proofs if no data contract ids were passed', async () => {
    const expectedProof = {
      rootTreeProof: Buffer.from('0100000001f0faf5f55674905a68eba1be2f946e667c1cb5010101', 'hex'),
      storeTreeProof: Buffer.from('03046b657931060076616c75653103046b657932060076616c75653210', 'hex'),
    };

    previousDataContractRepositoryMock.fetch.resolves(dataContract);
    previousRootTreeMock.getFullProofForOneLeaf.returns(expectedProof);

    const result = await getProofsQueryHandler({}, {});

    expect(previousRootTreeMock.getFullProofForOneLeaf).to.not.be.called();

    expect(result).to.be.deep.equal(new ResponseQuery({
      value: cbor.encode({
        documentsProof: null,
        identitiesProof: null,
        dataContractsProof: null,
        metadata: {
          height: 42,
          coreChainLockedHeight: 41,
        },
      }),
    }));
  });

  it('should return no proofs if an empty array of ids was passed', async () => {
    const expectedProof = {
      rootTreeProof: Buffer.from('0100000001f0faf5f55674905a68eba1be2f946e667c1cb5010101', 'hex'),
      storeTreeProof: Buffer.from('03046b657931060076616c75653103046b657932060076616c75653210', 'hex'),
    };

    previousDataContractRepositoryMock.fetch.resolves(dataContract);
    previousRootTreeMock.getFullProofForOneLeaf.returns(expectedProof);

    const result = await getProofsQueryHandler({}, {
      dataContractIds: [],
      identityIds: [],
      documentIds: [],
    });

    expect(previousRootTreeMock.getFullProofForOneLeaf).to.not.be.called();

    expect(result).to.be.deep.equal(new ResponseQuery({
      value: cbor.encode({
        documentsProof: null,
        identitiesProof: null,
        dataContractsProof: null,
        metadata: {
          height: 42,
          coreChainLockedHeight: 41,
        },
      }),
    }));
  });
});
