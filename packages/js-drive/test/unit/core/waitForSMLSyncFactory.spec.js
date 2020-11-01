const SimplifiedMNListDiff = require('@dashevo/dashcore-lib/lib/deterministicmnlist/SimplifiedMNListDiff');
const { expect } = require('chai');
const EventEmitter = require('events');
const LatestCoreChainLock = require('../../../lib/core/LatestCoreChainLock');
const waitForSMLSyncFactory = require('../../../lib/core/waitForSMLSyncFactory');
const MissingChainlockError = require('../../../lib/core/errors/MissingChainLockError');
const wait = require('../../../lib/util/wait');

describe('waitForSMLSyncFactory', function main() {
  this.timeout(20000);

  let waitForSMLSync;
  let coreRpcClientMock;
  let network;
  let latestCoreChainLockMock;
  let chainLock;
  let smlMaxListsLimit;
  let simplifiedMasternodeListMock;
  let rawDiff;

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

    chainLock = {
      height: 84202,
      signature: '0a43f1c3e5b3e8dbd670bca8d437dc25572f72d8e1e9be673e9ebbb606570307c3e5f5d073f7beb209dd7e0b8f96c751060ab3a7fb69a71d5ccab697b8cfa5a91038a6fecf76b7a827d75d17f01496302942aa5e2c7f4a48246efc8d3941bf6c',
    };

    coreRpcClientMock = {
      getBlockCount: this.sinon.stub().resolves({ result: 1000 }),
      protx: this.sinon.stub(),
    };

    coreRpcClientMock.protx.resolves({
      result: rawDiff,
    });

    latestCoreChainLockMock = new EventEmitter();
    latestCoreChainLockMock.getChainLock = this.sinon.stub().returns(chainLock);

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

    waitForSMLSync = waitForSMLSyncFactory(
      coreRpcClientMock,
      latestCoreChainLockMock,
      simplifiedMasternodeListMock,
      smlMaxListsLimit,
      network,
      loggerMock,
    );
  });

  it('should wait for 1000 height', async () => {
    coreRpcClientMock.getBlockCount.onCall(0).resolves({ result: 999 });
    coreRpcClientMock.getBlockCount.onCall(1).resolves({ result: 1000 });

    await waitForSMLSync();

    expect(latestCoreChainLockMock.getChainLock).to.have.been.calledOnce();
  });

  it('should throw MissingChainlockError if chainlock is empty', async () => {
    latestCoreChainLockMock.getChainLock.returns(null);

    try {
      await waitForSMLSync();

      expect.fail();
    } catch (e) {
      expect(e).to.be.an.instanceOf(MissingChainlockError);
    }
  });

  it('should obtain diff from core rpc', async () => {
    await waitForSMLSync();

    expect(latestCoreChainLockMock.getChainLock).to.have.been.calledOnce();

    const proTxCallCount = chainLock.height - (chainLock.height - smlMaxListsLimit) + 1;

    expect(coreRpcClientMock.protx.callCount).to.equal(proTxCallCount);

    expect(coreRpcClientMock.protx.getCall(0).args).to.have.deep.members(
      [
        'diff',
        1,
        (chainLock.height - smlMaxListsLimit),
      ],
    );

    for (let i = 1; i < proTxCallCount; i++) {
      expect(coreRpcClientMock.protx.getCall(i).args).to.have.deep.members(
        [
          'diff',
          (chainLock.height - smlMaxListsLimit) + (i - 1),
          (chainLock.height - smlMaxListsLimit) + (i - 1) + 1,
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

  it('should update diff on chainLock update', async () => {
    let resolvePromise;
    const done = new Promise((resolve) => {
      resolvePromise = resolve;
    });

    const updatedChainLock = {
      height: 3,
    };

    await waitForSMLSync();
    for (let i = 0; i < smlMaxListsLimit; i += 1) {
      latestCoreChainLockMock.emit(LatestCoreChainLock.EVENTS.update, updatedChainLock);

      await wait(100);
    }

    setTimeout(() => {
      expect(latestCoreChainLockMock.getChainLock).to.have.been.calledOnce();

      const proTxCallCount = smlMaxListsLimit + 1;

      expect(coreRpcClientMock.protx.callCount).to.equal(proTxCallCount);

      expect(coreRpcClientMock.protx.getCall(0).args).to.have.deep.members(
        [
          'diff',
          1,
          (chainLock.height - smlMaxListsLimit),
        ],
      );

      for (let i = 1; i < proTxCallCount; i++) {
        expect(coreRpcClientMock.protx.getCall(i).args).to.have.deep.members(
          [
            'diff',
            (chainLock.height - smlMaxListsLimit) + (i - 1),
            (chainLock.height - smlMaxListsLimit) + (i - 1) + 1,
          ],
        );
      }

      const simplifiedMNListDiffArray = [];

      for (let i = 0; i < proTxCallCount; i++) {
        simplifiedMNListDiffArray.push(new SimplifiedMNListDiff(rawDiff, network));
      }

      const argsDiffsBuffers = simplifiedMasternodeListMock.applyDiffs.getCall(0).args[0].map(
        (item) => item.toBuffer(),
      );

      const smlDiffBuffers = simplifiedMNListDiffArray.map((item) => item.toBuffer());

      expect(argsDiffsBuffers).to.deep.equal(smlDiffBuffers);

      resolvePromise();
    }, smlMaxListsLimit * 100 + 100);

    await done;
  });
});
