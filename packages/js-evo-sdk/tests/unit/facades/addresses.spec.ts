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
      new Uint8Array([0xb0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]),
    );
    this.sinon.stub(wasmSdk, 'addressFundsTransfer').resolves({
      type: 'VerifiedAddressInfos',
      addressInfos: [{ address: mockAddress, nonce: 1, balance: '90000' }],
    });
  });

  it('get() forwards address to getAddressInfo', async () => {
    const address = 'tevo1qr4nl2m5z7v7g7d2c4z6k8x9w3y2f5p6h0s1t4';
    await client.addresses.get(address);
    expect(wasmSdk.getAddressInfo).to.be.calledOnceWithExactly(address);
  });

  it('getWithProof() forwards address to getAddressInfoWithProofInfo', async () => {
    const address = 'tevo1qr4nl2m5z7v7g7d2c4z6k8x9w3y2f5p6h0s1t4';
    await client.addresses.getWithProof(address);
    expect(wasmSdk.getAddressInfoWithProofInfo).to.be.calledOnceWithExactly(address);
  });

  it('getMany() forwards array of addresses to getAddressesInfos', async () => {
    const addresses = [
      'tevo1qr4nl2m5z7v7g7d2c4z6k8x9w3y2f5p6h0s1t4',
      'tevo1abc123def456ghi789jkl012mno345pqr678st',
    ];
    await client.addresses.getMany(addresses);
    expect(wasmSdk.getAddressesInfos).to.be.calledOnceWithExactly(addresses);
  });

  it('getManyWithProof() forwards array of addresses to getAddressesInfosWithProofInfo', async () => {
    const addresses = [
      'tevo1qr4nl2m5z7v7g7d2c4z6k8x9w3y2f5p6h0s1t4',
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
    const bech32Address = 'tevo1qr4nl2m5z7v7g7d2c4z6k8x9w3y2f5p6h0s1t4';
    const bytesAddress = new Uint8Array(21);
    const mixedAddresses = [bech32Address, bytesAddress];
    await client.addresses.getMany(mixedAddresses);
    expect(wasmSdk.getAddressesInfos).to.be.calledOnceWithExactly(mixedAddresses);
  });

  it('transfer() forwards options to addressFundsTransfer with PlatformAddressSigner', async () => {
    // Create proper PlatformAddress objects
    const recipientAddr = wasmSDKPackage.PlatformAddress.fromBytes(
      new Uint8Array([0xb0, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1]),
    );

    // Create a PrivateKey object from bytes (32 bytes for testnet)
    const privateKeyBytes = new Uint8Array(32).fill(1); // Test key bytes
    const privateKey = wasmSDKPackage.PrivateKey.fromBytes(privateKeyBytes, 'testnet');

    // Create signer and derive the sender address from the private key
    const signer = new wasmSDKPackage.PlatformAddressSigner();
    const derivedSenderAddr = signer.addKey(privateKey);

    // Create typed input and output objects (address, nonce, amount)
    const input = new wasmSDKPackage.PlatformAddressInput(derivedSenderAddr, 0, 100000n);
    const output = new wasmSDKPackage.PlatformAddressOutput(recipientAddr, 90000n);

    const options = {
      inputs: [input],
      outputs: [output],
      signer,
    };
    const result = await client.addresses.transfer(options);
    expect(wasmSdk.addressFundsTransfer).to.be.calledOnce();
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
    const recipientAddr = wasmSDKPackage.PlatformAddress.fromBytes(
      new Uint8Array([0xb0, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1]),
    );

    // Create a PrivateKey object from bytes
    const privateKeyBytes = new Uint8Array(32).fill(1); // Test key bytes
    const privateKey = wasmSDKPackage.PrivateKey.fromBytes(privateKeyBytes, 'testnet');

    // Create signer and derive the sender address from the private key
    const signer = new wasmSDKPackage.PlatformAddressSigner();
    const derivedSenderAddr = signer.addKey(privateKey);

    // Create typed input and output objects (address, nonce, amount)
    const input = new wasmSDKPackage.PlatformAddressInput(derivedSenderAddr, 0, 100000n);
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

  it('topUpIdentity() forwards options to identityTopUpFromAddresses', async function topUpTest() {
    // Create mock address and result
    const mockAddress = wasmSDKPackage.PlatformAddress.fromBytes(
      new Uint8Array([0xb0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]),
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
    expect(wasmSdk.identityTopUpFromAddresses).to.be.calledOnce();
    expect(result.newBalance).to.equal(150000n);
  });

  it('withdraw() forwards options to addressFundsWithdraw', async function withdrawTest() {
    // Create mock address and result map
    const mockAddress = wasmSDKPackage.PlatformAddress.fromBytes(
      new Uint8Array([0xb0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]),
    );

    const resultMap = new Map();
    resultMap.set(mockAddress, { address: mockAddress, nonce: 1n, balance: 0n });

    this.sinon.stub(wasmSdk, 'addressFundsWithdraw').resolves(resultMap);

    // Mock options - the actual implementation will process these
    const options = {
      inputs: [],
      coreFeePerByte: 1,
      pooling: wasmSDKPackage.PoolingWasm.Standard,
      outputScript: new Uint8Array(25).fill(5), // Mock P2PKH script bytes
      signer: {},
    };

    const result = await client.addresses.withdraw(options);
    expect(wasmSdk.addressFundsWithdraw).to.be.calledOnce();
    expect(result).to.be.instanceOf(Map);
  });

  it('transferFromIdentity() forwards options to identityTransferToAddresses', async function transferFromIdentityTest() {
    // Create mock address
    const mockAddress = wasmSDKPackage.PlatformAddress.fromBytes(
      new Uint8Array([0xb0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]),
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
    expect(wasmSdk.identityTransferToAddresses).to.be.calledOnce();
    expect(result.newBalance).to.equal(400000n);
  });

  it('fundFromAssetLock() forwards options to addressFundingFromAssetLock', async function fundFromAssetLockTest() {
    // Create mock address and result map
    const mockAddress = wasmSDKPackage.PlatformAddress.fromBytes(
      new Uint8Array([0xb0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]),
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
    expect(wasmSdk.addressFundingFromAssetLock).to.be.calledOnce();
    expect(result).to.be.instanceOf(Map);
  });

  it('createIdentity() forwards options to identityCreateFromAddresses', async function createIdentityTest() {
    // Create mock address
    const mockAddress = wasmSDKPackage.PlatformAddress.fromBytes(
      new Uint8Array([0xb0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]),
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
    expect(wasmSdk.identityCreateFromAddresses).to.be.calledOnce();
    expect(result.identity).to.exist();
  });
});
