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
  let contractPublishStub: SinonStub;
  let contractUpdateStub: SinonStub;

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnetTrusted();
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

    // Stub transition methods
    contractPublishStub = this.sinon.stub(wasmSdk, 'contractPublish').resolves(dataContract);
    contractUpdateStub = this.sinon.stub(wasmSdk, 'contractUpdate').resolves();
  });

  describe('Query Methods', () => {
    it('fetch() returns a DataContract for valid ID', async () => {
      const contractId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';

      const result = await client.contracts.fetch(contractId);

      expect(getDataContractStub).to.be.calledOnceWithExactly(contractId);
      expect(result).to.be.instanceOf(wasmSDKPackage.DataContract);
    });

    it('fetchWithProof() returns DataContract with proof metadata', async () => {
      const contractId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';

      await client.contracts.fetchWithProof(contractId);

      expect(getDataContractWithProofInfoStub).to.be.calledOnceWithExactly(contractId);
    });

    it('getHistory() fetches contract version history', async () => {
      const query = {
        dataContractId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        limit: 10,
        startAtMs: 1700000000000,
      };

      await client.contracts.getHistory(query);

      expect(getDataContractHistoryStub).to.be.calledOnceWithExactly(query);
    });

    it('getHistoryWithProof() fetches contract version history with proof', async () => {
      const query = {
        dataContractId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
      };

      await client.contracts.getHistoryWithProof(query);

      expect(getDataContractHistoryWithProofInfoStub).to.be.calledOnceWithExactly(query);
    });

    it('getMany() fetches multiple contracts by IDs', async () => {
      const contractIds = [
        'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS',
      ];

      await client.contracts.getMany(contractIds);

      expect(getDataContractsStub).to.be.calledOnceWithExactly(contractIds);
    });

    it('getManyWithProof() fetches multiple contracts with proof', async () => {
      const contractIds = ['GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec'];

      await client.contracts.getManyWithProof(contractIds);

      expect(getDataContractsWithProofInfoStub).to.be.calledOnceWithExactly(contractIds);
    });
  });

  describe('Transition Methods', () => {
    it('publish() publishes a new data contract', async () => {
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

    it('update() updates an existing data contract', async () => {
      const options = {
        dataContract,
        identityKey,
        signer,
      };

      await client.contracts.update(options);

      expect(contractUpdateStub).to.be.calledOnceWithExactly(options);
    });
  });
});
