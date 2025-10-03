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
    this.sinon.stub(wasmSdk, 'contractCreate').resolves(true);
    this.sinon.stub(wasmSdk, 'contractUpdate').resolves(true);
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

  it('getHistory() converts startAtMs to BigInt and forwards', async () => {
    await client.contracts.getHistory({
      contractId: 'c',
      limit: 3,
      startAtMs: 5,
    });
    expect(wasmSdk.getDataContractHistory).to.be.calledOnce();
    const { args } = wasmSdk.getDataContractHistory.firstCall;
    expect(args[0]).to.equal('c');
    expect(args[1]).to.equal(3);
    expect(args[2]).to.equal(null);
    expect(typeof args[3]).to.equal('bigint');
    expect(args[3]).to.equal(BigInt(5));
  });

  it('getHistoryWithProof() forwards similarly', async () => {
    await client.contracts.getHistoryWithProof({ contractId: 'c' });
    expect(wasmSdk.getDataContractHistoryWithProofInfo).to.be.calledOnceWithExactly('c', null, null, null);
  });

  it('getMany() and getManyWithProof() forward arrays', async () => {
    await client.contracts.getMany(['a', 'b']);
    await client.contracts.getManyWithProof(['x']);
    expect(wasmSdk.getDataContracts).to.be.calledOnceWithExactly(['a', 'b']);
    expect(wasmSdk.getDataContractsWithProofInfo).to.be.calledOnceWithExactly(['x']);
  });

  it('create() calls wasmSdk.contractCreate with JSON', async () => {
    await client.contracts.create({
      ownerId: 'o',
      definition: { d: 1 },
      privateKeyWif: 'w',
      keyId: 2,
    });
    expect(wasmSdk.contractCreate).to.be.calledOnceWithExactly('o', JSON.stringify({ d: 1 }), 'w', 2);
  });

  it('update() calls wasmSdk.contractUpdate with JSON', async () => {
    await client.contracts.update({
      contractId: 'c',
      ownerId: 'o',
      updates: { u: true },
      privateKeyWif: 'w',
      keyId: 4,
    });
    expect(wasmSdk.contractUpdate).to.be.calledOnceWithExactly('c', 'o', JSON.stringify({ u: true }), 'w', 4);
  });
});
