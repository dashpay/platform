const { expect } = require('chai');
const mockedStore = require('../../../../fixtures/sirentonight-fullstore-snapshot-1562711703');
const getIdentityHDKeyById = require('./getIdentityHDKeyById');
const searchTransaction = require('../../Storage/methods/searchTransaction');

let accountMock;
let fetchTransactionInfoCalledNb = 0;
let expectedKeyMock;
describe('Account#getIdentityHDKeyById', function suite() {
  this.timeout(10000);
  before(() => {
    expectedKeyMock = "123";
    const storageMock = {
      store: mockedStore,
      getStore: () => mockedStore,
      mappedAddress: {},
      searchTransaction,
      getIndexedIdentityIds: () => mockedStore.wallets[Object.keys(mockedStore.wallets)].identityIds,
    };
    const walletId = Object.keys(mockedStore.wallets)[0];
    accountMock = {
      walletId,
      index: 0,
      storage: storageMock,
      transporter: {
        getTransaction: () => fetchTransactionInfoCalledNb += 1,
      },
      getIdentityHDKeyByIndex: (identityIndex) => {
        if (identityIndex === 0) {
          return expectedKeyMock;
        }
      }
    };
  });
  it('should filter empty indexes', async () => {
    const key = await getIdentityHDKeyById.call(accountMock, "9Gk9T5mJY9j3dDX1D1tG5WYaV8g6zQTS2ocFFXe6NCrq");
    expect(key).to.deep.equal(expectedKeyMock);
  });
  it('should throw an error if identity id was not found', async () => {
    try {
      await getIdentityHDKeyById.call(accountMock, 'randomstring');
    } catch (e) {
      expect(e.message).to.be.equal('Identity with ID randomstring is not associated with wallet, or it\'s not synced')
    }
  });
});
