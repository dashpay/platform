import loadWasmDpp, { IdentityPublicKey } from '@dashevo/wasm-dpp';

const getIdentityFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getIdentityFixture');

export async function createIdentityFixtureInAccount(account) {
  await loadWasmDpp();

  const identityFixture = await getIdentityFixture();
  const identityFixtureIndex = 0;
  const { privateKey: identityMasterPrivateKey } = account
    .identities.getIdentityHDKeyByIndex(identityFixtureIndex, 0);
  const { privateKey: identitySecondPrivateKey } = account
    .identities.getIdentityHDKeyByIndex(identityFixtureIndex, 1);

  const publicKeyOne = new IdentityPublicKey({
    $version: '0',
    id: 0,
    type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
    data: identityMasterPrivateKey.toPublicKey().toBuffer(),
    purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
    securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
    readOnly: false,
  });

  const publicKeyOneTwo = new IdentityPublicKey({
    $version: '0',
    id: 1,
    type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
    data: identitySecondPrivateKey.toPublicKey().toBuffer(),
    purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
    securityLevel: IdentityPublicKey.SECURITY_LEVELS.HIGH,
    readOnly: false,
  });

  identityFixture.setPublicKeys([publicKeyOne, publicKeyOneTwo]);

  account.storage
    .getWalletStore(account.walletId)
    .insertIdentityIdAtIndex(
      identityFixture.getId().toString(),
      identityFixtureIndex,
    );

  return identityFixture;
}
