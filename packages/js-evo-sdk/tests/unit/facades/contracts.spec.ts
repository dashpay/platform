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
    // for a ban, the edited one otherwise. `banned` is only set when `lists` includes `banlist`.
    const transitions = [
      {
        facade: 'banUser',
        wasm: 'contractBanUser',
        result: { lists: ['banlist', 'suspensions'], banned: true },
      },
      { facade: 'unbanUser', wasm: 'contractUnbanUser', result: { lists: ['banlist'], banned: false } },
      {
        facade: 'suspendUser',
        wasm: 'contractSuspendUser',
        result: { lists: ['suspensions'], suspendedUntil: BigInt(1800000000000) },
      },
      { facade: 'unsuspendUser', wasm: 'contractUnsuspendUser', result: { lists: ['suspensions'] } },
    ] as const;

    transitions.forEach(({ facade, wasm, result }) => {
      it(`should forward ${facade}() to ${wasm}() and return its per-list result`, async function run() {
        const stub = this.sinon.stub(wasmSdk, wasm).resolves({ contractId, identityId, ...result });
        const options = {
          identity: Object.create(wasmSDKPackage.Identity.prototype),
          contractId,
          identityId,
          until: BigInt(1800000000000),
          signer,
        };

        const moderated = await client.contracts[facade](options);

        expect(stub).to.be.calledOnceWithExactly(options);
        expect(moderated.lists).to.deep.equal(result.lists);
        if (!result.lists.includes('banlist')) {
          expect(moderated.banned).to.equal(undefined);
        }
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
  });
});
