import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import { EvoSDK } from '../../../dist/sdk.js';

describe('AddressesFacade', () => {
  let wasmSdk;
  let client;

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnetTrusted();
    wasmSdk = builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    this.sinon.stub(wasmSdk, 'getAddressInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getAddressInfoWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getAddressesInfos').resolves('ok');
    this.sinon.stub(wasmSdk, 'getAddressesInfosWithProofInfo').resolves('ok');
  });

  it('get() forwards address to getAddressInfo', async () => {
    const address = 'tdashevo1qr4nl2m5z7v7g7d2c4z6k8x9w3y2f5p6h0s1t4';
    await client.addresses.get(address);
    expect(wasmSdk.getAddressInfo).to.be.calledOnceWithExactly(address);
  });

  it('getWithProof() forwards address to getAddressInfoWithProofInfo', async () => {
    const address = 'tdashevo1qr4nl2m5z7v7g7d2c4z6k8x9w3y2f5p6h0s1t4';
    await client.addresses.getWithProof(address);
    expect(wasmSdk.getAddressInfoWithProofInfo).to.be.calledOnceWithExactly(address);
  });

  it('getMany() forwards array of addresses to getAddressesInfos', async () => {
    const addresses = [
      'tdashevo1qr4nl2m5z7v7g7d2c4z6k8x9w3y2f5p6h0s1t4',
      'tdashevo1abc123def456ghi789jkl012mno345pqr678st',
    ];
    await client.addresses.getMany(addresses);
    expect(wasmSdk.getAddressesInfos).to.be.calledOnceWithExactly(addresses);
  });

  it('getManyWithProof() forwards array of addresses to getAddressesInfosWithProofInfo', async () => {
    const addresses = [
      'tdashevo1qr4nl2m5z7v7g7d2c4z6k8x9w3y2f5p6h0s1t4',
    ];
    await client.addresses.getManyWithProof(addresses);
    expect(wasmSdk.getAddressesInfosWithProofInfo).to.be.calledOnceWithExactly(addresses);
  });

  it('get() accepts Uint8Array address', async () => {
    const addressBytes = new Uint8Array(21);
    await client.addresses.get(addressBytes);
    expect(wasmSdk.getAddressInfo).to.be.calledOnceWithExactly(addressBytes);
  });

  it('getMany() accepts mixed address formats', async () => {
    const bech32Address = 'tdashevo1qr4nl2m5z7v7g7d2c4z6k8x9w3y2f5p6h0s1t4';
    const bytesAddress = new Uint8Array(21);
    const mixedAddresses = [bech32Address, bytesAddress];
    await client.addresses.getMany(mixedAddresses);
    expect(wasmSdk.getAddressesInfos).to.be.calledOnceWithExactly(mixedAddresses);
  });
});

describe('PlatformAddress', () => {
  let PlatformAddress;

  before(async () => {
    await init();
    PlatformAddress = wasmSDKPackage.PlatformAddress;
  });

  it('can be created from bytes', () => {
    // Create P2PKH address from bytes (type 0x00)
    const p2pkhBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
    const p2pkhAddr = PlatformAddress.fromBytes(Array.from(p2pkhBytes));
    expect(p2pkhAddr).to.exist();
    expect(p2pkhAddr.addressType).to.equal('P2PKH');
    expect(p2pkhAddr.isP2pkh).to.be.true();
    expect(p2pkhAddr.isP2sh).to.be.false();

    // Create P2SH address from bytes (type 0x01)
    const p2shBytes = new Uint8Array([0x01, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
    const p2shAddr = PlatformAddress.fromBytes(Array.from(p2shBytes));
    expect(p2shAddr).to.exist();
    expect(p2shAddr.addressType).to.equal('P2SH');
    expect(p2shAddr.isP2pkh).to.be.false();
    expect(p2shAddr.isP2sh).to.be.true();
  });

  it('converts to bech32m and back', () => {
    // Create address from bytes
    const originalBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
    const addr = PlatformAddress.fromBytes(Array.from(originalBytes));

    // Convert to bech32m for testnet
    const bech32m = addr.toBech32m('testnet');
    expect(bech32m).to.be.a('string');
    expect(bech32m.startsWith('tdashevo1')).to.be.true();

    // Convert back from bech32m
    const recoveredAddr = PlatformAddress.fromBech32m(bech32m);
    expect(recoveredAddr).to.exist();
    expect(recoveredAddr.addressType).to.equal(addr.addressType);

    // Verify bytes match
    const recoveredBytes = recoveredAddr.toBytes();
    expect(Array.from(recoveredBytes)).to.deep.equal(Array.from(originalBytes));
  });
});
