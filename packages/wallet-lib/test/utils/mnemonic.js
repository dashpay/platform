const {expect} = require('chai');
const {
  generateNewMnemonic,
  seedToHDPrivateKey,
  mnemonicToHDPrivateKey,
  mnemonicToWalletId,
  mnemonicToSeed,
} = require('../../src/utils/mnemonic');
const is = require('../../src/utils/is');

const mnemonic1 = 'hole lesson insane entire dolphin scissors game dwarf polar ethics drip math';
const mnemonic2 = 'woman forest output essay bleak satisfy era ordinary exotic source portion wire';
const mnemonic3 = 'divorce radar castle wire sun timber master income exchange wash fluid loud';
const mnemonic4 = 'increase table banana fiscal innocent wool sport mercy motion stable prize promote';

const mnemonic24En = 'ability trim just nerve eternal sting jar sponsor nose fix explain acid thought cake evidence kite clog stable surge actress cushion awake latin trim';
const expectedSeed24En = '8b991cf7bbbcda09c6542f4497f3e32b6b7e199d69f7bcd65bde001c28a3283e330c5a20f620ac6169e198b65c7a4b5ec46dbdee8f86e082421af9f6a81933e8';
const expectedEnRootKey = 'xprv9s21ZrQH143K2EfPZFVjrocWU5451xCPASWvczv9bs7TFdxVKSgb6d4yb6rQWB8Qey321JQN3hDsfUScfiwzpzZALKXcwNnTzf5opbsuqbT';

const mnemonic24Jp = 'あいさつ　むじゅん　せんさい　ちたい　ごうまん　ふひょう　せのび　ふうふ　ちゃんこなべ　ざいえき　こそだて　あそぶ　ますく　おおどおり　こうりつ　せんむ　がっこう　ふすま　へらす　あっしゅく　きまる　いらい　そうなん　むじゅん';
const expectedSeed24Jp = '61b05c77ca18cd7d60254310886c1615a1d5397d58b032ee34a8886385a8a1106bd2cf1b8678eb2671cd744359f1b8c8bca183edb431f3e08d3c774189af1a3b';
const expectedJpRootKey = 'xprv9s21ZrQH143K3WoLUCke5VK7Kkrjvp3K8vfFvbuKBXQMvBZRrPGJY3kCFAcXzw4DU8vxLe9kt2ShQjNB1pbes7oMzdafrrmppBtNuFYhRAc';

const mnemonic24Es = 'abdomen tirón lado mozo enorme sequía jugo sanidad músculo fiar esquí aceite tapa bolsa ensayo largo carne sección soltar acoger collar aprobar lento tirón';
const expectedSeed24Es = '59681653fde7db94e926ab6e8da8ab1ced4b2e04a7a6b5fbec1eb718dba91206fe8bde5bef9b5234c11ebc223a35772cef4c2114a3b7bed6b3749812d122e292';
const expectedEsRootKey = 'xprv9s21ZrQH143K3xmLC7RGEgYJAmPwZ699jsPVYkswkgKBztxvUusGGM45vDyS6z8runs8s31fqygFmcQ6wTbp7jybyWQNF7xwvr9NseKqiyx';

const mnemonic24Cn = '一 扑 苗 坦 财 辽 赵 筋 麻 析 投 国 涌 处 富 饭 信 棒 驶 用 称 间 裁 扑';
const expectedSeed24Cn = '4e7d64cfb9d5f28f1ba2ece2e16a2263d928c84ce8657d8612b018ca16d8d9a8218596f819b4c5677d426eb7104c376c49f21ba4795888cc4239eedd3b7e6fa0';
const expectedCnRootKey = 'xprv9s21ZrQH143K2CtwuutnGUo2SXtcTCZYcZ8Tg4KFm6C6WTMKGaEm3ga63dMsNarig7hfX8XzNfXdtBjBrs2Uaans5MmwdFKr5uDPqjcsTKb';

const mnemonic24CnTrad = '一 撲 苗 坦 財 遼 趙 筋 麻 析 投 國 湧 處 富 飯 信 棒 駛 用 稱 間 裁 撲';
const expectedSeed24CnTrad = 'b26ac05cbba14b5ba6d8a42d725605fb32be201119ec3428a292f200a250d5a890509c473b79ac1c463704d1ca4a8d9b5cd5e82697f7194cf814a53b300e5f0c';
const expectedCnTradRootKey = 'xprv9s21ZrQH143K2P4mYzdrX9ZCgM3xks4vTu2eGMrsd92CbTphBmDhwd4xPUneLq4WF66ETDiwHVzMS7T9BecoYCEw79JmxfGc3DfUrQLmpkL';

const mnemonic24Fr = 'abandon taupe herbe majorer écraser sanglier guimauve rouge maritime épine élitisme absurde soulever bonbon écrivain honneur cavalier ruisseau sénateur accepter compact aquarium hygiène taupe';
const expectedSeed24fr = 'd9c46cf130dfa19e56e87a46e04be9bdec52c81b3f1db24f3011197deba2cfd4e9f62de3663165577d1d870e1f78e602eb3d0f1659a33bd2646fd58fbbbb7d7a';
const expected24frRootKeyMain = 'xprv9s21ZrQH143K26ssNac4eHjBoeKdcZ1jZC8q1eBfrDcumyQwmnELDVFrb6Xd2E5GNtHYU1ayKv4EjfJoq5httWyaW1Fq6Zt5zvJ658m85FC';

const mnemonic24It = 'abbaglio topazio mantide osmosi famiglia soppeso magnete smeraldo ovocito fucilata fermento acre svista bretella fanfara maturo chirurgo sociale spronato adeguato custode arbitro mese topazio';
const expectedSeed24It = '33100dc60a1661f5bef03282c61b7423d561bad7bfeef34b90efbb5fd4508033ae0e1f364abb8078cdeff20a3cb24824bb0f00f9166841b8a99893f3c4639ad3';
const expectedItRootKey = 'xprv9s21ZrQH143K4btNDndrRbCDjxvmUBd17kwaapRTrivby5UkpzKapYq9v4oVMRhy7jq32gYCAKmNcviLgfZbNuC95zSKTB2UStYBNYRXTT7';

const mnemonic24Ko = '가끔 평양 시아버지 예상 버튼 초순 승리 책임 오른발 분석 별명 가정 태권도 기본 번역 시험 다이어트 철학 충격 간섭 마음 공개 신비 평양';
const expectedSeed24Ko = '611498a9b705459973ae188db9659aec7b433af197b4f37e5e7832b4d13367f50e4ac6e9597f0decc1ec1287d8ff8fbdb7b5b35771cc7b349b2abbf9a9b42e50';
const expectedKoRootKey = 'xprv9s21ZrQH143K2KzJuwYrhiBVik3Zi1Q1BjMJVvaCh3v7AVYpCDwpcUvhMEdeducZgGYoVT6vMfwNU63UMTiLAoxHk4KEAH8XbMrzAUnu8aP';

const mnemonic24Cz = 'abeceda vracet nezvykle peklo kolaps trhlina nehoda terapie pikle kultura koprovka archa varovat dominant kolize nosnost finance tlupa uhradit atlas honitba buvol obejmout vracet';
const expectedSeed24Cz = '2735f49ddab96518fce00fc27b192c12bf05f884f83d55ef4464c820485569eea63be4a5b60378f3c85082774d76910c8bfd4eebe9705b77c75d8c9844a6cea5';
const expectedCzRootKey = 'xprv9s21ZrQH143K4WmszXEM3kb6vPtvL67zWyzcZZiaqtijPhW5r1zmjvN4V66HKJYUEfoEDq199gHCE5AUUo4kMo3nReUT4EVofnakjaFLRQy';

const passSeed1 = 'superpassphrase';
const expectedSeed1 = '436905e6756c24551bffaebe97d0ebd51b2fa027e838c18d45767bd833b02a80a1dd55728635b54f2b1dbed5963f4155e160ee1e96e2d67f7e8ac28557d87d96';
const expectedPassSeed1 = 'f637c95a551647f3f49c707c2f40ea0ee38a70995ab108004529af55ea43bcf02c6bcb156f8750e6b4188ac1f0955505173336a1a1b579fe970071b0014be44c';
const expectedPrivate1Mainnet = 'xprv9s21ZrQH143K3hVMJ7XzM4uiV1PndeSqGVzowkGjRpnSesDkmb3p5iGp8scGgAPjLw8Z3WZZr2BcbN2kfzqSYRG3VKSQgSszEdijEoWSDAC';
const expectedPrivate1Testnet = 'tprv8ZgxMBicQKsPeWisxgPVWiXho8ozsAUqc3uvpAhBuoGvSTxqkxPZbTeG43mvgXn3iNfL3cBL1NmR4DaVoDBPMUXe1xeiLoc39jU9gRTVBd2';

describe('Utils - mnemonic', () => {
  it('should generate new mnemonic', () => {
    const result = generateNewMnemonic();
    expect(result.constructor.name).to.be.equal('Mnemonic');
  });
  it('should do mnemonicToHDPrivateKey', () => {
    const mnem1 = generateNewMnemonic();
    const mnem2 = generateNewMnemonic().toString();
    const result = mnemonicToHDPrivateKey(mnem1);
    const result2 = mnemonicToHDPrivateKey(mnem2);
    expect(result.constructor.name).to.be.equal('HDPrivateKey');
    expect(result2.constructor.name).to.be.equal('HDPrivateKey');
  });
  it('should do mnemonicToWalletId', () => {
    const mnem1 = generateNewMnemonic();
    const result = mnemonicToWalletId(mnem1);
    expect(result.constructor.name).to.be.equal('String');
    expect(result.length).to.be.equal(10);
    expect(is.hex(result)).to.be.equal(true);

    expect(mnemonicToWalletId(mnemonic1)).to.equal('f566600d81');
    expect(mnemonicToWalletId(mnemonic2)).to.equal('74bbe91a47');
    expect(mnemonicToWalletId(mnemonic3)).to.equal('f351a836e6');
    expect(mnemonicToWalletId(mnemonic4)).to.equal('fad183cbf7');

    expect(() => mnemonicToWalletId()).to.throw('Expect mnemonic to be provided');
    expect(() => mnemonicToHDPrivateKey()).to.throw('Expect mnemonic to be provided');
  });
  it('should do mnemonicToSeed', () => {
    expect(mnemonicToSeed(mnemonic1)).to.equal(expectedSeed1);
    expect(mnemonicToSeed(mnemonic1, passSeed1)).to.equal(expectedPassSeed1);
  });
  it('should do seedToHDPrivateKey', () => {
    expect(seedToHDPrivateKey(expectedSeed1).toString()).to.equal(expectedPrivate1Testnet);
    expect(seedToHDPrivateKey(expectedSeed1, 'mainnet').toString()).to.equal(expectedPrivate1Mainnet);
  });
  it('should work with 24 words', () => {
    expect(mnemonicToSeed(mnemonic24En).toString()).to.equal(expectedSeed24En);
    expect(seedToHDPrivateKey(expectedSeed24En, 'mainnet').toString()).to.equal(expectedEnRootKey);
  });
  it('should work with all languages', () => {
    expect(mnemonicToSeed(mnemonic24Es).toString()).to.equal(expectedSeed24Es);
    expect(seedToHDPrivateKey(expectedSeed24Es, 'mainnet').toString()).to.equal(expectedEsRootKey);

    expect(mnemonicToSeed(mnemonic24Jp).toString()).to.equal(expectedSeed24Jp);
    expect(seedToHDPrivateKey(expectedSeed24Jp, 'mainnet').toString()).to.equal(expectedJpRootKey);

    expect(mnemonicToSeed(mnemonic24Cn).toString()).to.equal(expectedSeed24Cn);
    expect(seedToHDPrivateKey(expectedSeed24Cn, 'mainnet').toString()).to.equal(expectedCnRootKey);

    expect(mnemonicToSeed(mnemonic24CnTrad).toString()).to.equal(expectedSeed24CnTrad);
    expect(seedToHDPrivateKey(expectedSeed24CnTrad, 'mainnet').toString()).to.equal(expectedCnTradRootKey);

    expect(mnemonicToSeed(mnemonic24Fr).toString()).to.equal(expectedSeed24fr);
    expect(seedToHDPrivateKey(expectedSeed24fr, 'mainnet').toString()).to.equal(expected24frRootKeyMain);

    expect(mnemonicToSeed(mnemonic24It).toString()).to.equal(expectedSeed24It);
    expect(seedToHDPrivateKey(expectedSeed24It, 'mainnet').toString()).to.equal(expectedItRootKey);

    expect(mnemonicToSeed(mnemonic24Ko).toString()).to.equal(expectedSeed24Ko);
    expect(seedToHDPrivateKey(expectedSeed24Ko, 'mainnet').toString()).to.equal(expectedKoRootKey);

    expect(mnemonicToSeed(mnemonic24Cz).toString()).to.equal(expectedSeed24Cz);
    expect(seedToHDPrivateKey(expectedSeed24Cz, 'mainnet').toString()).to.equal(expectedCzRootKey);
  });
});
