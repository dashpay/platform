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
const StorageResult = require('../../../../../lib/storage/StorageResult');

describe('getProofsQueryHandlerFactory', () => {
  let getProofsQueryHandler;
  let dataContract;
  let identity;
  let documents;
  let dataContractData;
  let documentsData;
  let identityData;
  let blockExecutionContextMock;
  let signedIdentityRepositoryMock;
  let signedDataContractRepositoryMock;
  let signedDocumentRepository;

  beforeEach(function beforeEach() {
    dataContract = getDataContractFixture();
    identity = getIdentityFixture();
    documents = getDocumentsFixture();

    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);

    blockExecutionContextMock.getHeight.returns(new Long(42));
    blockExecutionContextMock.getCoreChainLockedHeight.returns(41);

    blockExecutionContextMock.getLastCommitInfo.returns({
      quorumHash: Buffer.alloc(32, 1),
      stateSignature: Buffer.alloc(32, 1),
    });

    signedIdentityRepositoryMock = {
      proveMany: this.sinon.stub().resolves(new StorageResult(Buffer.from([1]))),
    };
    signedDataContractRepositoryMock = {
      proveMany: this.sinon.stub().resolves(new StorageResult(Buffer.from([1]))),
    };

    signedDocumentRepository = {
      proveManyDocumentsFromDifferentContracts: this.sinon.stub().resolves(
        new StorageResult(Buffer.from([1])),
      ),
    };

    getProofsQueryHandler = getProofsQueryHandlerFactory(
      blockExecutionContextMock,
      signedIdentityRepositoryMock,
      signedDataContractRepositoryMock,
      signedDocumentRepository,
    );

    dataContractData = {
      id: dataContract.getId(),
    };
    identityData = {
      id: identity.getId(),
    };
    documentsData = documents.map((doc) => ({
      documentId: doc.getId(),
      dataContractId: doc.getDataContractId(),
      type: doc.getType(),
    }));
  });

  it('should return proof for passed data contract ids', async () => {
    const expectedProof = {
      signatureLlmqHash: Buffer.alloc(32, 1),
      signature: Buffer.alloc(32, 1),
      merkleProof: Buffer.from([1]),
      // rootTreeProof: Buffer.from('0100000001f0faf5f55674905a68eba1be2f946e667c1cb5010101',
      // 'hex'),
      // storeTreeProof: Buffer.from('03046b657931060076616c75653103046b657932060076616c75653210',
      // 'hex'),
    };

    const result = await getProofsQueryHandler({}, {
      dataContractIds: [dataContractData.id],
      identityIds: [identityData.id],
      documents: documentsData,
    });

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
});
