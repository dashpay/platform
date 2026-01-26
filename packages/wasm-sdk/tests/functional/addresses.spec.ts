import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';
import { wasmFunctionalTestRequirements } from './fixtures/requiredTestData.ts';

describe('Platform address queries', function describePlatformAddressQueries() {
  this.timeout(60000);

  wasmFunctionalTestRequirements();

  let client: sdk.WasmSdk;
  let testHash1: Uint8Array; // P2pkh hash for address with balance
  let testHash2: Uint8Array; // P2sh hash for address with balance

  before(async () => {
    await init();
    await sdk.WasmSdk.prefetchTrustedQuorumsLocal();
    const builder = sdk.WasmSdkBuilder.localTrusted();
    client = await builder.build();

    // Use the test addresses created by SDK_TEST_DATA=true during genesis
    // These correspond to:
    // - PLATFORM_ADDRESS_1 = P2pkh([10; 20]) with nonce=5, balance=1_000_000
    // - PLATFORM_ADDRESS_2 = P2sh([11; 20]) with nonce=7, balance=2_000_000
    testHash1 = new Uint8Array(20).fill(10);
    testHash2 = new Uint8Array(20).fill(11);
  });

  after(() => {
    if (client) { client.free(); }
  });

  it('should return address info for funded address via getAddressInfo()', async () => {
    const testAddress = sdk.PlatformAddress.fromP2pkhHash(testHash1);
    const res = await client.getAddressInfo(testAddress);
    // PLATFORM_ADDRESS_1 has nonce=5, balance=1_000_000
    expect(res).to.be.ok();
    expect(res.nonce).to.equal(5n);
    expect(res.balance).to.equal(1000000n);
  });

  it('should return proof info for address query via getAddressInfoWithProofInfo()', async () => {
    const testAddress = sdk.PlatformAddress.fromP2pkhHash(testHash1);
    const res = await client.getAddressInfoWithProofInfo(testAddress);
    expect(res).to.be.ok();
    expect(res.proof).to.be.ok();
    expect(res.metadata).to.be.ok();
    // data should be present for funded address
    expect(res.data).to.be.ok();
    expect(res.data.nonce).to.equal(5n);
    expect(res.data.balance).to.equal(1000000n);
  });

  it('should return map of address infos via getAddressesInfos()', async () => {
    const testAddress1 = sdk.PlatformAddress.fromP2pkhHash(testHash1);
    const testAddress2 = sdk.PlatformAddress.fromP2shHash(testHash2);
    const testAddresses = [testAddress1, testAddress2];

    const res = await client.getAddressesInfos(testAddresses);
    expect(res).to.be.instanceOf(Map);
    expect(res.size).to.equal(testAddresses.length);

    // Check values for both funded addresses
    // Note: Map keys are stringified addresses, so we need to check values
    const entries = Array.from(res.entries());
    expect(entries.length).to.equal(2);

    // Find entries by checking their nonce values (5n and 7n - BigInt)
    const info1 = entries.find(([, v]) => v && v.nonce === 5n);
    const info2 = entries.find(([, v]) => v && v.nonce === 7n);
    expect(info1).to.be.ok();
    expect(info1[1].balance).to.equal(1000000n);
    expect(info2).to.be.ok();
    expect(info2[1].balance).to.equal(2000000n);
  });

  it('should return proof info for multiple addresses via getAddressesInfosWithProofInfo()', async () => {
    const testAddress1 = sdk.PlatformAddress.fromP2pkhHash(testHash1);
    const testAddress2 = sdk.PlatformAddress.fromP2shHash(testHash2);
    const testAddresses = [testAddress1, testAddress2];

    const res = await client.getAddressesInfosWithProofInfo(testAddresses);
    expect(res).to.be.ok();
    expect(res.proof).to.be.ok();
    expect(res.metadata).to.be.ok();
    expect(res.data).to.be.instanceOf(Map);
    expect(res.data.size).to.equal(2);

    // Check data values (nonce may be BigInt or number depending on serialization)
    const entries = Array.from(res.data.entries());
    // Use loose equality (==) to handle both BigInt and number
    // eslint-disable-next-line eqeqeq
    const info1 = entries.find(([, v]) => v && v.nonce == 5);
    // eslint-disable-next-line eqeqeq
    const info2 = entries.find(([, v]) => v && v.nonce == 7);
    expect(info1).to.be.ok();
    // eslint-disable-next-line eqeqeq
    expect(info1[1].balance == 1000000).to.be.true();
    expect(info2).to.be.ok();
    // eslint-disable-next-line eqeqeq
    expect(info2[1].balance == 2000000).to.be.true();
  });
});
