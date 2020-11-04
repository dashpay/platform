const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');

const StoreMock = require('../../../lib/test/mock/StoreMock');

const DataContractStoreRepository = require('../../../lib/dataContract/DataContractStoreRepository');

describe('DataContractStoreRepository', () => {
  let dataContract;
  let repository;
  let dppMock;
  let storeMock;
  let transactionMock;

  beforeEach(function beforeEach() {
    dataContract = getDataContractFixture();

    dppMock = createDPPMock(this.sinon);
    dppMock
      .dataContract
      .createFromBuffer
      .resolves(dataContract);

    const containerMock = {
      resolve() {
        return dppMock;
      },
    };

    storeMock = new StoreMock(this.sinon);

    transactionMock = {};

    repository = new DataContractStoreRepository(storeMock, containerMock);
  });

  describe('#store', () => {
    it('should store data contract', async () => {
      const repositoryInstance = await repository.store(dataContract, transactionMock);
      expect(repositoryInstance).to.equal(repository);

      expect(storeMock.put).to.be.calledOnceWithExactly(
        dataContract.getId(),
        dataContract.toBuffer(),
        transactionMock,
      );
    });
  });

  describe('#fetch', () => {
    it('should return null if data contract is not present', async () => {
      storeMock.get.returns(null);

      const result = await repository.fetch(dataContract.getId(), transactionMock);

      expect(result).to.be.null();

      expect(storeMock.get).to.be.calledOnceWithExactly(
        dataContract.getId(),
        transactionMock,
      );
    });

    it('should return data contract', async () => {
      const encodedDataContract = dataContract.toBuffer();

      storeMock.get.returns(encodedDataContract);

      const result = await repository.fetch(dataContract.getId(), transactionMock);

      expect(result).to.be.deep.equal(dataContract);

      expect(storeMock.get).to.be.calledOnceWithExactly(
        dataContract.getId(),
        transactionMock,
      );
    });
  });
});
