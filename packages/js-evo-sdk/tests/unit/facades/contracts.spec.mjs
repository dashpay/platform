import { EvoSDK } from '../../../dist/evo-sdk.module.js';
import init, * as wasmSDKPackage from "@dashevo/wasm-sdk"

describe('ContractsFacade', () => {
  let wasmSdk;
  let client;

  beforeEach(async function () {
    await init();
    let builder = wasmSDKPackage.WasmSdkBuilder.new_testnet_trusted();
    let wasmSdk = builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    // class methods
    this.sinon.stub(wasmSDKPackage, 'data_contract_fetch').resolves(true);
    this.sinon.stub(wasmSDKPackage, 'data_contract_fetch_with_proof_info').resolves(true);
    this.sinon.stub(wasmSDKPackage, 'get_data_contract_history').resolves(true);
    this.sinon.stub(wasmSDKPackage, 'get_data_contract_history_with_proof_info').resolves(true);
    this.sinon.stub(wasmSDKPackage, 'get_data_contracts').resolves(true);
    this.sinon.stub(wasmSDKPackage, 'get_data_contracts_with_proof_info').resolves(true);

    // instance methods
    this.sinon.stub(wasmSdk, 'contractCreate').resolves(true);
    this.sinon.stub(wasmSdk, 'contractUpdate').resolves(true);
  })

  it('fetch() forward to free functions', async () => {
    await client.contracts.fetch('c');

    expect(wasmSdk).to.be.calledOnceWithExactly('c');
  });

  it('fetchWithProof() forward to free functions', async () => {
    await client.contracts.fetchWithProof('c2');

    expect(wasmSdk).to.be.calledOnceWithExactly('c2');
  });

  it('getHistory() converts startAtMs to BigInt and forwards', async () => {
    await client.contracts.getHistory({ contractId: 'c', limit: 3, startAtMs: 5 });
    expect(wasmSdk.get_data_contract_history).to.be.calledOnce();
    const args = wasmSdk.get_data_contract_history.firstCall.args;
    expect(args[0]).to.equal(client.wasm);
    expect(args[1]).to.equal('c');
    expect(args[2]).to.equal(3);
    expect(args[3]).to.equal(null);
    expect(typeof args[4]).to.equal('bigint');
    expect(args[4]).to.equal(5n);
  });

  it('getHistoryWithProof() forwards similarly', async () => {
    await client.contracts.getHistoryWithProof({ contractId: 'c' });
    expect(wasmSdk.get_data_contract_history_with_proof_info).to.be.calledOnceWithExactly(client.wasm, 'c', null, null, null);
  });

  it('getMany() and getManyWithProof() forward arrays', async () => {
    await client.contracts.getMany(['a', 'b']);
    await client.contracts.getManyWithProof(['x']);
    expect(wasmSdk.get_data_contracts).to.be.calledOnceWithExactly(client.wasm, ['a', 'b']);
    expect(wasmSdk.get_data_contracts_with_proof_info).to.be.calledOnceWithExactly(client.wasm, ['x']);
  });

  it('create() calls wasmSdk.contractCreate with JSON', async () => {
    await client.contracts.create({ ownerId: 'o', definition: { d: 1 }, privateKeyWif: 'w', keyId: 2 });
    expect(wasmSdk.contractCreate).to.be.calledOnceWithExactly('o', JSON.stringify({ d: 1 }), 'w', 2);
  });

  it('update() calls wasmSdk.contractUpdate with JSON', async () => {
    await client.contracts.update({ contractId: 'c', ownerId: 'o', updates: { u: true }, privateKeyWif: 'w', keyId: 4 });
    expect(wasmSdk.contractUpdate).to.be.calledOnceWithExactly('c', 'o', JSON.stringify({ u: true }), 'w', 4);
  });
});
