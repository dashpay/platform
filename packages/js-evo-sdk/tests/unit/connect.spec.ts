import type { SinonStub } from 'sinon';
import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import { EvoSDK } from '../../dist/sdk.js';

// RFC 6761 reserved test domain: the builder is stubbed, so nothing is dialled.
const TEST_ADDRESS = 'https://node-1.test:1443';
const QUORUM_URL = 'https://quorums.example';

describe('EvoSDK.connect()', () => {
  let context: wasmSDKPackage.WasmTrustedContext;
  let builder: {
    withTrustedContext: SinonStub;
    withVerifiedContext: SinonStub;
    withSettings: SinonStub;
    build: SinonStub;
  };
  let withAddressesStub: SinonStub;
  let testnetBuilderStub: SinonStub;
  let mainnetBuilderStub: SinonStub;
  let prefetchDevnetStub: SinonStub;
  let prefetchDevnetWithUrlStub: SinonStub;
  let prefetchTestnetStub: SinonStub;

  beforeEach(async function setup() {
    await init();
    context = {} as wasmSDKPackage.WasmTrustedContext;
    builder = {
      withTrustedContext: this.sinon.stub(),
      withVerifiedContext: this.sinon.stub(),
      withSettings: this.sinon.stub(),
      build: this.sinon.stub(),
    };
    builder.withTrustedContext.returns(builder);
    builder.withVerifiedContext.returns(builder);
    builder.withSettings.returns(builder);
    builder.build.returns({ version: () => 1 });
    const fakeBuilder = builder as unknown as wasmSDKPackage.WasmSdkBuilder;

    withAddressesStub = this.sinon.stub(wasmSDKPackage.WasmSdkBuilder, 'withAddresses').returns(fakeBuilder);
    testnetBuilderStub = this.sinon.stub(wasmSDKPackage.WasmSdkBuilder, 'testnet').returns(fakeBuilder);
    mainnetBuilderStub = this.sinon.stub(wasmSDKPackage.WasmSdkBuilder, 'mainnet').returns(fakeBuilder);
    prefetchDevnetStub = this.sinon.stub(wasmSDKPackage.WasmTrustedContext, 'prefetchDevnet').resolves(context);
    prefetchDevnetWithUrlStub = this.sinon.stub(wasmSDKPackage.WasmTrustedContext, 'prefetchDevnetWithUrl').resolves(context);
    prefetchTestnetStub = this.sinon.stub(wasmSDKPackage.WasmTrustedContext, 'prefetchTestnet').resolves(context);
  });

  it('should skip masternode discovery when explicit addresses are given', async () => {
    const sdk = new EvoSDK({
      network: 'devnet',
      devnetName: 'paloma',
      addresses: [TEST_ADDRESS],
      trusted: true,
    });

    await sdk.connect();

    // Explicit addresses win in withTrustedContext, so the prefetch is told
    // not to spend a round trip discovering ones that would be discarded.
    expect(prefetchDevnetStub).to.be.calledOnceWithExactly('paloma', false);
    expect(withAddressesStub).to.be.calledOnceWithExactly([TEST_ADDRESS], 'devnet');
    expect(builder.withTrustedContext).to.be.calledOnceWithExactly(context);
    expect(sdk.isConnected).to.equal(true);
  });

  it('should skip masternode discovery on the quorumUrl path as well', async () => {
    const sdk = new EvoSDK({
      network: 'devnet',
      quorumUrl: QUORUM_URL,
      addresses: [TEST_ADDRESS],
      trusted: true,
    });

    await sdk.connect();

    expect(prefetchDevnetWithUrlStub).to.be.calledOnceWithExactly(QUORUM_URL, false);
    expect(prefetchDevnetStub).to.not.have.been.called();
    expect(builder.withTrustedContext).to.be.calledOnceWithExactly(context);
  });

  it('should keep masternode discovery when no addresses are given', async () => {
    const sdk = EvoSDK.testnetTrusted();

    await sdk.connect();

    expect(prefetchTestnetStub).to.be.calledOnceWithExactly(true);
    expect(testnetBuilderStub).to.be.calledOnce();
    expect(withAddressesStub).to.not.have.been.called();
    expect(builder.withTrustedContext).to.be.calledOnceWithExactly(context);
  });

  describe('with verified: true', () => {
    const CHECKPOINT = {
      network: 0,
      height: 2300000,
      block_hash: '00000000000000000000000000000000000000000000000000000000000000aa',
      masternode_root: '00000000000000000000000000000000000000000000000000000000000000bb',
      quorum_root: '00000000000000000000000000000000000000000000000000000000000000cc',
    };
    let verifiedContext: wasmSDKPackage.WasmVerifiedContext;

    beforeEach(function setupVerified() {
      verifiedContext = {
        continuation: this.sinon.stub().returns({ anchor: CHECKPOINT, states: [] }),
      } as unknown as wasmSDKPackage.WasmVerifiedContext;
    });

    it('should attach a verified context from the embedded checkpoint and prefetch nothing', async function test() {
      const factory = this.sinon.stub(wasmSDKPackage.WasmVerifiedContext, 'testnet').returns(verifiedContext);
      const sdk = EvoSDK.testnetVerified();

      await sdk.connect();

      expect(factory).to.be.calledOnceWithExactly(undefined);
      expect(testnetBuilderStub).to.be.calledOnce();
      expect(builder.withVerifiedContext).to.be.calledOnceWithExactly(verifiedContext);
      expect(builder.withTrustedContext).to.not.have.been.called();
      expect(prefetchTestnetStub).to.not.have.been.called();
      expect(sdk.verifiedContinuation()).to.deep.equal({ anchor: CHECKPOINT, states: [] });
    });

    it('should pass a saved checkpoint and relays to the mainnet factory', async function test() {
      const factory = this.sinon.stub(wasmSDKPackage.WasmVerifiedContext, 'mainnetWithCheckpoint')
        .returns(verifiedContext);
      const relays = ['https://relay.test'];
      const sdk = EvoSDK.mainnetVerified({ checkpoint: CHECKPOINT, quorumRelays: relays });

      await sdk.connect();

      expect(factory).to.be.calledOnceWithExactly(CHECKPOINT, relays);
      expect(mainnetBuilderStub).to.be.calledOnce();
      expect(builder.withVerifiedContext).to.be.calledOnceWithExactly(verifiedContext);
    });

    it('should resume from a saved continuation', async function test() {
      const factory = this.sinon.stub(wasmSDKPackage.WasmVerifiedContext, 'testnetFromContinuation')
        .returns(verifiedContext);
      const continuation = { anchor: CHECKPOINT, states: [{ ...CHECKPOINT, height: 2300100 }] };
      const sdk = EvoSDK.testnetVerified({ continuation });

      await sdk.connect();

      expect(factory).to.be.calledOnceWithExactly(continuation, undefined);
      expect(builder.withVerifiedContext).to.be.calledOnceWithExactly(verifiedContext);
    });

    it('should keep explicit addresses', async function test() {
      this.sinon.stub(wasmSDKPackage.WasmVerifiedContext, 'testnet').returns(verifiedContext);
      const sdk = new EvoSDK({ network: 'testnet', verified: true, addresses: [TEST_ADDRESS] });

      await sdk.connect();

      expect(withAddressesStub).to.be.calledOnceWithExactly([TEST_ADDRESS], 'testnet');
      expect(builder.withVerifiedContext).to.be.calledOnceWithExactly(verifiedContext);
    });
  });
});
