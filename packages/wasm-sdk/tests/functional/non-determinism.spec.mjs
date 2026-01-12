import init, * as sdk from '../../dist/sdk.compressed.js';
import { wasmFunctionalTestRequirements, createTestSignerAndKey } from './fixtures/requiredTestData.mjs';

/**
 * Non-determinism reproduction test suite.
 *
 * This test suite attempts to reproduce non-determinism issues by ensuring
 * each state transition goes into its own block (3 second delay between each).
 *
 * Steps:
 * 1. Verify identity has TRANSFER key (added in genesis with SDK_TEST_DATA=true)
 * 2. Fund 5 platform addresses from identity (one per block, 3s delay between each)
 * 3. Wait 3 seconds
 * 4. Move funds from 5 addresses to 5 secondary addresses (one per block, 3s delay)
 * 5. Wait 3 seconds
 * 6. Withdraw from 5 secondary addresses to core (one per block, 3s delay)
 *
 * Total state transitions: 5 (funding) + 5 (transfers) + 5 (withdrawals) = 15
 * Each in its own block for maximum non-determinism exposure.
 *
 * NOTE: Address transfer fees are expensive (6M per output), withdrawal fees are
 * very expensive (400M base). Each address is funded with 500M credits.
 *
 * Test requires SDK_TEST_DATA=true when starting the platform.
 */

describe.only('Non-determinism Reproduction', function describeNonDeterminism() {
  this.timeout(300000); // 5 minutes for the full test
  this.bail(true);

  let client;
  const testData = wasmFunctionalTestRequirements();

  // Storage for generated addresses and their private key bytes
  // NOTE: Fee structure is expensive for address transfers:
  // - Input cost: 500,000 credits per input
  // - Output cost: 6,000,000 credits per output
  // So we use fewer addresses to keep fees manageable
  const initialAddresses = []; // 5 addresses (bech32m strings)
  const initialPrivateKeyBytes = []; // Private key bytes for initial 5 addresses
  const secondaryAddresses = []; // 5 addresses (bech32m strings)
  const secondaryPrivateKeyBytes = []; // Private key bytes for secondary 5 addresses

  // Transfer key that we'll add to the identity
  let transferKeyPair = null;
  let transferKeyId = null;

  before(async () => {
    await init();
    await sdk.WasmSdk.prefetchTrustedQuorumsLocal();
    const builder = sdk.WasmSdkBuilder.localTrusted();
    client = await builder.build();

    // Generate 5 initial addresses with private keys
    // Store key bytes instead of WASM objects to avoid lifecycle issues
    for (let i = 0; i < 5; i++) {
      const keyBytes = new Uint8Array(32);
      // Use deterministic seed based on index for reproducibility
      keyBytes.fill(0);
      keyBytes[0] = 0xa0 + i; // Unique first byte for each key
      keyBytes[31] = i;

      const privateKey = sdk.PrivateKey.fromBytes(keyBytes, 'regtest');
      const signer = new sdk.PlatformAddressSigner();
      const address = signer.addKey(privateKey);

      // Store raw bytes for later use
      initialPrivateKeyBytes.push(new Uint8Array(keyBytes));
      initialAddresses.push(address.toBech32m('regtest'));
    }

    // Generate 5 secondary addresses with private keys
    for (let i = 0; i < 5; i++) {
      const keyBytes = new Uint8Array(32);
      // Use deterministic seed based on index for reproducibility
      keyBytes.fill(0);
      keyBytes[0] = 0xb0 + i; // Unique first byte for each key
      keyBytes[31] = i;

      const privateKey = sdk.PrivateKey.fromBytes(keyBytes, 'regtest');
      const signer = new sdk.PlatformAddressSigner();
      const address = signer.addKey(privateKey);

      // Store raw bytes for later use
      secondaryPrivateKeyBytes.push(new Uint8Array(keyBytes));
      secondaryAddresses.push(address.toBech32m('regtest'));
    }

    console.log('Generated 5 initial addresses and 5 secondary addresses');
  });

  after(async () => {
    await new Promise((resolve) => setTimeout(resolve, 100));
    if (client) {
      try {
        client.free();
      } catch (err) {
        console.log('Note: client.free() error (may be harmless):', err.message);
      }
    }
  });

  it('Step 0: Verify identity has TRANSFER key', async function testVerifyTransferKey() {
    // Get identity 1
    const identity = await client.getIdentity(testData.identityId);
    expect(identity).to.exist;

    // Check identity balance - must have credits from SDK_TEST_DATA=true genesis
    // Note: balance is a getter property, not a method
    const balance = identity.balance;
    console.log(`Identity balance: ${balance} credits`);
    if (balance === 0n || balance === 0) {
      throw new Error(
        'Identity has 0 balance. Make sure platform was started with SDK_TEST_DATA=true. '
        + 'Run: yarn reset && SDK_TEST_DATA=true yarn start',
      );
    }

    // Check if identity has a TRANSFER key (should be added in genesis with SDK_TEST_DATA=true)
    const publicKeys = identity.getPublicKeys();
    console.log(`Identity has ${publicKeys.length} public keys`);

    const existingTransferKey = publicKeys.find((key) => key.purpose === 'TRANSFER');
    if (!existingTransferKey) {
      throw new Error(
        'Identity does not have a TRANSFER key. Make sure platform was started with SDK_TEST_DATA=true.',
      );
    }

    console.log(`Found TRANSFER key with ID ${existingTransferKey.keyId}`);
    transferKeyId = existingTransferKey.keyId;

    // Get the private key for the transfer key from test data (key index 3)
    // The genesis creates this key deterministically
    const { keyInfo } = createTestSignerAndKey(sdk, 1, 3);
    const privateKeyHex = keyInfo.get('privateKeyHex');

    // Store the key pair info for use in Step 1
    transferKeyPair = { privateKeyHex };

    console.log(`Step 0 complete: TRANSFER key verified with ID ${transferKeyId}`);
  });

  it('Step 1: Fund 5 platform addresses from identity (one per block)', async function testFundAddresses() {
    expect(transferKeyId).to.exist;
    expect(transferKeyPair).to.exist;

    // Get identity 1 which has funds
    const identity = await client.getIdentity(testData.identityId);
    expect(identity).to.exist;

    console.log(`Identity balance: ${identity.balance}`);

    // Fund with significantly more to cover:
    // - Fee for 1→1 transfers: 1 * 500k + 1 * 6M = 6.5M per transfer
    // - Fee for 1 address withdrawal: ~400M base + 1 * 500k = 400.5M per withdrawal
    // Fund 500M per address to ensure enough for withdrawal fees
    const amountPerAddress = 500000000n;
    const totalNeeded = amountPerAddress * 5n + 100000000n; // Extra for identity transfer fees

    if (BigInt(identity.balance) < totalNeeded) {
      throw new Error(
        `Insufficient identity balance: ${identity.balance}. Need at least ${totalNeeded} credits.`,
      );
    }

    // Create signer with the TRANSFER key
    const signer = new sdk.IdentitySigner();
    const transferPrivateKey = sdk.PrivateKey.fromHex(transferKeyPair.privateKeyHex, 'regtest');
    signer.addKey(transferPrivateKey);

    // Fund each address separately with 3s delay between each (one state transition per block)
    for (let i = 0; i < initialAddresses.length; i++) {
      // Refresh nonce before each transaction
      await client.refreshIdentityNonce(sdk.Identifier.fromBase58(testData.identityId));

      const outputs = [{
        address: initialAddresses[i],
        amount: amountPerAddress,
      }];

      console.log(`Funding address ${i + 1}/5...`);

      const result = await client.identityTransferToAddresses({
        identity,
        outputs,
        signer,
        signingTransferKeyId: transferKeyId,
      });

      expect(result).to.exist;
      console.log(`Address ${i + 1} funded, identity balance: ${result.newBalance}`);

      // Wait 3 seconds before next transaction (except after last one)
      if (i < initialAddresses.length - 1) {
        console.log('Waiting 3 seconds for next block...');
        await new Promise((resolve) => setTimeout(resolve, 3000));
      }
    }

    console.log('Step 1 complete: 5 addresses funded (one per block)');
  });

  it('Step 2: Wait 3 seconds', async function testWait1() {
    console.log('Waiting 3 seconds for platform to process...');
    await new Promise((resolve) => setTimeout(resolve, 3000));
    console.log('Wait complete');
  });

  it('Step 3: Move funds from 5 addresses to 5 addresses (one per block)', async function testTransferToSecondary() {
    // Fee for 1 input + 1 output = 500k + 6M = 6.5M
    const transferFee = 7000000n; // ~6.5M + buffer
    const minTransferAmount = 500000n;

    // Transfer from each initial address to corresponding secondary address
    for (let i = 0; i < initialAddresses.length; i++) {
      // Create signer with just this address's private key
      const inputSigner = new sdk.PlatformAddressSigner();
      const privateKey = sdk.PrivateKey.fromBytes(initialPrivateKeyBytes[i], 'regtest');
      inputSigner.addKey(privateKey);

      // Get current balance for this input address
      const inputInfos = await client.getAddressesInfos([initialAddresses[i]]);
      let balance = 0n;
      for (const [, info] of inputInfos.entries()) {
        if (info && info.balance > 0n) {
          balance = info.balance;
          break;
        }
      }

      if (balance === 0n) {
        throw new Error(`Address ${i} has no balance`);
      }

      // Check we have enough for fee + minimum transfer
      if (balance < transferFee + minTransferAmount) {
        throw new Error(`Address ${i} balance (${balance}) insufficient for transfer`);
      }

      // Transfer minimum amount, rest stays to cover fee
      const transferAmount = minTransferAmount;

      const inputs = [{
        address: initialAddresses[i],
        amount: transferAmount,
      }];

      const outputs = [{
        address: secondaryAddresses[i],
        amount: transferAmount,
      }];

      console.log(`Transferring from address ${i + 1}/5 to secondary...`);

      const result = await client.addressFundsTransfer({
        inputs,
        outputs,
        signer: inputSigner,
      });

      expect(result).to.exist;
      console.log(`Transfer ${i + 1} complete`);

      // Wait 3 seconds before next transaction (except after last one)
      if (i < initialAddresses.length - 1) {
        console.log('Waiting 3 seconds for next block...');
        await new Promise((resolve) => setTimeout(resolve, 3000));
      }
    }

    console.log('Step 3 complete: Funds moved to 5 addresses (one per block)');
  });

  it('Step 4: Wait 3 seconds', async function testWait2() {
    console.log('Waiting 3 seconds for platform to process...');
    await new Promise((resolve) => setTimeout(resolve, 3000));
    console.log('Wait complete');
  });

  it('Step 5: Withdraw from 5 addresses to core (one per block)', async function testWithdrawToCore() {
    // Withdrawal fee for 1 input: 400M base + 500k = 400.5M
    const withdrawalFee = 401000000n; // ~400.5M + buffer
    const minWithdrawAmount = 1000000n;

    // Create output script for core withdrawal
    const coreOutputHash = new Uint8Array(20).fill(0xcc);
    const outputScript = sdk.CoreScript.newP2PKH(coreOutputHash);

    // Withdraw from each secondary address separately
    for (let i = 0; i < secondaryAddresses.length; i++) {
      // Create signer with just this address's private key
      const withdrawSigner = new sdk.PlatformAddressSigner();
      const privateKey = sdk.PrivateKey.fromBytes(secondaryPrivateKeyBytes[i], 'regtest');
      withdrawSigner.addKey(privateKey);

      // Get current balance for this address
      const addressInfos = await client.getAddressesInfos([secondaryAddresses[i]]);
      let balance = 0n;
      for (const [, info] of addressInfos.entries()) {
        if (info && info.balance > 0n) {
          balance = info.balance;
          break;
        }
      }

      if (balance === 0n) {
        console.log(`Secondary address ${i} has no balance, skipping...`);
        continue;
      }

      // Check we have enough for fee + minimum withdrawal
      if (balance < withdrawalFee + minWithdrawAmount) {
        console.log(`Secondary address ${i} balance (${balance}) insufficient for withdrawal fee, skipping...`);
        continue;
      }

      const inputs = [{
        address: secondaryAddresses[i],
        amount: minWithdrawAmount,
      }];

      console.log(`Withdrawing from address ${i + 1}/5 (balance: ${balance})...`);

      const result = await client.addressFundsWithdraw({
        inputs,
        outputScript,
        coreFeePerByte: 1,
        pooling: 'never',
        signer: withdrawSigner,
      });

      expect(result).to.exist;

      for (const [, info] of result.entries()) {
        console.log(`Address ${i + 1} after withdrawal - balance: ${info.balance}`);
      }

      // Wait 3 seconds before next transaction (except after last one)
      if (i < secondaryAddresses.length - 1) {
        console.log('Waiting 3 seconds for next block...');
        await new Promise((resolve) => setTimeout(resolve, 3000));
      }
    }

    console.log('Step 5 complete: Funds withdrawn from addresses to core (one per block)');
  });
});
