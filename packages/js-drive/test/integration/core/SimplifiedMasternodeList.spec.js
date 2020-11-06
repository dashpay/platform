const getSmlFixture = require('../../../lib/test/fixtures/getSmlFixture');
const SimplifiedMasternodeList = require('../../../lib/core/SimplifiedMasternodeList');

describe('SimplifiedMasternodeList', () => {
  let simplifiedMasternodeList;
  let smlMaxListsLimit;
  let initialSmlDiffs;
  let updatedSmlDiffs;

  beforeEach(() => {
    simplifiedMasternodeList = new SimplifiedMasternodeList({
      smlMaxListsLimit,
    });

    initialSmlDiffs = getSmlFixture().slice(0, 16);
    updatedSmlDiffs = getSmlFixture().slice(16, 17);
  });

  it('should set options', async () => {
    expect(simplifiedMasternodeList.options).to.deep.equal({ maxListsLimit: smlMaxListsLimit });
  });

  describe('#applyDiffs', () => {
    it('should create simplifiedMNList', async () => {
      let simplifiedMNList = simplifiedMasternodeList.getStore();

      expect(simplifiedMNList).to.deep.equal(undefined);

      simplifiedMasternodeList.applyDiffs(initialSmlDiffs);

      simplifiedMNList = simplifiedMasternodeList.getStore();

      expect(simplifiedMNList.baseSimplifiedMNList.baseBlockHash).to.equal(
        initialSmlDiffs[0].baseBlockHash,
      );
      expect(simplifiedMNList.baseSimplifiedMNList.blockHash).to.equal(
        initialSmlDiffs[0].blockHash,
      );
      expect(simplifiedMNList.currentSML.baseBlockHash).to.equal(
        initialSmlDiffs[0].baseBlockHash,
      );
      expect(simplifiedMNList.currentSML.blockHash).to.equal(
        initialSmlDiffs[initialSmlDiffs.length - 1].blockHash,
      );
    });

    it('should add diff to simplifiedMNList', async () => {
      let simplifiedMNList = simplifiedMasternodeList.getStore();

      expect(simplifiedMNList).to.deep.equal(undefined);

      simplifiedMasternodeList.applyDiffs(initialSmlDiffs);

      simplifiedMNList = simplifiedMasternodeList.getStore();

      expect(simplifiedMNList.baseSimplifiedMNList.baseBlockHash).to.equal(
        initialSmlDiffs[0].baseBlockHash,
      );
      expect(simplifiedMNList.baseSimplifiedMNList.blockHash).to.equal(
        initialSmlDiffs[0].blockHash,
      );
      expect(simplifiedMNList.currentSML.baseBlockHash).to.equal(
        initialSmlDiffs[0].baseBlockHash,
      );
      expect(simplifiedMNList.currentSML.blockHash).to.equal(
        initialSmlDiffs[initialSmlDiffs.length - 1].blockHash,
      );

      simplifiedMasternodeList.applyDiffs(updatedSmlDiffs);

      simplifiedMNList = simplifiedMasternodeList.getStore();

      expect(simplifiedMNList.baseSimplifiedMNList.baseBlockHash).to.equal(
        initialSmlDiffs[0].baseBlockHash,
      );
      expect(simplifiedMNList.baseSimplifiedMNList.blockHash).to.equal(
        initialSmlDiffs[0].blockHash,
      );
      expect(simplifiedMNList.currentSML.baseBlockHash).to.equal(
        initialSmlDiffs[0].baseBlockHash,
      );
      expect(simplifiedMNList.currentSML.blockHash).to.equal(
        updatedSmlDiffs[0].blockHash,
      );
    });
  });

  describe('#getStore', () => {
    it('should return simplifiedMNList', async () => {
      let simplifiedMNList = simplifiedMasternodeList.getStore();

      expect(simplifiedMNList).to.deep.equal(undefined);

      simplifiedMasternodeList.applyDiffs(initialSmlDiffs);

      simplifiedMNList = simplifiedMasternodeList.getStore();

      expect(simplifiedMNList).to.deep.equal(simplifiedMasternodeList.store);
    });
  });
});
