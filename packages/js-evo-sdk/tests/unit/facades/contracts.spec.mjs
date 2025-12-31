import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import { EvoSDK } from '../../../dist/sdk.js';

describe('ContractsFacade', () => {
  let wasmSdk;
  let client;
  let dataContract;

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnetTrusted();
    wasmSdk = builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    dataContract = Object.create(wasmSDKPackage.DataContract.prototype);

    // instance methods used by ContractsFacade
    this.sinon.stub(wasmSdk, 'getDataContract').resolves(dataContract);
    this.sinon.stub(wasmSdk, 'getDataContractWithProofInfo').resolves(true);
    this.sinon.stub(wasmSdk, 'getDataContractHistory').resolves(true);
    this.sinon.stub(wasmSdk, 'getDataContractHistoryWithProofInfo').resolves(true);
    this.sinon.stub(wasmSdk, 'getDataContracts').resolves(true);
    this.sinon.stub(wasmSdk, 'getDataContractsWithProofInfo').resolves(true);
    this.sinon.stub(wasmSdk, 'contractPublish').resolves(dataContract);
    this.sinon.stub(wasmSdk, 'contractUpdate').resolves();
  });

  it('fetch() forwards to instance getDataContract', async () => {
    const result = await client.contracts.fetch('c');
    expect(wasmSdk.getDataContract).to.be.calledOnceWithExactly('c');
    expect(result).to.be.instanceOf(wasmSDKPackage.DataContract);
  });

  it('fetchWithProof() forwards to instance getDataContractWithProofInfo', async () => {
    await client.contracts.fetchWithProof('c2');
    expect(wasmSdk.getDataContractWithProofInfo).to.be.calledOnceWithExactly('c2');
  });

  it('getHistory() forwards query object', async () => {
    await client.contracts.getHistory({
      dataContractId: 'c',
      limit: 3,
      startAtMs: 5,
    });
    expect(wasmSdk.getDataContractHistory).to.be.calledOnceWithExactly({
      dataContractId: 'c',
      limit: 3,
      startAtMs: 5,
    });
  });

  it('getHistoryWithProof() forwards query object', async () => {
    await client.contracts.getHistoryWithProof({
      dataContractId: 'c',
    });
    expect(wasmSdk.getDataContractHistoryWithProofInfo).to.be.calledOnceWithExactly({
      dataContractId: 'c',
    });
  });

  it('getMany() and getManyWithProof() forward arrays', async () => {
    await client.contracts.getMany(['a', 'b']);
    await client.contracts.getManyWithProof(['x']);
    expect(wasmSdk.getDataContracts).to.be.calledOnceWithExactly(['a', 'b']);
    expect(wasmSdk.getDataContractsWithProofInfo).to.be.calledOnceWithExactly(['x']);
  });

  it('publish() calls wasmSdk.contractPublish with options', async () => {
    const options = {
      ownerId: 'o',
      definition: { d: 1 },
      signer: { privateKeyWif: 'w', keyId: 2 },
    };
    await client.contracts.publish(options);
    expect(wasmSdk.contractPublish).to.be.calledOnceWithExactly(options);
  });

  it('update() calls wasmSdk.contractUpdate with options', async () => {
    const options = {
      contractId: 'c',
      ownerId: 'o',
      updates: { u: true },
      signer: { privateKeyWif: 'w', keyId: 4 },
    };
    await client.contracts.update(options);
    expect(wasmSdk.contractUpdate).to.be.calledOnceWithExactly(options);
  });
});
