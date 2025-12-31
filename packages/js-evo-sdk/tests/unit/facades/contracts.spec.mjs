import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import { EvoSDK } from '../../../dist/sdk.js';

describe('ContractsFacade', () => {
  let wasmSdk;
  let client;
  let dataContract;
  let identityKey;
  let signer;

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnetTrusted();
    wasmSdk = builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    // Create mock objects
    dataContract = Object.create(wasmSDKPackage.DataContract.prototype);
    identityKey = Object.create(wasmSDKPackage.IdentityPublicKey.prototype);
    signer = Object.create(wasmSDKPackage.IdentitySigner.prototype);

    // Stub query methods
    this.sinon.stub(wasmSdk, 'getDataContract').resolves(dataContract);
    this.sinon.stub(wasmSdk, 'getDataContractWithProofInfo').resolves({
      data: dataContract,
      proof: {},
      metadata: {},
    });
    this.sinon.stub(wasmSdk, 'getDataContractHistory').resolves(new Map());
    this.sinon.stub(wasmSdk, 'getDataContractHistoryWithProofInfo').resolves({
      data: new Map(),
      proof: {},
      metadata: {},
    });
    this.sinon.stub(wasmSdk, 'getDataContracts').resolves(new Map());
    this.sinon.stub(wasmSdk, 'getDataContractsWithProofInfo').resolves({
      data: new Map(),
      proof: {},
      metadata: {},
    });

    // Stub transition methods
    this.sinon.stub(wasmSdk, 'contractPublish').resolves(dataContract);
    this.sinon.stub(wasmSdk, 'contractUpdate').resolves();
  });

  describe('Query Methods', () => {
    it('fetch() returns a DataContract for valid ID', async () => {
      const contractId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';

      const result = await client.contracts.fetch(contractId);

      expect(wasmSdk.getDataContract).to.be.calledOnceWithExactly(contractId);
      expect(result).to.be.instanceOf(wasmSDKPackage.DataContract);
    });

    it('fetchWithProof() returns DataContract with proof metadata', async () => {
      const contractId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';

      await client.contracts.fetchWithProof(contractId);

      expect(wasmSdk.getDataContractWithProofInfo).to.be.calledOnceWithExactly(contractId);
    });

    it('getHistory() fetches contract version history', async () => {
      const query = {
        dataContractId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        limit: 10,
        startAtMs: 1700000000000,
      };

      await client.contracts.getHistory(query);

      expect(wasmSdk.getDataContractHistory).to.be.calledOnceWithExactly(query);
    });

    it('getHistoryWithProof() fetches contract version history with proof', async () => {
      const query = {
        dataContractId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
      };

      await client.contracts.getHistoryWithProof(query);

      expect(wasmSdk.getDataContractHistoryWithProofInfo).to.be.calledOnceWithExactly(query);
    });

    it('getMany() fetches multiple contracts by IDs', async () => {
      const contractIds = [
        'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS',
      ];

      await client.contracts.getMany(contractIds);

      expect(wasmSdk.getDataContracts).to.be.calledOnceWithExactly(contractIds);
    });

    it('getManyWithProof() fetches multiple contracts with proof', async () => {
      const contractIds = ['GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec'];

      await client.contracts.getManyWithProof(contractIds);

      expect(wasmSdk.getDataContractsWithProofInfo).to.be.calledOnceWithExactly(contractIds);
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

      expect(wasmSdk.contractPublish).to.be.calledOnceWithExactly(options);
      expect(result).to.be.instanceOf(wasmSDKPackage.DataContract);
    });

    it('update() updates an existing data contract', async () => {
      const options = {
        dataContract,
        identityKey,
        signer,
      };

      await client.contracts.update(options);

      expect(wasmSdk.contractUpdate).to.be.calledOnceWithExactly(options);
    });
  });
});
