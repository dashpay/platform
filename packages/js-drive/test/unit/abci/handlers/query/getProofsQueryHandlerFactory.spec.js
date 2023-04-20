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
  let identityRepositoryMock;
  let dataContractRepositoryMock;
  let documentRepository;
  let timeMs;

  beforeEach(function beforeEach() {
    dataContract = getDataContractFixture();
    identity = getIdentityFixture();
    documents = getDocumentsFixture();

    const version = {
      app: Long.fromInt(1),
    };

    timeMs = Date.now();

    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);

    blockExecutionContextMock.getHeight.returns(new Long(42));
    blockExecutionContextMock.getCoreChainLockedHeight.returns(41);
    blockExecutionContextMock.getTimeMs.returns(timeMs);
    blockExecutionContextMock.getVersion.returns(version);
    blockExecutionContextMock.getRound.returns(42);
    blockExecutionContextMock.getLastCommitInfo.returns({
      quorumHash: Buffer.alloc(32, 1),
      blockSignature: Buffer.alloc(32, 1),
    });

    identityRepositoryMock = {
      proveMany: this.sinon.stub().resolves(new StorageResult(Buffer.from([1]))),
    };
    dataContractRepositoryMock = {
      proveMany: this.sinon.stub().resolves(new StorageResult(Buffer.from([1]))),
    };

    documentRepository = {
      proveManyDocumentsFromDifferentContracts: this.sinon.stub().resolves(
        new StorageResult(Buffer.from([1])),
      ),
    };

    getProofsQueryHandler = getProofsQueryHandlerFactory(
      blockExecutionContextMock,
      identityRepositoryMock,
      dataContractRepositoryMock,
      documentRepository,
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
      quorumHash: Buffer.alloc(32, 1),
      signature: Buffer.alloc(32, 1),
      merkleProof: Buffer.from([1]),
      round: 42,
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
            timeMs,
            protocolVersion: 1,
          },
        },
      ),
    });

    expect(result).to.be.deep.equal(expectedResult);
  });
});
