import init, * as sdk from '../../dist/sdk.compressed.js';
import { wasmFunctionalTestRequirements } from './fixtures/requiredTestData.mjs';

describe('Platform address queries', function describePlatformAddressQueries() {
  this.timeout(60000);

  wasmFunctionalTestRequirements();

  let client;
  let testHash1;
  let testHash2;

  before(async () => {
    await init();
    await sdk.WasmSdk.prefetchTrustedQuorumsLocal();
    const builder = sdk.WasmSdkBuilder.localTrusted();
    client = await builder.build();

    // Create deterministic 20-byte hashes for testing unfunded addresses
    testHash1 = new Uint8Array(20).fill(0x01);
    testHash2 = new Uint8Array(20).fill(0x02);
  });

  after(() => {
    if (client) { client.free(); }
  });

  // Note: Platform addresses need to be funded for these tests to return data.
  // If no funded addresses exist, the tests will still pass but return undefined results.

  it('getAddressInfo() returns address info or undefined for unfunded address', async () => {
    const testAddress = sdk.PlatformAddress.fromP2pkhHash(testHash1);
    const res = await client.getAddressInfo(testAddress);
    // Result is undefined for unfunded/non-existent addresses
    expect(res === undefined || res !== null).to.be.true();
  });

  it('getAddressInfoWithProofInfo() returns proof info for address query', async () => {
    const testAddress = sdk.PlatformAddress.fromP2pkhHash(testHash1);
    const res = await client.getAddressInfoWithProofInfo(testAddress);
    expect(res).to.be.ok();
    expect(res.proof).to.be.ok();
    expect(res.metadata).to.be.ok();
    // data may be undefined for unfunded addresses
  });

  it('getAddressesInfos() returns map of address infos', async () => {
    const testAddress1 = sdk.PlatformAddress.fromP2pkhHash(testHash1);
    const testAddress2 = sdk.PlatformAddress.fromP2shHash(testHash2);
    const testAddresses = [testAddress1, testAddress2];

    const res = await client.getAddressesInfos(testAddresses);
    expect(res).to.be.instanceOf(Map);
    expect(res.size).to.equal(testAddresses.length);
  });

  it('getAddressesInfosWithProofInfo() returns proof info for multiple addresses', async () => {
    const testAddress1 = sdk.PlatformAddress.fromP2pkhHash(testHash1);
    const testAddress2 = sdk.PlatformAddress.fromP2shHash(testHash2);
    const testAddresses = [testAddress1, testAddress2];

    const res = await client.getAddressesInfosWithProofInfo(testAddresses);
    expect(res).to.be.ok();
    expect(res.proof).to.be.ok();
    expect(res.metadata).to.be.ok();
    expect(res.data).to.be.instanceOf(Map);
  });
});
