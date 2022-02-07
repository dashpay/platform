const Dashcore = require('@dashevo/dashcore-lib');
const { expect } = require('chai');
const DerivableKeyChain = require('./DerivableKeyChain');
const { mnemonicToHDPrivateKey } = require('../../utils/mnemonic');

let derivableKeyChain;
let derivableKeyChain2;
const mnemonic = 'during develop before curtain hazard rare job language become verb message travel';
const mnemonic2 = 'birth kingdom trash renew flavor utility donkey gasp regular alert pave layer';
const hdPublicKey = 'xpub661MyMwAqRbcFGB6XSWBsD725rJDUbFUpy4zWe2u22nJ2BxpoHFxtVDfKnTnvVQHohnY7AsVpRTHDv6PyPQTYu1KxFPKw29MAVXPEpz1G7V';
const expectedRootDIP15AccountKey_0 = 'tprv8hRzmheQujhJN5XP2dj955nAFCKeEoSifJRWuutdbwWRtusdDQ426jbp75EqErUSuTxmPyxYmP1TpcF5qdxGhXLNXRLMGsRLG6NFCv1WnaQ';
const expectedRootDIP15AccountKey_1 = 'tprv8hRzmheQujhJQyCtFTuUFHxB3Ag5VLB994zhH4CfxbA41cq73HT2mpYq5M33V54oJyn6g514saxxVJB886G55eYX56J6D6x87UNNT6iQHkR';
const expectedKeyForChild_0 = 'tprv8d4podc2Tg459CH2bwLHXj3vdJFBT2rdsk5Nr1djH7hzHdt5LRdvN6QyFwMiDy7ffRdik7fEVRKKgsHB4F18sh8xF6jFXpKq4sUgGBoSbKw';

describe('DerivableKeyChain', function suite() {
  this.timeout(1000);
  it('should create a DerivableKeyChain', () => {
    const expectedException1 = 'Expect one of [mnemonic, HDPrivateKey, HDPublicKey, privateKey, publicKey, address] to be provided.';
    expect(() => new DerivableKeyChain()).to.throw(expectedException1);

    derivableKeyChain = new DerivableKeyChain({ mnemonic: mnemonic, network: 'testnet' });
    expect(derivableKeyChain.rootKeyType).to.equal('HDPrivateKey');
    expect(derivableKeyChain.network.toString()).to.equal('testnet');
    expect(derivableKeyChain.rootKey.network.toString()).to.equal('testnet');

    derivableKeyChain2 = new DerivableKeyChain({ mnemonic: mnemonic2, network: 'livenet' });
  });

  it('should generate key for full path', () => {
    const path = 'm/44\'/1\'/0\'/0/0';
    const pk2 = derivableKeyChain.getForPath(path).key;
    const address = new Dashcore.Address(pk2.publicKey.toAddress()).toString();
    expect(address).to.equal('yNfUebksUc5HoSfg8gv98ruC3jUNJUM8pT');
  });

  it('should get DIP15 account key', function () {
    const rootDIP15AccountKey_0 = derivableKeyChain.getHardenedDIP15AccountKey(0);
    expect(rootDIP15AccountKey_0.toString()).to.deep.equal(expectedRootDIP15AccountKey_0);
    const rootDIP15AccountKey_1 = derivableKeyChain.getHardenedDIP15AccountKey(1);
    expect(rootDIP15AccountKey_1.toString()).to.deep.equal(expectedRootDIP15AccountKey_1);
  });

  it('should get DIP15 extended key', function () {
    const userUniqueId = '0x555d3854c910b7dee436869c4724bed2fe0784e198b8a39f02bbb49d8ebcfc3a';
    const contactUniqueId = '0xa137439f36d04a15474ff7423e4b904a14373fafb37a41db74c84f1dbb5c89b5';

    //  m/9'/5'/15'/0'/0x555d3854c910b7dee436869c4724bed2fe0784e198b8a39f02bbb49d8ebcfc3a'/0xa137439f36d04a15474ff7423e4b904a14373fafb37a41db74c84f1dbb5c89b5'/0
    const DIP15ExtPubKey_0 = derivableKeyChain2.getDIP15ExtendedKey(userUniqueId, contactUniqueId, 0, 0, 'HDPublicKey');
    expect(DIP15ExtPubKey_0.toString()).to.equal('xpub6LTkTQFSb8KMgMSz4B6sMZLpkQAY6wSTDprDkHDmLwWLpnjxazuxZn13FrSLKUafitsxuaaffM5a49P6aswhpppWUuYW6eFnwBXshR2W2eY');
    expect(DIP15ExtPubKey_0.publicKey.toString()).to.equal('038030c88ab0106e1f4af3b939db2bafc56f892554106f08da1ce1f9ef10f807bd')

    const DIP15ExtPrivKey_0 = derivableKeyChain2.getDIP15ExtendedKey(userUniqueId, contactUniqueId, 0, 0);
    expect(DIP15ExtPrivKey_0.toString()).to.equal('xprvA7UQ3tiYkkm4TsNWx9ZrzRQ6CNL3hUibrbvcwtp9nbyMwzQp3Tbi1ygZQaPoigDhCf8XUjMmGK2NbnB2kLXPYg99Lp6e3iki318sdWcFN3q');
    expect(DIP15ExtPrivKey_0.privateKey.toString()).to.equal('fac40790776d171ee1db90899b5eb2df2f7d2aaf35ad56f07ffb8ed2c57f8e60')
    expect(DIP15ExtPrivKey_0.publicKey.toString()).to.equal('038030c88ab0106e1f4af3b939db2bafc56f892554106f08da1ce1f9ef10f807bd')

    // This comes from the test factor of DIP-15
    const userAhash = "0xa11ce14f698b32e9bb306dba7bbbee831263dcf658abeebb39930460ead117e5";
    const userBhash = "0xb0b052ff075c5ca3c16c3e20e9ac8223834475cc1324ab07889cb24ce6a62793";
    const DIP15ExtKey_1 = derivableKeyChain.getDIP15ExtendedKey(userAhash, userBhash, 0, 0);
    expect(DIP15ExtKey_1.privateKey.toString()).to.equal('60581b6dca8244d3fb3cfe619b5a22277e5423b01e5285f356981f247e0f4a60')
    expect(DIP15ExtKey_1.publicKey.toString()).to.equal('03deaac00f721151307fbc7bf80d7b8afab98c1f026d67e5f56b21e2013f551ce6')
  });

  it('should derive from hardened path and get address', () => {
    const hardenedHDKey = derivableKeyChain.getHardenedBIP44HDKey();
    const pk2 = derivableKeyChain.getForPath(`m/44'/1'`).key;
    expect(pk2.toString()).to.equal(hardenedHDKey.toString());
    expect(hardenedHDKey.toString()).to.deep.equal('tprv8dtrJNytYHRiZY585hmHGbguS6VjGpK49puSB7oXZjLHcQfrAzQkF4ZCxM2DkEbyY85J4EYcZ8EjT5ZCU8ozB727TDdodbfXet5GkGau2RQ');
    const derivedPk = hardenedHDKey.deriveChild(0, true).deriveChild(0).deriveChild(0);
    // m/44'/1'/0'/0/0 (this is first external address of the account 0)
    const address = new Dashcore.Address(derivedPk.publicKey.toAddress()).toString();
    expect(address).to.equal('yNfUebksUc5HoSfg8gv98ruC3jUNJUM8pT');
  });

  it('should get hardened DIP9FeatureHDKey', function () {
    const hardenedHDKey = derivableKeyChain.getHardenedDIP9FeatureHDKey();
    const pk2 = derivableKeyChain.getForPath(`m/9'/1'`).key;
    expect(pk2.toString()).to.equal(hardenedHDKey.toString());
    expect(hardenedHDKey.toString()).to.deep.equal('tprv8fBJjWoGgCpGRCbyzE9RUA59rmoN1RUijhLnXGL4VHnLxvSe523yVg4GrGzbR6TyXtdynAEh5z8UX55EXt2Cb3xjvrsx2PgTY9BHxzFVkWn');
  });

  it('should get key for path using the HDPrivateKey', () => {
    const derivableKeyChain2 = new DerivableKeyChain({ HDPrivateKey: mnemonicToHDPrivateKey(mnemonic, 'testnet') });
    const keyForChild = derivableKeyChain2.getForPath('m/0').key;
    expect(keyForChild.toString()).to.equal(expectedKeyForChild_0);
  });

  it('should mark address watched and get watched addresses', function () {
    const key0 = derivableKeyChain.getForPath('m/0');
    derivableKeyChain.getForPath('m/0').isWatched = true
    key0.isWatched = true;
    derivableKeyChain.getForPath('m/1', { isWatched: false });
    derivableKeyChain.getForPath('m/2', { isWatched: true });

    const watchedAddresses = derivableKeyChain.getWatchedAddresses();
    let expectedWatchedAddresses = [
      derivableKeyChain.getForPath('m/0').address.toString(),
      derivableKeyChain.getForPath('m/2').address.toString()
    ];
    expect(watchedAddresses).to.deep.equal(expectedWatchedAddresses);
  });

  it('should get watched addresses', function () {
    derivableKeyChain.getForPath('m/1').isWatched = true
    const watchedAddresses = derivableKeyChain.getWatchedAddresses();
    const expectedWatchedAddresses = [
      'ybQDfNwiDjk8ZH5UUmHQzAMEmjbrbK5dAj',
      'yhFX5rseJPitV45HUCaa9haeGHtLuooBaq',
      'yhqxsmYk6jfoGWf1hJKq7d4U2cGHCgzpFU'
    ]
    expect(watchedAddresses).to.deep.equal(expectedWatchedAddresses);
  });

  it('should remove an address from watched addresses', function () {
    derivableKeyChain.getForPath('m/0', { isWatched: false });
    derivableKeyChain.getForPath('m/1');
    const data2 = derivableKeyChain.getForPath('m/2');
    data2.isWatched = false;

    expect(derivableKeyChain.getWatchedAddresses().length).to.equal(1);
  });

  it('should get address for path', function (){
    const address0_1 = derivableKeyChain.getForPath('m/1').address;
    expect(address0_1.toString()).to.equal('yhFX5rseJPitV45HUCaa9haeGHtLuooBaq')
  })

  it('should mark address as used', function () {
    const address0_0 = derivableKeyChain.getForPath('m/0').address;
    derivableKeyChain.markAddressAsUsed(address0_0);
    expect(derivableKeyChain.issuedPaths.get('m/0').isUsed).to.equal(true)
  });
});

describe('DerivableKeyChain - HDPublicKey', function suite(){
  let hdpubDerivableKeyChain;
  it('should initiate from a HDPublicKey', function () {
    hdpubDerivableKeyChain = new DerivableKeyChain({
      HDPublicKey: new Dashcore.HDPublicKey(hdPublicKey),
      network: 'testnet'
    });
    // As the HDPublicKey starts with xpub, it's livenet and should take priority over our network being set.
    expect(hdpubDerivableKeyChain.network.toString()).to.equal('livenet');
    expect(hdpubDerivableKeyChain.keyChainId).to.equal('kc5059442d66');
    expect(hdpubDerivableKeyChain.getRootKey().toString()).to.equal(hdPublicKey);
  });

  it('should derivate', function () {
    const key0_1 = hdpubDerivableKeyChain.getForPath('m/1').key;
    expect(key0_1.publicKey.toAddress(hdpubDerivableKeyChain.network).toString()).to.equal('XoL5LcBiDWcj6L7fFwytsFoX5Vz7BVXw9w')
  });

  it('should get address for path', function (){
    const address0_1 = hdpubDerivableKeyChain.getForPath('m/2').address;
    expect(address0_1.toString()).to.equal('XwAzpxQKbgebaLiadq1c6rDeFJ4FKPUufy')
  })
})

describe('DerivableKeyChain - single privateKey', function suite() {
  this.timeout(10000);

  it('should correctly throw errors out when not a HDPublicKey (privateKey)', () => {
    const privateKey = Dashcore.PrivateKey().toString();
    const network = 'livenet';
    const pkDerivableKeyChain = new DerivableKeyChain({ privateKey, network });
    expect(pkDerivableKeyChain.network).to.equal(network);
    expect(pkDerivableKeyChain.rootKeyType).to.equal('privateKey');
    expect(pkDerivableKeyChain.rootKey.toString()).to.equal(privateKey);

    const expectedException1 = 'Wallet is not loaded from a mnemonic or a HDPrivateKey, impossible to derivate keys for path m/0';
    expect(() => pkDerivableKeyChain.getForPath('m/0')).to.throw(expectedException1);
  });

  it('should get private key', () => {
    const privateKey = Dashcore.PrivateKey().toString();
    const pkDerivableKeyChain = new DerivableKeyChain({ privateKey, network: 'livenet' });
    expect(pkDerivableKeyChain.getRootKey().toString()).to.equal(privateKey);
    expect(pkDerivableKeyChain.rootKey.toString()).to.equal(privateKey);
  });
});
