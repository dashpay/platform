import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import { EvoSDK } from '../../../dist/sdk.js';

describe('ProtocolFacade', () => {
  let wasmSdk;
  let client;

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnetTrusted();
    wasmSdk = builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    this.sinon.stub(wasmSdk, 'getProtocolVersionUpgradeState').resolves('ok');
    this.sinon.stub(wasmSdk, 'getProtocolVersionUpgradeStateWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getProtocolVersionUpgradeVoteStatus').resolves('ok');
    this.sinon.stub(wasmSdk, 'getProtocolVersionUpgradeVoteStatusWithProofInfo').resolves('ok');
  });

  it('versionUpgradeState and versionUpgradeStateWithProof forward', async () => {
    await client.protocol.versionUpgradeState();
    await client.protocol.versionUpgradeStateWithProof();
    expect(wasmSdk.getProtocolVersionUpgradeState).to.be.calledOnce();
    expect(wasmSdk.getProtocolVersionUpgradeStateWithProofInfo).to.be.calledOnce();
  });

  it('versionUpgradeVoteStatus and withProof forward with positional args', async () => {
    await client.protocol.versionUpgradeVoteStatus('h', 5);
    await client.protocol.versionUpgradeVoteStatusWithProof('g', 3);
    expect(wasmSdk.getProtocolVersionUpgradeVoteStatus).to.be.calledOnceWithExactly('h', 5);
    expect(wasmSdk.getProtocolVersionUpgradeVoteStatusWithProofInfo).to.be.calledOnceWithExactly('g', 3);
  });

  it('versionUpgradeVoteStatus accepts Uint8Array and positional args', async () => {
    const bytes = new Uint8Array([0xde, 0xad, 0xbe, 0xef]);
    await client.protocol.versionUpgradeVoteStatus(bytes, 2);
    await client.protocol.versionUpgradeVoteStatusWithProof(bytes, 4);
    expect(wasmSdk.getProtocolVersionUpgradeVoteStatus).to.be.calledWith(bytes, 2);
    expect(wasmSdk.getProtocolVersionUpgradeVoteStatusWithProofInfo).to.be.calledWith(bytes, 4);
  });
});
