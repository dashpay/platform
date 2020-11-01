const SimplifiedMNListDiff = require('@dashevo/dashcore-lib/lib/deterministicmnlist/SimplifiedMNListDiff');
const SimplifiedMasternodeList = require('../../../lib/core/SimplifiedMasternodeList');

describe('SimplifiedMasternodeList', () => {
  let simplifiedMasternodeList;
  let smlMaxListsLimit;
  let initialSmlDiffs;
  let updatedSmlDiffs;
  let network;
  let initialRawDiff;
  let updatedRawDiff;

  beforeEach(() => {
    network = 'regtest';

    simplifiedMasternodeList = new SimplifiedMasternodeList({
      smlMaxListsLimit,
    });

    initialRawDiff = {
      baseBlockHash: '644bd9dcbc0537026af6d31181570f934d868f121c55513009bb36f509ec816e',
      blockHash: '23beac1b700c4a49855a9653e036219384ac2fab7eeba2ec45b3e2d0063d1285',
      cbTxMerkleTree: '03000000032f7f142e19bee0c595dac9f900695d1e428a4db70a805fda6c834cfec0de506a0d39baea39dbbaf9827a1f3b8f381a65ebcf4c2ef415025bc4d20afd372e680d12c226f084a6e28e421fbedff22b13aa1191d6a80744d104fa75ede12332467d0107',
      cbTx: '03000500010000000000000000000000000000000000000000000000000000000000000000ffffffff0502e9030101ffffffff01a2567a76070000001976a914f713c2fa5ef0e7c48f0d1b3ad2a79150037c72d788ac00000000460200e90300003fdbe53b9a4cd0b62284195cbd4f4c1655ebdd70e9117ed3c0e49c37bfce46060000000000000000000000000000000000000000000000000000000000000000',
      deletedMNs: [],
      mnList: [
        {
          proRegTxHash: 'e57402007ca10454d77437d9c1156b1c4ff8af86d699c08e9a31dbd1dfe3c991',
          confirmedHash: '0000000000000000000000000000000000000000000000000000000000000000',
          service: '127.0.0.1:20001',
          pubKeyOperator: '906d84cb88f532145d8838414f777b971c976ffcf8ccfc57413a13cf2f8a7750a92f9b997a5a741f1afa34d989f4312b',
          votingAddress: 'ydC3Qkhq6qc1qgHD8PVSHyAB6t3NYa7aw4',
          isValid: true,
        },
      ],
      deletedQuorums: [],
      newQuorums: [],
      merkleRootMNList: '0646cebf379ce4c0d37e11e970ddeb55164c4fbd5c198422b6d04c9a3be5db3f',
      merkleRootQuorums: '0000000000000000000000000000000000000000000000000000000000000000',
    };

    updatedRawDiff = {
      baseBlockHash: '644bd9dcbc0537026af6d31181570f934d868f121c55513009bb36f509ec816e',
      blockHash: '65cbdeb9027b64385d4d86ef34b8f270183c9a54a4f9d26b913a9cad590ce667',
      cbTxMerkleTree: '01000000011e99e0a974bfd455b1d9e9ed3093a48520ea8b61f29b4f67f3a1eee1d05284850101',
      cbTx: '03000500010000000000000000000000000000000000000000000000000000000000000000ffffffff050221040101ffffffff02aa8b4e1e030000001976a914fd0483580565779e09cec599cdf0bd80749cc81288aca28b4e1e030000001976a91437a0126bfb415a90fb3e480e4f966d5cc7415f6788ac0000000046020021040000c169ff4e8d01ae25a61ea28de8cdd5b007942583bbbd6cac542b1dee362ce4d20000000000000000000000000000000000000000000000000000000000000000',
      deletedMNs: [],
      mnList: [
        {
          proRegTxHash: 'e57402007ca10454d77437d9c1156b1c4ff8af86d699c08e9a31dbd1dfe3c991',
          confirmedHash: '31a9c472893da180035d4f44c06b549cf3606bdf8fc6bc64ff1a1a49c91a0a27',
          service: '127.0.0.1:20001',
          pubKeyOperator: '906d84cb88f532145d8838414f777b971c976ffcf8ccfc57413a13cf2f8a7750a92f9b997a5a741f1afa34d989f4312b',
          votingAddress: 'ydC3Qkhq6qc1qgHD8PVSHyAB6t3NYa7aw4',
          isValid: true,
        },
      ],
      deletedQuorums: [],
      newQuorums: [],
      merkleRootMNList: 'd2e42c36ee1d2b54ac6cbdbb83259407b0d5cde88da21ea625ae018d4eff69c1',
      merkleRootQuorums: '0000000000000000000000000000000000000000000000000000000000000000',
    };

    initialSmlDiffs = [new SimplifiedMNListDiff(initialRawDiff, network)];
    updatedSmlDiffs = [new SimplifiedMNListDiff(updatedRawDiff, network)];
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
        initialRawDiff.baseBlockHash,
      );
      expect(simplifiedMNList.baseSimplifiedMNList.blockHash).to.equal(
        initialRawDiff.blockHash,
      );
      expect(simplifiedMNList.currentSML.baseBlockHash).to.equal(
        initialRawDiff.baseBlockHash,
      );
      expect(simplifiedMNList.currentSML.blockHash).to.equal(
        initialRawDiff.blockHash,
      );
    });

    it('should add diff to simplifiedMNList', async () => {
      let simplifiedMNList = simplifiedMasternodeList.getStore();

      expect(simplifiedMNList).to.deep.equal(undefined);

      simplifiedMasternodeList.applyDiffs(initialSmlDiffs);

      simplifiedMNList = simplifiedMasternodeList.getStore();

      expect(simplifiedMNList.baseSimplifiedMNList.baseBlockHash).to.equal(
        initialRawDiff.baseBlockHash,
      );
      expect(simplifiedMNList.baseSimplifiedMNList.blockHash).to.equal(
        initialRawDiff.blockHash,
      );
      expect(simplifiedMNList.currentSML.baseBlockHash).to.equal(
        initialRawDiff.baseBlockHash,
      );
      expect(simplifiedMNList.currentSML.blockHash).to.equal(
        initialRawDiff.blockHash,
      );

      simplifiedMasternodeList.applyDiffs(updatedSmlDiffs);

      simplifiedMNList = simplifiedMasternodeList.getStore();

      expect(simplifiedMNList.baseSimplifiedMNList.baseBlockHash).to.equal(
        initialRawDiff.baseBlockHash,
      );
      expect(simplifiedMNList.baseSimplifiedMNList.blockHash).to.equal(
        initialRawDiff.blockHash,
      );
      expect(simplifiedMNList.currentSML.baseBlockHash).to.equal(
        updatedRawDiff.baseBlockHash,
      );
      expect(simplifiedMNList.currentSML.blockHash).to.equal(
        updatedRawDiff.blockHash,
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
