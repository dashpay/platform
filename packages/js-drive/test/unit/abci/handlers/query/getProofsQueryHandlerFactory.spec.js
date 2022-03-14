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
const BlockExecutionContextStackMock = require('../../../../../lib/test/mock/BlockExecutionContextStackMock');

describe('getProofsQueryHandlerFactory', () => {
  let getProofsQueryHandler;
  let dataContract;
  let identity;
  let documents;
  let dataContractData;
  let documentsData;
  let identityData;
  let blockExecutionContextStackMock;
  let signedBlockExecutionContextMock;
  let blockExecutionContextMock;

  beforeEach(function beforeEach() {
    dataContract = getDataContractFixture();
    identity = getIdentityFixture();
    documents = getDocumentsFixture();

    signedBlockExecutionContextMock = new BlockExecutionContextMock(this.sinon);
    signedBlockExecutionContextMock.getHeader.returns({
      height: new Long(42),
      coreChainLockedHeight: 41,
    });

    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);

    blockExecutionContextMock.getLastCommitInfo.returns({
      quorumHash: Buffer.alloc(32, 1),
      stateSignature: Buffer.alloc(32, 1),
    });

    blockExecutionContextStackMock = new BlockExecutionContextStackMock(this.sinon);
    blockExecutionContextStackMock.getLast.returns(signedBlockExecutionContextMock);
    blockExecutionContextStackMock.getFirst.returns(blockExecutionContextMock);

    getProofsQueryHandler = getProofsQueryHandlerFactory(
      blockExecutionContextStackMock,
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

  it('should return empty response if there is no signed state', async () => {
    blockExecutionContextStackMock.getLast.returns(null);

    const result = await getProofsQueryHandler({}, {}, {});

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
      merkleProof: Buffer.from([1]),
      // rootTreeProof: Buffer.from('0100000001f0faf5f55674905a68eba1be2f946e667c1cb5010101',
      // 'hex'),
      // storeTreeProof: Buffer.from('03046b657931060076616c75653103046b657932060076616c75653210',
      // 'hex'),
    };

    const result = await getProofsQueryHandler({}, {
      dataContractIds: [dataContractData.id],
      identityIds: [identityData.id],
      documentIds: documentsData.ids,
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
