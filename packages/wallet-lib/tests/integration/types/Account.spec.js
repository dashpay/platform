const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');

const {
    HDPrivateKey,
    MerkleBlock,
} = require('@dashevo/dashcore-lib');

const TransactionSyncStreamWorker = require('../../../src/plugins/Workers/TransactionSyncStreamWorker/TransactionSyncStreamWorker');

const LocalForageAdapterMock = require('../../../src/test/mocks/LocalForageAdapterMock');
const createTransactionInAccount = require('../../../src/test/mocks/createTransactionInAccount');

const createAndAttachTransportMocksToWallet = require('../../../src/test/mocks/createAndAttachTransportMocksToWallet')

const { Wallet } = require('../../../src');

chai.use(chaiAsPromised);
const { expect } = chai;

describe('Account', function suite() {
    this.timeout(60000);
    let worker;
    let storage;
    let walletId;
    let wallet;
    let account;
    let txStreamMock;
    let address;
    let addressAtIndex19;
    let testHDKey;
    let merkleBlockMock;
    let transportMock;
    let storageAdapterMock;

    beforeEach(async function beforeEach() {
        testHDKey = "xprv9s21ZrQH143K4PgfRZPuYjYUWRZkGfEPuWTEUESMoEZLC274ntC4G49qxgZJEPgmujsmY52eVggtwZgJPrWTMXmbYgqDVySWg46XzbGXrSZ";
        merkleBlockMock = new MerkleBlock(Buffer.from([0, 0, 0, 32, 61, 11, 102, 108, 38, 155, 164, 49, 91, 246, 141, 178, 126, 155, 13, 118, 248, 83, 250, 15, 206, 21, 102, 65, 104, 183, 243, 167, 235, 167, 60, 113, 140, 110, 120, 87, 208, 191, 240, 19, 212, 100, 228, 121, 192, 125, 143, 44, 226, 9, 95, 98, 51, 25, 139, 172, 175, 27, 205, 201, 158, 85, 37, 8, 72, 52, 36, 95, 255, 255, 127, 32, 2, 0, 0, 0, 1, 0, 0, 0, 1, 140, 110, 120, 87, 208, 191, 240, 19, 212, 100, 228, 121, 192, 125, 143, 44, 226, 9, 95, 98, 51, 25, 139, 172, 175, 27, 205, 201, 158, 85, 37, 8, 1, 1]));

        testHDKey = new HDPrivateKey(testHDKey).toString();

        // Override default value of executeOnStart to prevent worker from starting
        worker = new TransactionSyncStreamWorker({executeOnStart: false});

        storageAdapterMock = new LocalForageAdapterMock();

        // This is a full instance of wallet with a mocked transport
        wallet = new Wallet({
            offlineMode: true,
            plugins: [worker],
            allowSensitiveOperations: true,
            HDPrivateKey: new HDPrivateKey(testHDKey),
            adapter: storageAdapterMock
        });

        ({txStreamMock, transportMock} = await createAndAttachTransportMocksToWallet(wallet, this.sinonSandbox));

        account = await wallet.getAccount();

        storage = account.storage;
        walletId = Object.keys(storage.store.wallets)[0];

        address = account.getAddress(0).address;
        addressAtIndex19 = account.getAddress(19).address;
    });

    afterEach(() => {
        worker.stopWorker();
    })

    describe('getUTXO', () => {
        it('should work if storage adapter behaves like a local forage', async () => {
            await createTransactionInAccount(account);

            // Saving state to restore it later
            await account.storage.saveState();

            // Restoring wallet from the saved state
            const restoredWallet = new Wallet({
                offlineMode: true,
                plugins: [worker],
                allowSensitiveOperations: true,
                HDPrivateKey: new HDPrivateKey(testHDKey),
                adapter: storageAdapterMock
            });
            const restoredAccount = await restoredWallet.getAccount();

            const utxos = await restoredAccount.getUTXOS();

            expect(utxos.length).to.be.equal(1);
        });
    });
});