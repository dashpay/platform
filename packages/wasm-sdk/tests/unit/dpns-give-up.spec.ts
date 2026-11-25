/**
 * `dpnsDeleteName` and `dpnsTombstoneName` read the name from the network and broadcast, so
 * the unit suite checks only what they do before any read: the options are taken apart first,
 * and a name without a label is refused. `dpnsIsNameTombstoned` answers for such a name
 * before reading anything.
 */
import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';

const AUTHENTICATION = 0;
const HIGH = 2;
const ECDSA_SECP256K1 = 0;

const EMPTY_NAME = 'the DPNS name is empty after normalization';

describe('giving up a DPNS name', () => {
  let client: sdk.WasmSdk;
  let identity: sdk.Identity;
  let identityKey: sdk.IdentityPublicKey;
  let signer: sdk.IdentitySigner;

  before(async () => {
    await init();
    client = await sdk.WasmSdkBuilder.testnet().build();

    const privateKey = sdk.PrivateKey.fromHex('21'.repeat(32), 'testnet');
    identityKey = new sdk.IdentityPublicKey({
      keyId: 1,
      purpose: AUTHENTICATION,
      securityLevel: HIGH,
      keyType: ECDSA_SECP256K1,
      isReadOnly: false,
      data: privateKey.getPublicKey().toBytes(),
    });
    signer = new sdk.IdentitySigner();
    signer.addKey(privateKey);
    identity = new sdk.Identity(new Uint8Array(32).fill(7));
  });

  after(() => {
    client?.free();
  });

  describe('dpnsIsNameTombstoned()', () => {
    it('should report a name without a label not tombstoned before reading anything', async () => {
      expect(await client.dpnsIsNameTombstoned('')).to.equal(false);
      expect(await client.dpnsIsNameTombstoned('.dash')).to.equal(false);
    });
  });

  describe('dpnsDeleteName()', () => {
    it('should refuse options without a name before reading anything', async () => {
      await expect(client.dpnsDeleteName({} as never)).to.be.rejectedWith("'name' is required");
    });

    it('should refuse options without an identity key before reading anything', async () => {
      await expect(client.dpnsDeleteName({ name: 'alice.dash' } as never))
        .to.be.rejectedWith("'identityKey' is required");
    });

    it('should refuse a name without a label before reading anything', async () => {
      await expect(client.dpnsDeleteName({ name: '.dash', identityKey, signer }))
        .to.be.rejectedWith(EMPTY_NAME);
    });
  });

  describe('dpnsTombstoneName()', () => {
    it('should refuse options without a name before reading anything', async () => {
      await expect(client.dpnsTombstoneName({} as never)).to.be.rejectedWith("'name' is required");
    });

    it('should refuse options without an identity before reading anything', async () => {
      await expect(client.dpnsTombstoneName({ name: 'alice.dash' } as never))
        .to.be.rejectedWith("'identity' is required");
    });

    it('should refuse a name without a label before reading anything', async () => {
      await expect(client.dpnsTombstoneName({
        name: '.dash', identity, identityKey, signer,
      })).to.be.rejectedWith(EMPTY_NAME);
    });
  });
});
