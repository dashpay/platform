import { EvoSDK } from '../../dist/evo-sdk.module.js';

describe('EvoSDK', () => {
  it('exposes constructor and factories', () => {
    expect(EvoSDK).to.be.a('function');
    expect(EvoSDK.testnet).to.be.a('function');
    expect(EvoSDK.mainnet).to.be.a('function');
    expect(EvoSDK.testnetTrusted).to.be.a('function');
    expect(EvoSDK.mainnetTrusted).to.be.a('function');
  });

  it('fromWasm() marks instance as connected', () => {
    const wasmStub = { version: () => 1 };
    const sdk = EvoSDK.fromWasm(wasmStub);
    expect(sdk.isConnected).to.equal(true);
    expect(sdk.wasm).to.equal(wasmStub);
  });
});
