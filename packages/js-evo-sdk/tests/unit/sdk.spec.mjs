import { EvoSDK } from '../../dist/sdk.js';

describe('EvoSDK', () => {
  it('exposes constructor and factories', () => {
    expect(EvoSDK).to.be.a('function');
    expect(EvoSDK.testnet).to.be.a('function');
    expect(EvoSDK.mainnet).to.be.a('function');
    expect(EvoSDK.testnetTrusted).to.be.a('function');
    expect(EvoSDK.mainnetTrusted).to.be.a('function');
  });

  it('connects and sets isConnected', async function () {
    this.timeout(90000);

    const sdk = new EvoSDK();
    expect(sdk.isConnected).to.equal(false);
    await sdk.connect();
    expect(sdk.isConnected).to.equal(true);
  });
});

