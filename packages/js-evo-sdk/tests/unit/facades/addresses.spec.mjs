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
    // Create a mock PlatformAddress for test results
    const mockAddress = wasmSDKPackage.PlatformAddress.fromBytes(
      new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]),
    );
    this.sinon.stub(wasmSdk, 'addressFundsTransfer').resolves({
      type: 'VerifiedAddressInfos',
      addressInfos: [{ address: mockAddress, nonce: 1, balance: '90000' }],
    });
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

  it('transfer() forwards options to addressFundsTransfer with PlatformAddressSigner', async () => {
    // Create proper PlatformAddress objects
    const senderAddr = wasmSDKPackage.PlatformAddress.fromBytes(
      new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]),
    );
    const recipientAddr = wasmSDKPackage.PlatformAddress.fromBytes(
      new Uint8Array([0x00, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1]),
    );

    // Create a PrivateKey object from bytes (32 bytes for testnet)
    const privateKeyBytes = new Uint8Array(32).fill(1); // Test key bytes
    const privateKey = wasmSDKPackage.PrivateKey.fromBytes(privateKeyBytes, 'testnet');

    // Create signer and add the private key
    const signer = new wasmSDKPackage.PlatformAddressSigner();
    signer.addKey(senderAddr, privateKey);

    // Create typed input and output objects (address, nonce, amount)
    const input = new wasmSDKPackage.PlatformAddressInput(senderAddr, 0n, 100000n);
    const output = new wasmSDKPackage.PlatformAddressOutput(recipientAddr, 90000n);

    const options = {
      inputs: [input],
      outputs: [output],
      signer,
    };
    const result = await client.addresses.transfer(options);
    expect(wasmSdk.addressFundsTransfer).to.be.calledOnce;
    expect(result.type).to.equal('VerifiedAddressInfos');
    expect(result.addressInfos).to.have.lengthOf(1);
    expect(result.addressInfos[0].address.addressType).to.equal('P2PKH');
  });

  it('transfer() handles success result type', async () => {
    wasmSdk.addressFundsTransfer.resolves({
      type: 'Success',
      message: 'Address funds transfer completed successfully',
    });

    // Create proper PlatformAddress objects
    const senderAddr = wasmSDKPackage.PlatformAddress.fromBytes(
      new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]),
    );
    const recipientAddr = wasmSDKPackage.PlatformAddress.fromBytes(
      new Uint8Array([0x00, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1]),
    );

    // Create a PrivateKey object from bytes
    const privateKeyBytes = new Uint8Array(32).fill(1); // Test key bytes
    const privateKey = wasmSDKPackage.PrivateKey.fromBytes(privateKeyBytes, 'testnet');

    // Create signer and add the private key
    const signer = new wasmSDKPackage.PlatformAddressSigner();
    signer.addKey(senderAddr, privateKey);

    // Create typed input and output objects (address, nonce, amount)
    const input = new wasmSDKPackage.PlatformAddressInput(senderAddr, 0n, 100000n);
    const output = new wasmSDKPackage.PlatformAddressOutput(recipientAddr, 90000n);

    const options = {
      inputs: [input],
      outputs: [output],
      signer,
    };
    const result = await client.addresses.transfer(options);
    expect(result.type).to.equal('Success');
    expect(result.message).to.include('successfully');
  });

  it('topUpIdentity() forwards options to identityTopUpFromAddresses', async function () {
    // Create mock address and result
    const mockAddress = wasmSDKPackage.PlatformAddress.fromBytes(
      new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]),
    );

    this.sinon.stub(wasmSdk, 'identityTopUpFromAddresses').resolves({
      addressInfos: new Map([[mockAddress, { address: mockAddress, nonce: 1n, balance: 50000n }]]),
      newBalance: 150000n,
    });

    // Create a mock identity ID (32 bytes)
    const identityId = wasmSDKPackage.Identifier.fromBytes(new Uint8Array(32).fill(42));

    // Mock options - the actual implementation will process these
    const options = {
      identityId,
      inputs: [],
      signer: {},
    };

    const result = await client.addresses.topUpIdentity(options);
    expect(wasmSdk.identityTopUpFromAddresses).to.be.calledOnce;
    expect(result.newBalance).to.equal(150000n);
  });

  it('withdraw() forwards options to addressFundsWithdraw', async function () {
    // Create mock address and result map
    const mockAddress = wasmSDKPackage.PlatformAddress.fromBytes(
      new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]),
    );

    const resultMap = new Map();
    resultMap.set(mockAddress, { address: mockAddress, nonce: 1n, balance: 0n });

    this.sinon.stub(wasmSdk, 'addressFundsWithdraw').resolves(resultMap);

    // Create Core output script (P2PKH)
    const coreScript = wasmSDKPackage.CoreScript.newP2PKH(new Uint8Array(20).fill(5));

    // Mock options - the actual implementation will process these
    const options = {
      inputs: [],
      coreFeePerByte: 1,
      pooling: wasmSDKPackage.PoolingWasm.Standard,
      outputScript: coreScript,
      signer: {},
    };

    const result = await client.addresses.withdraw(options);
    expect(wasmSdk.addressFundsWithdraw).to.be.calledOnce;
    expect(result).to.be.instanceOf(Map);
  });

  it('transferFromIdentity() forwards options to identityTransferToAddresses', async function () {
    // Create mock address
    const mockAddress = wasmSDKPackage.PlatformAddress.fromBytes(
      new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]),
    );

    this.sinon.stub(wasmSdk, 'identityTransferToAddresses').resolves({
      addressInfos: new Map([[mockAddress, { address: mockAddress, nonce: 0n, balance: 100000n }]]),
      newBalance: 400000n,
    });

    // Create a mock identity ID
    const identityId = wasmSDKPackage.Identifier.fromBytes(new Uint8Array(32).fill(42));

    // Mock options - the actual implementation will process these
    const options = {
      identityId,
      outputs: [],
      signer: {},
    };

    const result = await client.addresses.transferFromIdentity(options);
    expect(wasmSdk.identityTransferToAddresses).to.be.calledOnce;
    expect(result.newBalance).to.equal(400000n);
  });

  it('fundFromAssetLock() forwards options to addressFundingFromAssetLock', async function () {
    // Create mock address and result map
    const mockAddress = wasmSDKPackage.PlatformAddress.fromBytes(
      new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]),
    );

    const resultMap = new Map();
    resultMap.set(mockAddress, { address: mockAddress, nonce: 0n, balance: 100000n });

    this.sinon.stub(wasmSdk, 'addressFundingFromAssetLock').resolves(resultMap);

    // Mock options - the actual implementation will process these
    const options = {
      assetLockProof: {},
      assetLockPrivateKey: {},
      outputs: [],
      signer: {},
    };

    const result = await client.addresses.fundFromAssetLock(options);
    expect(wasmSdk.addressFundingFromAssetLock).to.be.calledOnce;
    expect(result).to.be.instanceOf(Map);
  });

  it('createIdentity() forwards options to identityCreateFromAddresses', async function () {
    // Create mock address
    const mockAddress = wasmSDKPackage.PlatformAddress.fromBytes(
      new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]),
    );

    // Create mock identity ID
    const mockIdentityId = wasmSDKPackage.Identifier.fromBytes(new Uint8Array(32).fill(99));

    this.sinon.stub(wasmSdk, 'identityCreateFromAddresses').resolves({
      identity: { id: () => mockIdentityId },
      addressInfos: new Map([[mockAddress, { address: mockAddress, nonce: 1n, balance: 0n }]]),
    });

    // Mock options - the actual implementation will process these
    const options = {
      identity: {},
      inputs: [],
      identitySigner: {},
      addressSigner: {},
    };

    const result = await client.addresses.createIdentity(options);
    expect(wasmSdk.identityCreateFromAddresses).to.be.calledOnce;
    expect(result.identity).to.exist();
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
    const p2pkhAddr = PlatformAddress.fromBytes(p2pkhBytes);
    expect(p2pkhAddr).to.exist();
    expect(p2pkhAddr.addressType).to.equal('P2PKH');
    expect(p2pkhAddr.isP2pkh).to.be.true();
    expect(p2pkhAddr.isP2sh).to.be.false();

    // Create P2SH address from bytes (type 0x01)
    const p2shBytes = new Uint8Array([0x01, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
    const p2shAddr = PlatformAddress.fromBytes(p2shBytes);
    expect(p2shAddr).to.exist();
    expect(p2shAddr.addressType).to.equal('P2SH');
    expect(p2shAddr.isP2pkh).to.be.false();
    expect(p2shAddr.isP2sh).to.be.true();
  });

  it('converts to bech32m and back', () => {
    // Create address from bytes
    const originalBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
    const addr = PlatformAddress.fromBytes(originalBytes);

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
