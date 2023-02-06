const {
  tendermint: {
    abci: {
      ResponseQuery,
    },
  },
} = require('@dashevo/abci/types');

const {
  v0: {
    GetDataContractResponse,
    Proof,
  },
} = require('@dashevo/dapi-grpc');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');
const dataContractQueryHandlerFactory = require('../../../../../lib/abci/handlers/query/dataContractQueryHandlerFactory');

const NotFoundAbciError = require('../../../../../lib/abci/errors/NotFoundAbciError');
const StoreRepositoryMock = require('../../../../../lib/test/mock/StoreRepositoryMock');
const InvalidArgumentAbciError = require('../../../../../lib/abci/errors/InvalidArgumentAbciError');
const StorageResult = require('../../../../../lib/storage/StorageResult');

describe('dataContractQueryHandlerFactory', () => {
  let dataContractQueryHandler;
  let dataContract;
  let params;
  let data;
  let createQueryResponseMock;
  let responseMock;
  let dataContractRepositoryMock;

  beforeEach(function beforeEach() {
    dataContract = getDataContractFixture();

    createQueryResponseMock = this.sinon.stub();

    responseMock = new GetDataContractResponse();
    responseMock.setProof(new Proof());

    createQueryResponseMock.returns(responseMock);

    dataContractRepositoryMock = new StoreRepositoryMock(this.sinon);

    dataContractQueryHandler = dataContractQueryHandlerFactory(
      dataContractRepositoryMock,
      createQueryResponseMock,
    );

    params = { };
    data = {
      id: dataContract.getId(),
    };
  });

  it('should throw NotFoundAbciError if Data Contract not found', async () => {
    dataContractRepositoryMock.fetch.resolves(
      new StorageResult(null),
    );

    try {
      await dataContractQueryHandler(params, data, {});

      expect.fail('should throw NotFoundAbciError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(NotFoundAbciError);
      expect(dataContractRepositoryMock.fetch).to.be.calledOnce();
    }
  });

  it('should return data contract', async () => {
    dataContractRepositoryMock.fetch.resolves(
      new StorageResult(dataContract),
    );

    const result = await dataContractQueryHandler(params, data, {});

    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);
    expect(result.value).to.deep.equal(responseMock.serializeBinary());
  });

  it('should InvalidArgumentAbciError on wrong Id', async () => {
    data.id = Buffer.alloc(0);

    try {
      await dataContractQueryHandler(params, data, {});

      expect.fail('should throw InvalidArgumentAbciError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidArgumentAbciError);
    }
  });

  it('should return proof if it was requested', async () => {
    // const proof = {
    // rootTreeProof: Buffer.from('0100000001f0faf5f55674905a68eba1be2f946e667c1cb5010101', 'hex'),
    // storeTreeProof: Buffer.from('03046b657931060076616c75653103046b657932060076616c75653210',
    // 'hex'),
    // };

    const proof = Buffer.alloc(20, 255);

    dataContractRepositoryMock.fetch.resolves(
      new StorageResult(dataContract),
    );

    dataContractRepositoryMock.prove.resolves(
      new StorageResult(proof),
    );

    const result = await dataContractQueryHandler(params, data, { prove: true });

    expect(dataContractRepositoryMock.prove).to.be.calledOnceWithExactly(
      new Identifier(data.id),
    );

    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);
    expect(result.value).to.deep.equal(responseMock.serializeBinary());
  });
});
