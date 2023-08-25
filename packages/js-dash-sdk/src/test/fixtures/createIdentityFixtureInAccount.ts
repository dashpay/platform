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

  const publicKeyOne = new IdentityPublicKey(1);
  publicKeyOne.setData(identityMasterPrivateKey.toPublicKey().toBuffer());

  const publicKeyTwo = new IdentityPublicKey(1);
  publicKeyTwo.setData(identitySecondPrivateKey.toPublicKey().toBuffer());
  publicKeyTwo.setSecurityLevel(IdentityPublicKey.SECURITY_LEVELS.HIGH);

  identityFixture.setPublicKeys([publicKeyOne, publicKeyTwo]);

  account.storage
    .getWalletStore(account.walletId)
    .insertIdentityIdAtIndex(
      identityFixture.getId().toString(),
      identityFixtureIndex,
    );

  return identityFixture;
}
