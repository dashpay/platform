const cbor = require('cbor');

const Long = require('long');
const ChainInfoCommonStoreRepository = require('../../../lib/chainInfo/ChainInfoCommonStoreRepository');
const ChainInfo = require('../../../lib/chainInfo/ChainInfo');

describe('ChainInfoCommonStoreRepository', () => {
  let commonStoreMock;
  let repository;
  let chainInfo;
  let lastBlockHeight;
  let lastBlockAppHash;

  beforeEach(function beforeEach() {
    commonStoreMock = {
      put: this.sinon.stub(),
      get: this.sinon.stub(),
    };

    repository = new ChainInfoCommonStoreRepository(commonStoreMock);

    lastBlockHeight = Long.fromInt(42);
    lastBlockAppHash = Buffer.from('something');

    chainInfo = new ChainInfo(lastBlockHeight, lastBlockAppHash);
  });

  describe('#store', () => {
    it('should store chain info', async () => {
      const repositoryInstance = await repository.store(chainInfo);
      expect(repositoryInstance).to.equal(repository);

      expect(commonStoreMock.put).to.be.calledOnceWithExactly(
        ChainInfoCommonStoreRepository.COMMON_STORE_KEY_NAME,
        cbor.encodeCanonical(chainInfo.toJSON()),
      );
    });
  });

  describe('#fetch', () => {
    it('should return empty chain info if it is not stored', async () => {
      commonStoreMock.get.returns(null);

      const result = await repository.fetch();

      expect(result).to.be.instanceOf(ChainInfo);
      expect(result.getLastBlockHeight()).to.be.instanceOf(Long);
      expect(result.getLastBlockHeight().toInt()).to.equal(0);
      expect(result.getLastBlockAppHash()).to.deep.equal(Buffer.alloc(0));

      expect(commonStoreMock.get).to.be.calledOnceWithExactly(
        ChainInfoCommonStoreRepository.COMMON_STORE_KEY_NAME,
      );
    });

    it('should return stored chain info', async () => {
      const storedStateBuffer = cbor.encode(chainInfo.toJSON());

      commonStoreMock.get.returns(storedStateBuffer);

      const result = await repository.fetch();

      expect(result).to.be.instanceOf(ChainInfo);
      expect(result.getLastBlockHeight()).to.be.instanceOf(Long);
      expect(result.getLastBlockHeight()).to.deep.equal(lastBlockHeight);
      expect(result.getLastBlockAppHash()).to.deep.equal(lastBlockAppHash);

      expect(commonStoreMock.get).to.be.calledOnceWithExactly(
        ChainInfoCommonStoreRepository.COMMON_STORE_KEY_NAME,
      );
    });
  });
});
