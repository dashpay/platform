const SimplifiedMNListDiff = require('@dashevo/dashcore-lib/lib/deterministicmnlist/SimplifiedMNListDiff');
const { expect } = require('chai');
const updateSimplifiedMasternodeListFactory = require('../../../lib/core/updateSimplifiedMasternodeListFactory');
const NotEnoughBlocksForValidSMLError = require('../../../lib/core/errors/NotEnoughBlocksForValidSMLError');

describe('updateSimplifiedMasternodeListFactory', () => {
  let updateSimplifiedMasternodeList;
  let coreRpcClientMock;
  let network;
  let smlMaxListsLimit;
  let simplifiedMasternodeListMock;
  let rawDiff;
  let coreHeight;

  beforeEach(function beforeEach() {
    network = 'regtest';

    rawDiff = {
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

    coreHeight = 84202;

    coreRpcClientMock = {
      protx: this.sinon.stub(),
    };

    coreRpcClientMock.protx.resolves({
      result: rawDiff,
    });

    simplifiedMasternodeListMock = {
      applyDiffs: this.sinon.stub(),
    };

    smlMaxListsLimit = 2;

    const loggerMock = {
      debug: this.sinon.stub(),
      info: this.sinon.stub(),
      trace: this.sinon.stub(),
      error: this.sinon.stub(),
    };

    updateSimplifiedMasternodeList = updateSimplifiedMasternodeListFactory(
      coreRpcClientMock,
      simplifiedMasternodeListMock,
      smlMaxListsLimit,
      network,
      loggerMock,
    );
  });

  it('should throw error if not enough blocks for valid SML', async () => {
    try {
      await updateSimplifiedMasternodeList(smlMaxListsLimit);

      expect.fail('should throw NotEnoughBlocksForValidSMLError');
    } catch (e) {
      expect(e).to.be.instanceOf(NotEnoughBlocksForValidSMLError);
      expect(e.getBlockHeight()).to.be.equal(smlMaxListsLimit);
    }
  });

  it('should obtain 16 latest diffs according to core height on first call', async () => {
    await updateSimplifiedMasternodeList(coreHeight);

    const proTxCallCount = coreHeight - (coreHeight - smlMaxListsLimit) + 1;

    expect(coreRpcClientMock.protx.callCount).to.equal(proTxCallCount);

    expect(coreRpcClientMock.protx.getCall(0).args).to.have.deep.members(
      [
        'diff',
        1,
        (coreHeight - smlMaxListsLimit),
      ],
    );

    for (let i = 1; i < proTxCallCount; i++) {
      expect(coreRpcClientMock.protx.getCall(i).args).to.have.deep.members(
        [
          'diff',
          (coreHeight - smlMaxListsLimit) + (i - 1),
          (coreHeight - smlMaxListsLimit) + (i - 1) + 1,
        ],
      );
    }

    const smlDiffs = [];
    for (let i = 0; i < proTxCallCount; i++) {
      smlDiffs.push(new SimplifiedMNListDiff(rawDiff, network));
    }

    const argsDiffBuffers = simplifiedMasternodeListMock.applyDiffs.getCall(0).args[0].map(
      (item) => item.toBuffer(),
    );

    const smlDiffBuffers = smlDiffs.map((item) => item.toBuffer());

    expect(argsDiffBuffers).to.deep.equal(smlDiffBuffers);
  });

  it('should update diffs since last call and up to passed core height', async () => {
    await updateSimplifiedMasternodeList(coreHeight);
    await updateSimplifiedMasternodeList(coreHeight + 1);

    const proTxCallCount = smlMaxListsLimit + 2;

    expect(coreRpcClientMock.protx.callCount).to.equal(proTxCallCount);

    expect(coreRpcClientMock.protx.getCall(0).args).to.have.deep.members(
      [
        'diff',
        1,
        (coreHeight - smlMaxListsLimit),
      ],
    );

    for (let i = 1; i < proTxCallCount; i++) {
      expect(coreRpcClientMock.protx.getCall(i).args).to.have.deep.members(
        [
          'diff',
          (coreHeight - smlMaxListsLimit) + (i - 1),
          (coreHeight - smlMaxListsLimit) + (i - 1) + 1,
        ],
      );
    }

    const simplifiedMNListDiffArray = [];

    for (let i = 0; i < proTxCallCount - 1; i++) {
      simplifiedMNListDiffArray.push(new SimplifiedMNListDiff(rawDiff, network));
    }

    const argsDiffsBuffers = simplifiedMasternodeListMock.applyDiffs.getCall(0).args[0].map(
      (item) => item.toBuffer(),
    );

    const smlDiffBuffers = simplifiedMNListDiffArray.map((item) => item.toBuffer());

    expect(argsDiffsBuffers).to.deep.equal(smlDiffBuffers);
  });

  it('should not update more than 16 diffs', async () => {
    await updateSimplifiedMasternodeList(coreHeight); // 3
    await updateSimplifiedMasternodeList(coreHeight + 10); // 2

    const proTxCallCount = 3 + 2;

    expect(coreRpcClientMock.protx.callCount).to.equal(proTxCallCount);
  });
});
