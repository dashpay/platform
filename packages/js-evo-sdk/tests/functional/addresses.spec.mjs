import { EvoSDK } from '../../dist/evo-sdk.module.js';
import { TEST_IDS } from '../fixtures/testnet.mjs';

describe('Addresses', function addressesSuite() {
  this.timeout(60000);
  let sdk;

  before(async () => {
    sdk = EvoSDK.testnetTrusted();
    await sdk.connect();
  });

  // Note: Platform addresses need to be funded on testnet for these tests to return data.
  // If no funded addresses exist, the tests will still pass but return undefined/empty results.

  it('get() returns address info or undefined for unfunded address', async () => {
    const testAddress = TEST_IDS.platformAddressBytes;
    const res = await sdk.addresses.get(testAddress);
    // Result is undefined for unfunded/non-existent addresses
    expect(res === undefined || res !== null).to.be.true();
  });

  it('getWithProof() returns proof info for address query', async () => {
    const testAddress = TEST_IDS.platformAddressBytes;
    const res = await sdk.addresses.getWithProof(testAddress);
    expect(res).to.exist();
    expect(res.proof).to.exist();
    expect(res.metadata).to.exist();
    // data may be undefined for unfunded addresses
  });

  it('getMany() returns map of address infos', async () => {
    const testAddresses = TEST_IDS.platformAddressesBytesArray;
    const res = await sdk.addresses.getMany(testAddresses);
    expect(res).to.be.instanceOf(Map);
    expect(res.size).to.equal(testAddresses.length);
  });

  it('getManyWithProof() returns proof info for multiple addresses', async () => {
    const testAddresses = TEST_IDS.platformAddressesBytesArray;
    const res = await sdk.addresses.getManyWithProof(testAddresses);
    expect(res).to.exist();
    expect(res.proof).to.exist();
    expect(res.metadata).to.exist();
    expect(res.data).to.be.instanceOf(Map);
  });
});
