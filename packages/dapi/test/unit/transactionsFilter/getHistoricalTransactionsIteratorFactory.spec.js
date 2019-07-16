const chai = require('chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const { MerkleBlock, Transaction, BloomFilter } = require('@dashevo/dashcore-lib');

const getHistoricalTransactionsIteratorFactory = require('../../../lib/transactionsFilter/getHistoricalTransactionsIteratorFactory');

const { expect } = chai;
chai.use(dirtyChai);
chai.use(chaiAsPromised);

describe('getHistoricalTransactionsIteratorFactory', () => {
  let mockData;
  let rawMerkleBlock;
  let coreRpcMock;
  let bloomFilter;

  beforeEach(() => {
    rawMerkleBlock = '03000000' // Version
      + '35ce79ae46a65f0d0115d831584d0a6882117f75a65386f8f14e150000000000' // prevHash
      + 'a0055d45ad9b35e77fb01c59a4feb9976921493d2557a5ac0798b49e82ea1e99' // MerkleRoot
      + '6a04a055' // Time
      + 'c380181b' // Bits
      + '00270c9b' // Nonce
      + '0c000000' // Transaction Count
      + '08' // Hash Count
      + '9d0a368bc9923c6cb966135a4ceda30cc5f259f72c8843ce015056375f8a06ec' // Hash1
      + '39e5cd533567ac0a8602bcc4c29e2f01a4abb0fe68ffbc7be6c393db188b72e0' // Hash2
      + 'cd75b421157eca03eff664bdc165730f91ef2fa52df19ff415ab5acb30045425' // Hash3
      + '2ef9795147caaeecee5bc2520704bb372cde06dbd2e871750f31336fd3f02be3' // Hash4
      + '2241d3448560f8b1d3a07ea5c31e79eb595632984a20f50944809a61fdd9fe0b' // Hash5
      + '45afbfe270014d5593cb065562f1fed726f767fe334d8b3f4379025cfa5be8c5' // Hash6
      + '198c03da0ccf871db91fe436e2795908eac5cc7d164232182e9445f7f9db1ab2' // Hash7
      + 'ed07c181ce5ba7cb66d205bc970f43e1ca11996d611aa8e91e305eb8608c543c' // Hash8
      + '02' // Num Flag Bytes
      + 'db3f';
    mockData = {
      blocks: [{
        hash: '45afbfe270014d5593cb065562f1fed726f767fe334d8b3f4379025cfa5be8c5',
        height: 2002,
      },
      {
        hash: '000000000000000000000000000000000000000000000000000000000000001c',
        height: 2582,
      },
      {
        hash: 'ed07c181ce5ba7cb66d205bc970f43e1ca11996d611aa8e91e305eb8608c543c',
        height: 4002,
      },
      {
        hash: '9d0a368bc9923c6cb966135a4ceda30cc5f259f72c8843ce015056375f8a06ec',
        height: 6002,
      },
      {
        hash: '198c03da0ccf871db91fe436e2795908eac5cc7d164232182e9445f7f9db1ab2',
        height: 6125,
      },
        // {
        //   hash: 'ed07c181ce5ba7cb66d205bc970f43e1ca11996d611aa8e91e305eb8608c543c',
        //   height: 8125,
        // }
      ],
      transactions: [],
    };
    coreRpcMock = {
      getMerkleBlocks: sinon.stub(),
      getRawTransaction: sinon.stub(),
      getBlock: sinon.stub(),
      getBlockHash: sinon.stub(),
    };
    coreRpcMock.getMerkleBlocks
      .withArgs()
      .resolves([rawMerkleBlock]);

    coreRpcMock.getRawTransaction
      .withArgs('cd75b421157eca03eff664bdc165730f91ef2fa52df19ff415ab5acb30045425')
      .resolves('020000000a1b8972f91733804588910d466c0e77cfa65afad2a025a106a82ec6e32d617760050000006b483045022100a576b4e5bf5db550c95ae3d4c56773cdf3e7c632b0c52b6daf70fbcc37a405b502201e091da13954d861040ce382b51c50384ed12095308e0b5d92892361f7e94379812103d57433a7d82e48cd246ef29b9098b24dcf7eebc565586ab26e2bafe5b4d25750ffffffff0df9aeac7e6feeac4a935df9a0220dc498a0fc95a798bcb63a5ab58e5e8e5b6e0d0000006b4830450221008409d69f155c250eb0b4e6a0cf6dca3eb4910c3fd00c1ff245b4c9cc5f319a6b02206985a6d417e707d2d64a19225882c090e4527ec216e219f5ebea8e24bb7278038121036ac21c340c9041eb634f05ab5b6523514bc24784f132b945eef9fd148d616a2dffffffff00b4819721a0e85781c32d65d0f19b3884d7b8d3fe4f0ea3f9c1ea99087e297d010000006a47304402203cc336d6e6dd9a4986eea95dc9b5c7059949503ab227adc7425a6815ab4b9a0102202325b9d275bd4d7ed533d7974f73efae7ad27227f785ec95741e246444e82f978121033c5d60a3ecb98a3cb2214da030e8c28ecaae4663a7f389988c03b123ac0520cbffffffff85d4b234e8da90562da99f35e32cfc0ac93b8d8f041a6701c5e2e82619a9288b0e0000006b483045022100b0c50f44d12831d0f81ce28f498fbd51870b358dacafdc1764f09b021d2c72dd0220511845b6202274950a6a55c4023745f40991463e82a96851f1caec00b5a8a08c812102274dcc3b3e116ad930197f445554c0f3a7514e754c2672555fba335ec69cc5a3ffffffff8c46e46c9a44c4e3998caae7ff9fed881d4e91093fceb61fc33ddc0d5a1b919f040000006a47304402204dcb8938cdf74e80a87c69fa0da61988a12d73ed3495360d5a2d15a441f5e441022033f6b881ad83b10c6affb86a7d562ad421308693e457826e47fa25e57e51f249812103353ec1efb7ebe7aac6fa8431370613bc96d1b1b6cfef8706d7c3e10191fb947fffffffffc9a1d3293ec8604027ee6e1deec58924efacc896009c8b85003b6269553cf5b1010000006a473044022038af9647a90b082d49fa8471dd14a1955b5f1a8984d50dfe08b960b5c8206d440220076d9cf76adf5f86bc8dac5e84c6c14dc6f4a8605ad4832bd305fdc55b8f871b81210294a545f4bf055ee5e4b52bc98b40273ebe7ce172f70d75e48cafcbaaa43d0168ffffffff9d964428902aeb56acfc268b27ab828dc356597ff96c7b789062a1b90cbdf8be020000006b483045022100dce2c3c2bf6ed6b3d2d6eb55d530881b37fca13d08812accd9ed79c76eed75ed02206b0806366fabe8f70c9b1e08f7359861aea480a9a2dd2e3b15145ef06d46d7ed812102faaf13ed5f32f439bfbc5a9d6b5fa392eaace8c596d6b608fd48e1e40b697c90fffffffffc17a363992de5501cb6969b9e835fb333494382990ab319b0343c9d38218abf030000006a473044022012c80bec0b961cf7f8286ac525492a27dc09de299b59e6c294088eb04bb4a1d0022001cc1a8e1bf796f8a4a1e9fd1098e3dbc6bf949be7b4b8424388e3c83792188e812103905c6ad7c04f4594d135d26d980a9c2e16b50946886731b51ed61a9bdd49900bffffffff1892cd6c3038df794331ce437f8060cc2398a3744ec0f7488d4816e43b714ad6050000006a473044022044f3946ba17f87bffcf3fd4c5cfc43c530b97d4a97e3a258372f2074029cf4e802204d22e1afac51ef05c706afc43693b75542f2a82ef89b5026189135e0b1b54f44812102a37654d73b59ab13140a5adfdb79698fe23f6f05c5bb881704b3de2435161e1cffffffff66d6108688b82371210a987b1e22e9aa69a17c98803cc2f4e20e9957552053ea050000006b483045022100d4c315756462c0be66c786617fa8054fb266b5f6c7214cbc7cb00a41869bd26c022001288863647ece1a75718abd9bce8519b99a3e4509ef5554d8a92a633da525588121025ef06ce1ef91f2e535188d8c9ce1863f9ae6d37df8f143e6f558395f849fccb5ffffffff0ae4969800000000001976a91435b4e7ccf37e7933e2b9494f840b792b234e227388ace4969800000000001976a9144accca5ad3fa16538449b20ab51734dbdc9ef22288ace4969800000000001976a9145ac4de0775a027d0f0ec54e4c3d55fe57229fc4e88ace4969800000000001976a9147d224284b61bb5c3c0a8f32324052efd4781e28b88ace4969800000000001976a914900be7732a34e46d00dbaace243f65486448f8e388ace4969800000000001976a914a16dece64baadfec54d6ca0bf77f7380a66150c388ace4969800000000001976a914cc0822f608a13008b559d9536e69251f71e1096e88ace4969800000000001976a914f43dc02e4228e32df7c9d04fe3a59f69be05da6788ace4969800000000001976a914fe70994bce368c86578ae95845e1be7a908df06388ace4969800000000001976a914ff2f09b4f1e2f6e48b1e2a470ca1db3fe20baf6988ac00000000')
      .withArgs('2ef9795147caaeecee5bc2520704bb372cde06dbd2e871750f31336fd3f02be3')
      .resolves('020000000a1b8972f91733804588910d466c0e77cfa65afad2a025a106a82ec6e32d617760050000006b483045022100a576b4e5bf5db550c95ae3d4c56773cdf3e7c632b0c52b6daf70fbcc37a405b502201e091da13954d861040ce382b51c50384ed12095308e0b5d92892361f7e94379812103d57433a7d82e48cd246ef29b9098b24dcf7eebc565586ab26e2bafe5b4d25750ffffffff0df9aeac7e6feeac4a935df9a0220dc498a0fc95a798bcb63a5ab58e5e8e5b6e0d0000006b4830450221008409d69f155c250eb0b4e6a0cf6dca3eb4910c3fd00c1ff245b4c9cc5f319a6b02206985a6d417e707d2d64a19225882c090e4527ec216e219f5ebea8e24bb7278038121036ac21c340c9041eb634f05ab5b6523514bc24784f132b945eef9fd148d616a2dffffffff00b4819721a0e85781c32d65d0f19b3884d7b8d3fe4f0ea3f9c1ea99087e297d010000006a47304402203cc336d6e6dd9a4986eea95dc9b5c7059949503ab227adc7425a6815ab4b9a0102202325b9d275bd4d7ed533d7974f73efae7ad27227f785ec95741e246444e82f978121033c5d60a3ecb98a3cb2214da030e8c28ecaae4663a7f389988c03b123ac0520cbffffffff85d4b234e8da90562da99f35e32cfc0ac93b8d8f041a6701c5e2e82619a9288b0e0000006b483045022100b0c50f44d12831d0f81ce28f498fbd51870b358dacafdc1764f09b021d2c72dd0220511845b6202274950a6a55c4023745f40991463e82a96851f1caec00b5a8a08c812102274dcc3b3e116ad930197f445554c0f3a7514e754c2672555fba335ec69cc5a3ffffffff8c46e46c9a44c4e3998caae7ff9fed881d4e91093fceb61fc33ddc0d5a1b919f040000006a47304402204dcb8938cdf74e80a87c69fa0da61988a12d73ed3495360d5a2d15a441f5e441022033f6b881ad83b10c6affb86a7d562ad421308693e457826e47fa25e57e51f249812103353ec1efb7ebe7aac6fa8431370613bc96d1b1b6cfef8706d7c3e10191fb947fffffffffc9a1d3293ec8604027ee6e1deec58924efacc896009c8b85003b6269553cf5b1010000006a473044022038af9647a90b082d49fa8471dd14a1955b5f1a8984d50dfe08b960b5c8206d440220076d9cf76adf5f86bc8dac5e84c6c14dc6f4a8605ad4832bd305fdc55b8f871b81210294a545f4bf055ee5e4b52bc98b40273ebe7ce172f70d75e48cafcbaaa43d0168ffffffff9d964428902aeb56acfc268b27ab828dc356597ff96c7b789062a1b90cbdf8be020000006b483045022100dce2c3c2bf6ed6b3d2d6eb55d530881b37fca13d08812accd9ed79c76eed75ed02206b0806366fabe8f70c9b1e08f7359861aea480a9a2dd2e3b15145ef06d46d7ed812102faaf13ed5f32f439bfbc5a9d6b5fa392eaace8c596d6b608fd48e1e40b697c90fffffffffc17a363992de5501cb6969b9e835fb333494382990ab319b0343c9d38218abf030000006a473044022012c80bec0b961cf7f8286ac525492a27dc09de299b59e6c294088eb04bb4a1d0022001cc1a8e1bf796f8a4a1e9fd1098e3dbc6bf949be7b4b8424388e3c83792188e812103905c6ad7c04f4594d135d26d980a9c2e16b50946886731b51ed61a9bdd49900bffffffff1892cd6c3038df794331ce437f8060cc2398a3744ec0f7488d4816e43b714ad6050000006a473044022044f3946ba17f87bffcf3fd4c5cfc43c530b97d4a97e3a258372f2074029cf4e802204d22e1afac51ef05c706afc43693b75542f2a82ef89b5026189135e0b1b54f44812102a37654d73b59ab13140a5adfdb79698fe23f6f05c5bb881704b3de2435161e1cffffffff66d6108688b82371210a987b1e22e9aa69a17c98803cc2f4e20e9957552053ea050000006b483045022100d4c315756462c0be66c786617fa8054fb266b5f6c7214cbc7cb00a41869bd26c022001288863647ece1a75718abd9bce8519b99a3e4509ef5554d8a92a633da525588121025ef06ce1ef91f2e535188d8c9ce1863f9ae6d37df8f143e6f558395f849fccb5ffffffff0ae4969800000000001976a91435b4e7ccf37e7933e2b9494f840b792b234e227388ace4969800000000001976a9144accca5ad3fa16538449b20ab51734dbdc9ef22288ace4969800000000001976a9145ac4de0775a027d0f0ec54e4c3d55fe57229fc4e88ace4969800000000001976a9147d224284b61bb5c3c0a8f32324052efd4781e28b88ace4969800000000001976a914900be7732a34e46d00dbaace243f65486448f8e388ace4969800000000001976a914a16dece64baadfec54d6ca0bf77f7380a66150c388ace4969800000000001976a914cc0822f608a13008b559d9536e69251f71e1096e88ace4969800000000001976a914f43dc02e4228e32df7c9d04fe3a59f69be05da6788ace4969800000000001976a914fe70994bce368c86578ae95845e1be7a908df06388ace4969800000000001976a914ff2f09b4f1e2f6e48b1e2a470ca1db3fe20baf6988ac00000000')
      .withArgs('2241d3448560f8b1d3a07ea5c31e79eb595632984a20f50944809a61fdd9fe0b')
      .resolves('020000000a1b8972f91733804588910d466c0e77cfa65afad2a025a106a82ec6e32d617760050000006b483045022100a576b4e5bf5db550c95ae3d4c56773cdf3e7c632b0c52b6daf70fbcc37a405b502201e091da13954d861040ce382b51c50384ed12095308e0b5d92892361f7e94379812103d57433a7d82e48cd246ef29b9098b24dcf7eebc565586ab26e2bafe5b4d25750ffffffff0df9aeac7e6feeac4a935df9a0220dc498a0fc95a798bcb63a5ab58e5e8e5b6e0d0000006b4830450221008409d69f155c250eb0b4e6a0cf6dca3eb4910c3fd00c1ff245b4c9cc5f319a6b02206985a6d417e707d2d64a19225882c090e4527ec216e219f5ebea8e24bb7278038121036ac21c340c9041eb634f05ab5b6523514bc24784f132b945eef9fd148d616a2dffffffff00b4819721a0e85781c32d65d0f19b3884d7b8d3fe4f0ea3f9c1ea99087e297d010000006a47304402203cc336d6e6dd9a4986eea95dc9b5c7059949503ab227adc7425a6815ab4b9a0102202325b9d275bd4d7ed533d7974f73efae7ad27227f785ec95741e246444e82f978121033c5d60a3ecb98a3cb2214da030e8c28ecaae4663a7f389988c03b123ac0520cbffffffff85d4b234e8da90562da99f35e32cfc0ac93b8d8f041a6701c5e2e82619a9288b0e0000006b483045022100b0c50f44d12831d0f81ce28f498fbd51870b358dacafdc1764f09b021d2c72dd0220511845b6202274950a6a55c4023745f40991463e82a96851f1caec00b5a8a08c812102274dcc3b3e116ad930197f445554c0f3a7514e754c2672555fba335ec69cc5a3ffffffff8c46e46c9a44c4e3998caae7ff9fed881d4e91093fceb61fc33ddc0d5a1b919f040000006a47304402204dcb8938cdf74e80a87c69fa0da61988a12d73ed3495360d5a2d15a441f5e441022033f6b881ad83b10c6affb86a7d562ad421308693e457826e47fa25e57e51f249812103353ec1efb7ebe7aac6fa8431370613bc96d1b1b6cfef8706d7c3e10191fb947fffffffffc9a1d3293ec8604027ee6e1deec58924efacc896009c8b85003b6269553cf5b1010000006a473044022038af9647a90b082d49fa8471dd14a1955b5f1a8984d50dfe08b960b5c8206d440220076d9cf76adf5f86bc8dac5e84c6c14dc6f4a8605ad4832bd305fdc55b8f871b81210294a545f4bf055ee5e4b52bc98b40273ebe7ce172f70d75e48cafcbaaa43d0168ffffffff9d964428902aeb56acfc268b27ab828dc356597ff96c7b789062a1b90cbdf8be020000006b483045022100dce2c3c2bf6ed6b3d2d6eb55d530881b37fca13d08812accd9ed79c76eed75ed02206b0806366fabe8f70c9b1e08f7359861aea480a9a2dd2e3b15145ef06d46d7ed812102faaf13ed5f32f439bfbc5a9d6b5fa392eaace8c596d6b608fd48e1e40b697c90fffffffffc17a363992de5501cb6969b9e835fb333494382990ab319b0343c9d38218abf030000006a473044022012c80bec0b961cf7f8286ac525492a27dc09de299b59e6c294088eb04bb4a1d0022001cc1a8e1bf796f8a4a1e9fd1098e3dbc6bf949be7b4b8424388e3c83792188e812103905c6ad7c04f4594d135d26d980a9c2e16b50946886731b51ed61a9bdd49900bffffffff1892cd6c3038df794331ce437f8060cc2398a3744ec0f7488d4816e43b714ad6050000006a473044022044f3946ba17f87bffcf3fd4c5cfc43c530b97d4a97e3a258372f2074029cf4e802204d22e1afac51ef05c706afc43693b75542f2a82ef89b5026189135e0b1b54f44812102a37654d73b59ab13140a5adfdb79698fe23f6f05c5bb881704b3de2435161e1cffffffff66d6108688b82371210a987b1e22e9aa69a17c98803cc2f4e20e9957552053ea050000006b483045022100d4c315756462c0be66c786617fa8054fb266b5f6c7214cbc7cb00a41869bd26c022001288863647ece1a75718abd9bce8519b99a3e4509ef5554d8a92a633da525588121025ef06ce1ef91f2e535188d8c9ce1863f9ae6d37df8f143e6f558395f849fccb5ffffffff0ae4969800000000001976a91435b4e7ccf37e7933e2b9494f840b792b234e227388ace4969800000000001976a9144accca5ad3fa16538449b20ab51734dbdc9ef22288ace4969800000000001976a9145ac4de0775a027d0f0ec54e4c3d55fe57229fc4e88ace4969800000000001976a9147d224284b61bb5c3c0a8f32324052efd4781e28b88ace4969800000000001976a914900be7732a34e46d00dbaace243f65486448f8e388ace4969800000000001976a914a16dece64baadfec54d6ca0bf77f7380a66150c388ace4969800000000001976a914cc0822f608a13008b559d9536e69251f71e1096e88ace4969800000000001976a914f43dc02e4228e32df7c9d04fe3a59f69be05da6788ace4969800000000001976a914fe70994bce368c86578ae95845e1be7a908df06388ace4969800000000001976a914ff2f09b4f1e2f6e48b1e2a470ca1db3fe20baf6988ac00000000')
      .withArgs('45afbfe270014d5593cb065562f1fed726f767fe334d8b3f4379025cfa5be8c5')
      .resolves('020000000a1b8972f91733804588910d466c0e77cfa65afad2a025a106a82ec6e32d617760050000006b483045022100a576b4e5bf5db550c95ae3d4c56773cdf3e7c632b0c52b6daf70fbcc37a405b502201e091da13954d861040ce382b51c50384ed12095308e0b5d92892361f7e94379812103d57433a7d82e48cd246ef29b9098b24dcf7eebc565586ab26e2bafe5b4d25750ffffffff0df9aeac7e6feeac4a935df9a0220dc498a0fc95a798bcb63a5ab58e5e8e5b6e0d0000006b4830450221008409d69f155c250eb0b4e6a0cf6dca3eb4910c3fd00c1ff245b4c9cc5f319a6b02206985a6d417e707d2d64a19225882c090e4527ec216e219f5ebea8e24bb7278038121036ac21c340c9041eb634f05ab5b6523514bc24784f132b945eef9fd148d616a2dffffffff00b4819721a0e85781c32d65d0f19b3884d7b8d3fe4f0ea3f9c1ea99087e297d010000006a47304402203cc336d6e6dd9a4986eea95dc9b5c7059949503ab227adc7425a6815ab4b9a0102202325b9d275bd4d7ed533d7974f73efae7ad27227f785ec95741e246444e82f978121033c5d60a3ecb98a3cb2214da030e8c28ecaae4663a7f389988c03b123ac0520cbffffffff85d4b234e8da90562da99f35e32cfc0ac93b8d8f041a6701c5e2e82619a9288b0e0000006b483045022100b0c50f44d12831d0f81ce28f498fbd51870b358dacafdc1764f09b021d2c72dd0220511845b6202274950a6a55c4023745f40991463e82a96851f1caec00b5a8a08c812102274dcc3b3e116ad930197f445554c0f3a7514e754c2672555fba335ec69cc5a3ffffffff8c46e46c9a44c4e3998caae7ff9fed881d4e91093fceb61fc33ddc0d5a1b919f040000006a47304402204dcb8938cdf74e80a87c69fa0da61988a12d73ed3495360d5a2d15a441f5e441022033f6b881ad83b10c6affb86a7d562ad421308693e457826e47fa25e57e51f249812103353ec1efb7ebe7aac6fa8431370613bc96d1b1b6cfef8706d7c3e10191fb947fffffffffc9a1d3293ec8604027ee6e1deec58924efacc896009c8b85003b6269553cf5b1010000006a473044022038af9647a90b082d49fa8471dd14a1955b5f1a8984d50dfe08b960b5c8206d440220076d9cf76adf5f86bc8dac5e84c6c14dc6f4a8605ad4832bd305fdc55b8f871b81210294a545f4bf055ee5e4b52bc98b40273ebe7ce172f70d75e48cafcbaaa43d0168ffffffff9d964428902aeb56acfc268b27ab828dc356597ff96c7b789062a1b90cbdf8be020000006b483045022100dce2c3c2bf6ed6b3d2d6eb55d530881b37fca13d08812accd9ed79c76eed75ed02206b0806366fabe8f70c9b1e08f7359861aea480a9a2dd2e3b15145ef06d46d7ed812102faaf13ed5f32f439bfbc5a9d6b5fa392eaace8c596d6b608fd48e1e40b697c90fffffffffc17a363992de5501cb6969b9e835fb333494382990ab319b0343c9d38218abf030000006a473044022012c80bec0b961cf7f8286ac525492a27dc09de299b59e6c294088eb04bb4a1d0022001cc1a8e1bf796f8a4a1e9fd1098e3dbc6bf949be7b4b8424388e3c83792188e812103905c6ad7c04f4594d135d26d980a9c2e16b50946886731b51ed61a9bdd49900bffffffff1892cd6c3038df794331ce437f8060cc2398a3744ec0f7488d4816e43b714ad6050000006a473044022044f3946ba17f87bffcf3fd4c5cfc43c530b97d4a97e3a258372f2074029cf4e802204d22e1afac51ef05c706afc43693b75542f2a82ef89b5026189135e0b1b54f44812102a37654d73b59ab13140a5adfdb79698fe23f6f05c5bb881704b3de2435161e1cffffffff66d6108688b82371210a987b1e22e9aa69a17c98803cc2f4e20e9957552053ea050000006b483045022100d4c315756462c0be66c786617fa8054fb266b5f6c7214cbc7cb00a41869bd26c022001288863647ece1a75718abd9bce8519b99a3e4509ef5554d8a92a633da525588121025ef06ce1ef91f2e535188d8c9ce1863f9ae6d37df8f143e6f558395f849fccb5ffffffff0ae4969800000000001976a91435b4e7ccf37e7933e2b9494f840b792b234e227388ace4969800000000001976a9144accca5ad3fa16538449b20ab51734dbdc9ef22288ace4969800000000001976a9145ac4de0775a027d0f0ec54e4c3d55fe57229fc4e88ace4969800000000001976a9147d224284b61bb5c3c0a8f32324052efd4781e28b88ace4969800000000001976a914900be7732a34e46d00dbaace243f65486448f8e388ace4969800000000001976a914a16dece64baadfec54d6ca0bf77f7380a66150c388ace4969800000000001976a914cc0822f608a13008b559d9536e69251f71e1096e88ace4969800000000001976a914f43dc02e4228e32df7c9d04fe3a59f69be05da6788ace4969800000000001976a914fe70994bce368c86578ae95845e1be7a908df06388ace4969800000000001976a914ff2f09b4f1e2f6e48b1e2a470ca1db3fe20baf6988ac00000000');

    mockData.blocks.forEach((mockedBlockData) => {
      coreRpcMock.getBlock.withArgs(mockedBlockData.hash).resolves(mockedBlockData);
    });

    mockData.blocks.forEach((mockedBlockData) => {
      coreRpcMock.getBlockHash.withArgs(mockedBlockData.height).resolves(mockedBlockData.hash);
    });

    bloomFilter = BloomFilter.create(1, 0.001);
  });

  it('count is lesser than max block headers', async () => {
    const fetchHistoricalTransactions = getHistoricalTransactionsIteratorFactory(coreRpcMock);
    const fromBlockHash = '45afbfe270014d5593cb065562f1fed726f767fe334d8b3f4379025cfa5be8c5';
    const count = 580;

    const merkleBlocksAndTransactions = fetchHistoricalTransactions(
      bloomFilter,
      fromBlockHash,
      count,
    );

    const { value: { merkleBlock, transactions } } = await merkleBlocksAndTransactions.next();

    expect(coreRpcMock.getBlock.callCount).to.be.equal(1);
    expect(coreRpcMock.getBlock.getCall(0).calledWith(mockData.blocks[0].hash)).to.be.true();

    expect(coreRpcMock.getBlockHash.callCount).to.be.equal(1);
    expect(coreRpcMock.getBlockHash.getCall(0).calledWith(mockData.blocks[0].height)).to.be.true();

    expect(coreRpcMock.getMerkleBlocks.callCount).to.be.equal(1);
    expect(
      coreRpcMock.getMerkleBlocks.getCall(0).calledWith(
        bloomFilter.toBuffer().toString('hex'), mockData.blocks[0].hash, 580,
      ),
    ).to.be.true();

    expect(merkleBlock).to.be.an.instanceof(MerkleBlock);
    expect(transactions).to.be.an('array');
    transactions.forEach((rawTx) => {
      expect(rawTx).to.be.an.instanceof(Transaction);
    });

    const { done } = await merkleBlocksAndTransactions.next();

    expect(done).to.be.true();
  });

  it('count is bigger than max block headers', async () => {
    const fetchHistoricalTransactions = getHistoricalTransactionsIteratorFactory(coreRpcMock);
    const fromBlockHash = '45afbfe270014d5593cb065562f1fed726f767fe334d8b3f4379025cfa5be8c5';
    const count = 4123;

    const merkleBlocksIterator = fetchHistoricalTransactions(
      bloomFilter,
      fromBlockHash,
      count,
    );

    const { value: { merkleBlock, transactions } } = await merkleBlocksIterator.next();

    expect(coreRpcMock.getBlock.callCount).to.be.equal(1);
    expect(coreRpcMock.getBlock.getCall(0).calledWith(mockData.blocks[0].hash)).to.be.true();

    expect(coreRpcMock.getBlockHash.callCount).to.be.equal(1);
    expect(coreRpcMock.getBlockHash.getCall(0).calledWith(mockData.blocks[0].height)).to.be.true();

    expect(coreRpcMock.getMerkleBlocks.callCount).to.be.equal(1);
    expect(
      coreRpcMock.getMerkleBlocks.getCall(0).calledWith(
        bloomFilter.toBuffer().toString('hex'), mockData.blocks[0].hash, 2000,
      ),
    ).to.be.true();

    expect(merkleBlock).to.be.an.instanceof(MerkleBlock);
    expect(transactions).to.be.an('array');
    transactions.forEach((rawTx) => {
      expect(rawTx).to.be.an.instanceof(Transaction);
    });

    await merkleBlocksIterator.next();

    expect(coreRpcMock.getBlock.callCount).to.be.equal(1);

    expect(coreRpcMock.getBlockHash.callCount).to.be.equal(2);
    expect(coreRpcMock.getBlockHash.getCall(1).calledWith(mockData.blocks[2].height)).to.be.true();

    expect(coreRpcMock.getMerkleBlocks.callCount).to.be.equal(2);
    expect(
      coreRpcMock.getMerkleBlocks.getCall(1).calledWith(
        bloomFilter.toBuffer().toString('hex'), mockData.blocks[2].hash, 2000,
      ),
    ).to.be.true();

    await merkleBlocksIterator.next();

    expect(coreRpcMock.getBlock.callCount).to.be.equal(1);

    expect(coreRpcMock.getBlockHash.callCount).to.be.equal(3);
    expect(coreRpcMock.getBlockHash.getCall(2).calledWith(mockData.blocks[3].height)).to.be.true();

    expect(coreRpcMock.getMerkleBlocks.callCount).to.be.equal(3);
    expect(
      coreRpcMock.getMerkleBlocks.getCall(2).calledWith(
        bloomFilter.toBuffer().toString('hex'), mockData.blocks[3].hash, 123,
      ),
    ).to.be.true();

    const { done } = await merkleBlocksIterator.next();

    expect(done).to.be.true();
  });

  it('should return one merkle block at a time, even if there is more than two blocks found in a range', async () => {
    coreRpcMock.getMerkleBlocks
      .withArgs()
      .resolves([rawMerkleBlock, rawMerkleBlock]);

    const fetchHistoricalTransactions = getHistoricalTransactionsIteratorFactory(coreRpcMock);
    const fromBlockHash = '45afbfe270014d5593cb065562f1fed726f767fe334d8b3f4379025cfa5be8c5';
    const count = 580;

    const merkleBlocksAndTransactions = fetchHistoricalTransactions(
      bloomFilter,
      fromBlockHash,
      count,
    );

    const { value: { merkleBlock, transactions } } = await merkleBlocksAndTransactions.next();

    expect(coreRpcMock.getBlock.callCount).to.be.equal(1);
    expect(coreRpcMock.getBlock.getCall(0).calledWith(mockData.blocks[0].hash)).to.be.true();

    expect(coreRpcMock.getBlockHash.callCount).to.be.equal(1);
    expect(coreRpcMock.getBlockHash.getCall(0).calledWith(mockData.blocks[0].height)).to.be.true();

    expect(coreRpcMock.getMerkleBlocks.callCount).to.be.equal(1);
    expect(
      coreRpcMock.getMerkleBlocks.getCall(0).calledWith(
        bloomFilter.toBuffer().toString('hex'), mockData.blocks[0].hash, 580,
      ),
    ).to.be.true();

    expect(merkleBlock).to.be.an.instanceof(MerkleBlock);
    expect(transactions).to.be.an('array');
    transactions.forEach((rawTx) => {
      expect(rawTx).to.be.an.instanceof(Transaction);
    });

    const {
      value: {
        merkleBlock: secondMerkleBlock,
        transactions: secondSetOfTransactions,
      },
    } = await merkleBlocksAndTransactions.next();

    expect(secondMerkleBlock).to.be.an.instanceof(MerkleBlock);
    expect(secondSetOfTransactions).to.be.an('array');
    secondSetOfTransactions.forEach((rawTx) => {
      expect(rawTx).to.be.an.instanceof(Transaction);
    });

    expect(coreRpcMock.getBlock.callCount).to.be.equal(1);
    expect(coreRpcMock.getBlockHash.callCount).to.be.equal(1);
    expect(coreRpcMock.getMerkleBlocks.callCount).to.be.equal(1);

    const { done } = await merkleBlocksAndTransactions.next();

    expect(done).to.be.true();
  });

  it('should skip interval with no merkle blocks', async () => {
    coreRpcMock.getMerkleBlocks
      .withArgs(bloomFilter, mockData.blocks[2].hash, 2000)
      .resolves([]);

    const fromBlockHash = '45afbfe270014d5593cb065562f1fed726f767fe334d8b3f4379025cfa5be8c5';
    const count = 4123;

    const fetchHistoricalTransactions = getHistoricalTransactionsIteratorFactory(coreRpcMock);

    const merkleBlocksIterator = fetchHistoricalTransactions(
      bloomFilter,
      fromBlockHash,
      count,
    );

    await merkleBlocksIterator.next();

    expect(coreRpcMock.getBlockHash.callCount).to.be.equal(1);
    expect(coreRpcMock.getMerkleBlocks.callCount).to.be.equal(1);

    await merkleBlocksIterator.next();

    expect(coreRpcMock.getMerkleBlocks.getCall(1)
      .calledWith(bloomFilter.toBuffer().toString('hex'), mockData.blocks[2].hash, 2000)).to.be.true();

    await merkleBlocksIterator.next();

    // As there will be one interval (4002-6002) with no merkle blocks,
    // all call count should increase by 2
    expect(coreRpcMock.getBlockHash.callCount).to.be.equal(3);
    expect(coreRpcMock.getMerkleBlocks.callCount).to.be.equal(3);
  });

  it('should proceed straight to done if all ranges are empty', async () => {
    coreRpcMock.getMerkleBlocks
      .withArgs()
      .resolves([]);

    const fromBlockHash = '45afbfe270014d5593cb065562f1fed726f767fe334d8b3f4379025cfa5be8c5';
    const count = 4123;

    const fetchHistoricalTransactions = getHistoricalTransactionsIteratorFactory(coreRpcMock);

    const merkleBlocksIterator = fetchHistoricalTransactions(
      bloomFilter,
      fromBlockHash,
      count,
    );

    const { done } = await merkleBlocksIterator.next();

    expect(coreRpcMock.getBlockHash.callCount).to.be.equal(3);
    expect(coreRpcMock.getMerkleBlocks.callCount).to.be.equal(3);
    expect(done).to.be.true();
  });
});
