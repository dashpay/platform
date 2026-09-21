import type { SinonStub } from 'sinon';
import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import { EvoSDK } from '../../../dist/sdk.js';

describe('ContractsFacade', () => {
  let wasmSdk: wasmSDKPackage.WasmSdk;
  let client: EvoSDK;
  let dataContract: wasmSDKPackage.DataContract;
  let identityKey: wasmSDKPackage.IdentityPublicKey;
  let signer: wasmSDKPackage.IdentitySigner;

  // Stub references for type-safe assertions
  let getDataContractStub: SinonStub;
  let getDataContractWithProofInfoStub: SinonStub;
  let getDataContractHistoryStub: SinonStub;
  let getDataContractHistoryWithProofInfoStub: SinonStub;
  let getDataContractsStub: SinonStub;
  let getDataContractsWithProofInfoStub: SinonStub;
  let getDataContractsByRangeStub: SinonStub;
  let getDataContractsByRangeWithProofInfoStub: SinonStub;
  let getDataContractsLatestVersionsStub: SinonStub;
  let getDataContractsLatestVersionsWithProofInfoStub: SinonStub;
  let contractPublishStub: SinonStub;
  let contractUpdateStub: SinonStub;

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnet();
    wasmSdk = await builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    // Create mock objects
    dataContract = Object.create(wasmSDKPackage.DataContract.prototype);
    identityKey = Object.create(wasmSDKPackage.IdentityPublicKey.prototype);
    signer = Object.create(wasmSDKPackage.IdentitySigner.prototype);

    // Stub query methods
    getDataContractStub = this.sinon.stub(wasmSdk, 'getDataContract').resolves(dataContract);
    getDataContractWithProofInfoStub = this.sinon.stub(wasmSdk, 'getDataContractWithProofInfo').resolves({
      data: dataContract,
      proof: {},
      metadata: {},
    });
    getDataContractHistoryStub = this.sinon.stub(wasmSdk, 'getDataContractHistory').resolves(new Map());
    getDataContractHistoryWithProofInfoStub = this.sinon.stub(wasmSdk, 'getDataContractHistoryWithProofInfo').resolves({
      data: new Map(),
      proof: {},
      metadata: {},
    });
    getDataContractsStub = this.sinon.stub(wasmSdk, 'getDataContracts').resolves(new Map());
    getDataContractsWithProofInfoStub = this.sinon.stub(wasmSdk, 'getDataContractsWithProofInfo').resolves({
      data: new Map(),
      proof: {},
      metadata: {},
    });
    getDataContractsByRangeStub = this.sinon.stub(wasmSdk, 'getDataContractsByRange').resolves(new Map());
    getDataContractsByRangeWithProofInfoStub = this.sinon.stub(wasmSdk, 'getDataContractsByRangeWithProofInfo').resolves({
      data: new Map(),
      proof: {},
      metadata: {},
    });
    getDataContractsLatestVersionsStub = this.sinon.stub(wasmSdk, 'getDataContractsLatestVersions').resolves(new Map());
    getDataContractsLatestVersionsWithProofInfoStub = this.sinon.stub(wasmSdk, 'getDataContractsLatestVersionsWithProofInfo').resolves({
      data: new Map(),
      proof: {},
      metadata: {},
    });

    // Stub transition methods
    contractPublishStub = this.sinon.stub(wasmSdk, 'contractPublish').resolves(dataContract);
    contractUpdateStub = this.sinon.stub(wasmSdk, 'contractUpdate').resolves();
  });

  describe('fetch()', () => {
    it('should return a DataContract for valid ID', async () => {
      const contractId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';

      const result = await client.contracts.fetch(contractId);

      expect(getDataContractStub).to.be.calledOnceWithExactly(contractId);
      expect(result).to.be.instanceOf(wasmSDKPackage.DataContract);
    });
  });

  describe('fetchWithProof()', () => {
    it('should return DataContract with proof metadata', async () => {
      const contractId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';

      await client.contracts.fetchWithProof(contractId);

      expect(getDataContractWithProofInfoStub).to.be.calledOnceWithExactly(contractId);
    });
  });

  describe('getHistory()', () => {
    it('should fetch contract version history', async () => {
      const query = {
        dataContractId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        limit: 10,
        startAtMs: 1700000000000,
      };

      await client.contracts.getHistory(query);

      expect(getDataContractHistoryStub).to.be.calledOnceWithExactly(query);
    });
  });

  describe('getHistoryWithProof()', () => {
    it('should fetch contract version history with proof', async () => {
      const query = {
        dataContractId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
      };

      await client.contracts.getHistoryWithProof(query);

      expect(getDataContractHistoryWithProofInfoStub).to.be.calledOnceWithExactly(query);
    });
  });

  describe('getMany()', () => {
    it('should fetch multiple contracts by IDs', async () => {
      const contractIds = [
        'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS',
      ];

      await client.contracts.getMany(contractIds);

      expect(getDataContractsStub).to.be.calledOnceWithExactly(contractIds);
    });
  });

  describe('getManyWithProof()', () => {
    it('should fetch multiple contracts with proof', async () => {
      const contractIds = ['GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec'];

      await client.contracts.getManyWithProof(contractIds);

      expect(getDataContractsWithProofInfoStub).to.be.calledOnceWithExactly(contractIds);
    });
  });

  describe('getByRange()', () => {
    it('should fetch a page of contracts by range', async () => {
      const query = { limit: 2, startAfter: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec' };

      await client.contracts.getByRange(query);

      expect(getDataContractsByRangeStub).to.be.calledOnceWithExactly(query);
    });
  });

  describe('getByRangeWithProof()', () => {
    it('should fetch a page of contracts by range with proof', async () => {
      const query = { idsOnly: true };

      await client.contracts.getByRangeWithProof(query);

      expect(getDataContractsByRangeWithProofInfoStub).to.be.calledOnceWithExactly(query);
    });
  });

  describe('getLatestVersions()', () => {
    it('should fetch the latest versions of contracts', async () => {
      const query = { contractIds: ['GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec'], includeContracts: true };

      await client.contracts.getLatestVersions(query);

      expect(getDataContractsLatestVersionsStub).to.be.calledOnceWithExactly(query);
    });
  });

  describe('getLatestVersionsWithProof()', () => {
    it('should fetch the latest versions of contracts with proof', async () => {
      const query = { contractIds: ['GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec'] };

      await client.contracts.getLatestVersionsWithProof(query);

      expect(getDataContractsLatestVersionsWithProofInfoStub).to.be.calledOnceWithExactly(query);
    });
  });

  describe('publish()', () => {
    it('should publish a new data contract', async () => {
      const options = {
        dataContract,
        identityKey,
        signer,
        settings: { retries: 3 },
      };

      const result = await client.contracts.publish(options);

      expect(contractPublishStub).to.be.calledOnceWithExactly(options);
      expect(result).to.be.instanceOf(wasmSDKPackage.DataContract);
    });
  });

  describe('update()', () => {
    it('should update an existing data contract', async () => {
      const options = {
        dataContract,
        identityKey,
        signer,
      };

      await client.contracts.update(options);

      expect(contractUpdateStub).to.be.calledOnceWithExactly(options);
    });
  });

  describe('contract moderation', () => {
    const contractId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
    const identityId = 'H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1';

    // Every moderation transition resolves to the status on the lists its proof covers: both
    // barring lists for a ban, the edited one otherwise. `banned` is only set when `lists`
    // includes `banlist`. Each action gets the options the WASM entrypoint accepts for it: a
    // reason for a ban, a suspend and a warn, `until` for a suspend alone, neither for an
    // unban, an unsuspend or a clearWarnings.
    const transitions = [
      {
        facade: 'banUser',
        wasm: 'contractBanUser',
        extraOptions: { reason: { text: 'spam' } },
        result: { lists: ['banlist', 'suspensions'], banned: true, banReason: { text: 'spam' } },
      },
      {
        facade: 'unbanUser',
        wasm: 'contractUnbanUser',
        extraOptions: {},
        result: { lists: ['banlist'], banned: false },
      },
      {
        facade: 'suspendUser',
        wasm: 'contractSuspendUser',
        extraOptions: { until: BigInt(1800000000000), reason: { code: 7, text: 'flooding' } },
        result: {
          lists: ['suspensions'],
          suspendedUntil: BigInt(1800000000000),
          suspensionReason: { code: 7, text: 'flooding' },
        },
      },
      {
        facade: 'unsuspendUser',
        wasm: 'contractUnsuspendUser',
        extraOptions: {},
        result: { lists: ['suspensions'] },
      },
      {
        facade: 'warnUser',
        wasm: 'contractWarnUser',
        extraOptions: { reason: { text: 'first strike' } },
        result: {
          lists: ['warnings'],
          warnings: [{ warnedAt: BigInt(1700000000000), reason: { text: 'first strike' } }],
        },
      },
      {
        facade: 'clearUserWarnings',
        wasm: 'contractClearUserWarnings',
        extraOptions: {},
        result: { lists: ['warnings'], warnings: [] },
      },
    ] as const;

    transitions.forEach(({
      facade, wasm, extraOptions, result,
    }) => {
      it(`should forward ${facade}() to ${wasm}() and return its per-list result`, async function run() {
        const stub = this.sinon.stub(wasmSdk, wasm).resolves({ contractId, identityId, ...result });
        const options = {
          identity: Object.create(wasmSDKPackage.Identity.prototype),
          contractId,
          identityId,
          signer,
          ...extraOptions,
        };

        // The per-action option types differ, so the facade method is called through one
        // signature wide enough for all six.
        const method = client.contracts[facade].bind(client.contracts) as (
          moderationOptions: typeof options,
        ) => Promise<wasmSDKPackage.ContractModerationResult>;
        const moderated = await method(options);

        expect(stub).to.be.calledOnceWithExactly(options);
        expect(moderated.lists).to.deep.equal(result.lists);
        if (!result.lists.includes('banlist')) {
          expect(moderated.banned).to.equal(undefined);
        }
        expect(moderated.banReason).to.deep.equal('banReason' in result ? result.banReason : undefined);
        expect(moderated.suspensionReason).to.deep.equal(
          'suspensionReason' in result ? result.suspensionReason : undefined,
        );
        expect(moderated.warnings).to.deep.equal('warnings' in result ? result.warnings : undefined);
      });
    });

    it('should fetch a status without naming lists', async function run() {
      const status = { lists: ['banlist'], banned: true };
      const stub = this.sinon.stub(wasmSdk, 'getContractModerationStatus').resolves(status);
      const query = { contractId, identityId };

      const result = await client.contracts.moderationStatus(query);

      expect(stub).to.be.calledOnceWithExactly(query);
      expect(result).to.equal(status);
    });

    it('should fetch a status with proof', async function run() {
      const response = { data: { lists: ['suspensions'] }, proof: {}, metadata: {} };
      const stub = this.sinon.stub(wasmSdk, 'getContractModerationStatusWithProofInfo').resolves(response);
      const query = { contractId, identityId, lists: ['suspensions' as const] };

      const result = await client.contracts.moderationStatusWithProof(query);

      expect(stub).to.be.calledOnceWithExactly(query);
      expect(result.data.banned).to.equal(undefined);
    });

    it('should fetch a page of entries, and the last page carries no cursor', async function run() {
      const page = { entries: [{ identityId }] };
      const stub = this.sinon.stub(wasmSdk, 'getContractModerationEntries').resolves(page);
      const query = { contractId, list: 'banlist' as const, limit: 10 };

      const result = await client.contracts.moderationEntries(query);

      expect(stub).to.be.calledOnceWithExactly(query);
      expect(result.nextStartAfter).to.equal(undefined);
    });

    it('should fetch a page of entries with proof', async function run() {
      const response = { data: { entries: [] }, proof: {}, metadata: {} };
      const stub = this.sinon.stub(wasmSdk, 'getContractModerationEntriesWithProofInfo').resolves(response);
      const query = { contractId, list: 'suspensions' as const };

      const result = await client.contracts.moderationEntriesWithProof(query);

      expect(stub).to.be.calledOnceWithExactly(query);
      expect(result).to.equal(response);
    });

    // A deletion names a document, not an identity, and resolves to the record it left.
    const documentTypeName = 'post';
    const documentId = '2QjL594djCH2NyDsn45vd6yQjEDHupMKo7CEGVTHtQxU';
    const removal = {
      documentOwnerId: identityId,
      moderatorId: contractId,
      reason: { code: 2, text: 'spam' },
      removedAt: BigInt(1800000000000),
      documentHash: '11'.repeat(32),
    };

    it('should forward moderatorDeleteDocument() to contractDeleteDocument() and return the removal record', async function run() {
      const record = {
        contractId, documentTypeName, documentId, ...removal,
      };
      const stub = this.sinon.stub(wasmSdk, 'contractDeleteDocument').resolves(record);
      const options = {
        identity: Object.create(wasmSDKPackage.Identity.prototype),
        contractId,
        documentTypeName,
        documentId,
        reason: { code: 2, text: 'spam' },
        signer,
      };

      const result = await client.contracts.moderatorDeleteDocument(options);

      expect(stub).to.be.calledOnceWithExactly(options);
      expect(result).to.equal(record);
    });

    it('should delete a document without a reason', async function run() {
      const record = {
        contractId, documentTypeName, documentId, ...removal, reason: { text: '' },
      };
      const stub = this.sinon.stub(wasmSdk, 'contractDeleteDocument').resolves(record);
      const options = {
        identity: Object.create(wasmSDKPackage.Identity.prototype),
        contractId,
        documentTypeName,
        documentId,
        signer,
      };

      const result = await client.contracts.moderatorDeleteDocument(options);

      expect(stub).to.be.calledOnceWithExactly(options);
      expect(result.reason).to.deep.equal({ text: '' });
    });

    it('should forward moderatorRestoreDocument() to contractRestoreDocument() and return the marked record', async function run() {
      const record = {
        contractId,
        documentTypeName,
        documentId,
        ...removal,
        restoredBy: identityId,
        restoredAt: BigInt(1800000001000),
      };
      const stub = this.sinon.stub(wasmSdk, 'contractRestoreDocument').resolves(record);
      const options = {
        identity: Object.create(wasmSDKPackage.Identity.prototype),
        contractId,
        documentTypeName,
        document: Object.create(wasmSDKPackage.Document.prototype),
        signer,
      };

      const result = await client.contracts.moderatorRestoreDocument(options);

      expect(stub).to.be.calledOnceWithExactly(options);
      expect(result).to.equal(record);
      expect(result.restoredBy).to.equal(identityId);
    });

    it('should fetch the removal records of the documents named, which carry no cursor', async function run() {
      const page = { removals: [{ documentId, ...removal }] };
      const stub = this.sinon.stub(wasmSdk, 'getContractDocumentRemovals').resolves(page);
      const query = { contractId, documentTypeName, documentIds: [documentId] };

      const result = await client.contracts.documentRemovals(query);

      expect(stub).to.be.calledOnceWithExactly(query);
      expect(result.removals).to.deep.equal(page.removals);
      expect(result.nextStartAfter).to.equal(undefined);
    });

    it('should fetch a page of removal records and its cursor', async function run() {
      const page = { removals: [{ documentId, ...removal }], nextStartAfter: documentId };
      const stub = this.sinon.stub(wasmSdk, 'getContractDocumentRemovals').resolves(page);
      const query = { contractId, documentTypeName, limit: 1 };

      const result = await client.contracts.documentRemovals(query);

      expect(stub).to.be.calledOnceWithExactly(query);
      expect(result.nextStartAfter).to.equal(documentId);
    });

    it('should fetch removal records with proof', async function run() {
      const response = { data: { removals: [] }, proof: {}, metadata: {} };
      const stub = this.sinon.stub(wasmSdk, 'getContractDocumentRemovalsWithProofInfo').resolves(response);
      const query = { contractId, documentTypeName, startAfter: documentId };

      const result = await client.contracts.documentRemovalsWithProof(query);

      expect(stub).to.be.calledOnceWithExactly(query);
      expect(result).to.equal(response);
    });
  });

  describe('contract fee pots', () => {
    const contractId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
    const moderatorId = 'H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1';

    it('should fetch the pots, and a pot never paid out carries no last claim', async function run() {
      const pots = {
        owner: { credits: BigInt(10000000) },
        moderators: {
          credits: BigInt(100000000),
          lastClaimEpoch: 0,
          lastClaimTimeMs: BigInt(1700000000000),
          lastClaimantId: moderatorId,
        },
      };
      const stub = this.sinon.stub(wasmSdk, 'getContractFeePots').resolves(pots);

      const result = await client.contracts.feePots(contractId);

      expect(stub).to.be.calledOnceWithExactly(contractId);
      expect(result.owner.lastClaimEpoch).to.equal(undefined);
      expect(result.owner.lastClaimantId).to.equal(undefined);
      expect(result.moderators.lastClaimEpoch).to.equal(0);
      expect(result.moderators.lastClaimTimeMs).to.equal(BigInt(1700000000000));
      expect(result.moderators.lastClaimantId).to.equal(moderatorId);
    });

    it('should fetch the pots with proof', async function run() {
      const response = {
        data: { owner: { credits: BigInt(0) }, moderators: { credits: BigInt(0) } },
        proof: {},
        metadata: {},
      };
      const stub = this.sinon.stub(wasmSdk, 'getContractFeePotsWithProofInfo').resolves(response);

      const result = await client.contracts.feePotsWithProof(contractId);

      expect(stub).to.be.calledOnceWithExactly(contractId);
      expect(result).to.equal(response);
    });

    it('should forward claimFees() to contractClaimFees() and return the pot it paid out', async function run() {
      const claimed = {
        contractId,
        pot: 'moderators' as const,
        lastClaimEpoch: 12,
        lastClaimTimeMs: BigInt(1700000000000),
        lastClaimantId: Object.create(wasmSDKPackage.Identifier.prototype),
        remainingCredits: BigInt(1),
        balances: new Map([[moderatorId, BigInt(50000000)]]),
      };
      const stub = this.sinon.stub(wasmSdk, 'contractClaimFees').resolves(claimed);
      const options = {
        identity: Object.create(wasmSDKPackage.Identity.prototype),
        contractId,
        pot: 'moderators' as const,
        signer,
      };

      const result = await client.contracts.claimFees(options);

      expect(stub).to.be.calledOnceWithExactly(options);
      expect(result.pot).to.equal('moderators');
      expect(result.lastClaimTimeMs).to.equal(BigInt(1700000000000));
      expect(result.balances.get(moderatorId)).to.equal(BigInt(50000000));
    });
  });
});
