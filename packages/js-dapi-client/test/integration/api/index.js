require('../../bootstrap');

const path = require('path');
const dotenvSafe = require('dotenv-safe');

const sinon = require('sinon');

const MNDiscovery = require('../../../src/MNDiscovery/index');
const fetch = require('node-fetch');
const {startDapi} = require('@dashevo/dp-services-ctl');
const DAPIClient = require('../../../src/index');

const {
    Transaction,
    PrivateKey,
    PublicKey,
    Address,
} = require('@dashevo/dashcore-lib');

const DashPlatformProtocol = require('@dashevo/dpp');
const entropy = require('@dashevo/dpp/lib/util/entropy');
const DPObject = require('@dashevo/dpp/lib/object/DPObject');

const wait = require('../../utils/wait');

process.env.NODE_ENV = 'test';

dotenvSafe.config({
    sample : path.resolve(__dirname, '../.env'),
    path: path.resolve(__dirname, '../.env'),
});


describe('basicAPIs', () => {
    let masterNode;

    const attempts = 40;

    let transactionIdSendToAddress;
    let insightURL;

    let dpp;

    let dapiClient;

    let faucetPrivateKey;
    let faucetAddress;

    let bobPrivateKey;
    let bobUserName;
    let bobRegTxId;

    let bobPreviousST;

    before(async function it() {
        dpp = new DashPlatformProtocol();
        this.timeout(400000);
        const privKey = "cVwyvFt95dzwEqYCLd8pv9CzktajP4tWH2w9RQNPeHYA7pH35wcJ";
        faucetPrivateKey = new PrivateKey(privKey);

        const faucetPublicKey = PublicKey.fromPrivateKey(faucetPrivateKey);

        faucetAddress = Address
            .fromPublicKey(faucetPublicKey, 'testnet')
            .toString();

        bobUserName = Math.random().toString(36).substring(7);
        aliceUserName = Math.random().toString(36).substring(7);
        const dpContract = dpp.contract.create(entropy.generate().substr(0, 24), {
            user: {
                properties: {
                    avatarUrl: {
                        type: 'string',
                        format: 'url',
                    },
                    about: {
                        type: 'string',
                    },
                },
                required: ['avatarUrl', 'about'],
                additionalProperties: false,
            },
            contact: {
                properties: {
                    toUserId: {
                        type: 'string',
                    },
                    publicKey: {
                        type: 'string',
                    },
                },
                required: ['toUserId', 'publicKey'],
                additionalProperties: false,
            },
        });
        dpp.setDPContract(dpContract);

        sinon.stub(MNDiscovery.prototype, 'getRandomMasternode')
            .returns(Promise.resolve({ip: '127.0.0.1'}));

        [masterNode] = await startDapi.many(1);

        const seeds = [{ip: masterNode.dapi.container.getIp()}];
        await masterNode.dashCore.getApi().generate(1500);

        dapiClient = new DAPIClient({
            seeds,
            port: masterNode.dapi.options.getRpcPort(),
        });

        insightURL = `http://127.0.0.1:${masterNode.insight.options.getApiPort()}/insight-api`;

        transactionIdSendToAddress = await masterNode.dashCore.getApi().sendToAddress(faucetAddress, 100);
        await dapiClient.generate(20);
        let result = await masterNode.dashCore.getApi().getAddressUtxos({"addresses": ["ygPcCwVy7Fxg7ruxZzqVYdPLtvw7auHAFh"]});
        await wait(20000);

    });

    after('cleanup lone services', async () => {
        const instances = [
            masterNode,
        ];

        await Promise.all(instances.filter(i => i)
            .map(i => i.remove()));

        MNDiscovery.prototype.getRandomMasternode.restore();
    });

    describe('Address', () => {
        it('should return correct getUTXO', async function it() {
            let dapiOutput = await dapiClient.getUTXO(faucetAddress);
            const {result: coreOutput} = await masterNode.dashCore.getApi().getAddressUtxos({"addresses": [faucetAddress]});
            expect(dapiOutput).to.be.deep.equal(
              {
                  "totalItems": 1,
                  "from": 0,
                  "to": 1,
                  "items": [
                      {
                          "address": faucetAddress,
                          "txid": coreOutput[0].txid,
                          "outputIndex": 0,
                          "script": coreOutput[0].script,
                          "satoshis": coreOutput[0].satoshis,
                          "height": coreOutput[0].height
                      }
                  ]
              }
              // why we missed confirmations?
            );
        });

        it('should return correct getAddressSummary', async function it() {
            let dapiOutput = await dapiClient.getAddressSummary(faucetAddress);
            const {result: coreOutput} = await masterNode.dashCore.getApi().getAddressUtxos({"addresses": [faucetAddress]});
            expect(dapiOutput).to.be.deep.equal({
                "addrStr": faucetAddress,
                "balance": coreOutput[0].satoshis / 100000000,
                "balanceSat": coreOutput[0].satoshis,
                "totalReceived": coreOutput[0].satoshis / 100000000,
                "totalReceivedSat": coreOutput[0].satoshis,
                "totalSent": 0,
                "totalSentSat": 0,
                "transactions": [
                    transactionIdSendToAddress.result,
                ],
                "txApperances": 1,
                "unconfirmedBalance": 0,
                "unconfirmedBalanceSat": 0,
                "unconfirmedTxApperances": 0,
            });
        });

        it('should return correct getAddressUnconfirmedBalance', async function it() {
            let dapiOutput = await dapiClient.getAddressUnconfirmedBalance(faucetAddress);
            const url = insightURL + `/addr/${faucetAddress}/unconfirmedBalance`;
            const response = await fetch(url);
            let value = await response.text();
            expect(dapiOutput).to.be.deep.equal(parseInt(value));
        });

        it('should return correct getAddressTotalReceived', async function it() {
            let dapiOutput = await dapiClient.getAddressTotalReceived(faucetAddress);
            const url = insightURL + `/addr/${faucetAddress}/totalReceived`;
            const response = await fetch(url);
            const value = await response.json();
            expect(dapiOutput).to.be.deep.equal(value);
        });

        it('should return correct getAddressTotalSent', async function it() {
            let dapiOutput = await dapiClient.getAddressTotalSent(faucetAddress);
            const url = insightURL + `/addr/${faucetAddress}/totalSent`;
            const response = await fetch(url);
            const value = await response.json();
            expect(dapiOutput).to.be.deep.equal(value); // todo add verification after sending

        });

        it('should return correct getTransactionsByAddress', async function it() {
            let dapiOutput = await dapiClient.getTransactionsByAddress(faucetAddress);
            const url = insightURL + `/txs/?address=${faucetAddress}`;
            const response = await fetch(url);
            const value = await response.json();
            expect(dapiOutput).to.be.deep.equal({
                from: 0,
                items: value.txs,
                to: 1,
                totalItems: value.pagesTotal
            });
        });
    });

    describe('Block', () => {

        it('should return correct getBestBlockHash', async function it() {
            const dapiOutput = await dapiClient.getBestBlockHash();
            const coreOutput = await masterNode.dashCore.getApi().getbestblockhash();
            expect(dapiOutput).to.be.deep.equal(coreOutput.result);
        });

        it('should return correct getBestBlockHeight', async function it() {
            const dapiOutput = await dapiClient.getBestBlockHeight();
            const coreOutput = await masterNode.dashCore.getApi().getblockcount();

            expect(dapiOutput).to.be.deep.equal(coreOutput.result);
        });

        it('should return correct getBlockHeaders', async function it() {
            const height = await dapiClient.getBestBlockHeight();
            let dapiOutput = await dapiClient.getBlockHeaders(height, 1);
            const blockHash = await dapiClient.getBlockHash(height);
            const coreOutput = await masterNode.dashCore.getApi().getblockheaders(blockHash);
            expect(dapiOutput).to.be.deep.equal([coreOutput.result[0]]);
        });

        it('should return correct getBlocks', async function it() {
            const today = new Date().toISOString().substring(0, 10);
            const dapiOutput = await dapiClient.getBlocks(today, 1);
            const url = insightURL + `/blocks?blockDate=${today}&limit=1`;
            const response = await fetch(url);
            const value = await response.json();
            expect(dapiOutput).to.be.deep.equal(value.blocks);
            expect(dapiOutput).to.be.an('array')
        });

        it('should return correct getRawBlock', async function it() {
            const blockHash = await dapiClient.getBestBlockHash();
            const dapiOutput = await dapiClient.getRawBlock(blockHash);
            const url = insightURL + `/rawblock/${blockHash}`;
            const response = await fetch(url);
            const value = await response.json();
            expect(dapiOutput).to.be.deep.equal(value);

        });

        it('should return correct getHistoricBlockchainDataSyncStatus', async function it() {
            let dapiOutput = await dapiClient.getHistoricBlockchainDataSyncStatus();
            const url = insightURL + `/sync`;
            const response = await fetch(url);
            const value = await response.json();
            expect(dapiOutput).to.be.deep.equal(value);

        });

    });

    describe('Mempool', () => {

        it('should return correct getMempoolInfo output', async function it() {
          const dapiOutput = await dapiClient.getMempoolInfo();
          const coreOutput = await masterNode.dashCore.getApi().getmempoolinfo();
          expect(dapiOutput).to.be.deep.equal(coreOutput.result);
        });

    });

    describe('Transaction', () => {
        it('should return correct getTransaction', async function it() {
            let dapiOutput = await dapiClient.getTransaction(transactionIdSendToAddress.result);
            const url = insightURL + `/tx/${transactionIdSendToAddress.result}`;
            const response = await fetch(url);
            const value = await response.json();
            expect(dapiOutput).to.be.deep.equal(value);

        });

        it('should return correct getTransactionById', async function it() {
            let dapiOutput = await dapiClient.getTransactionById(transactionIdSendToAddress.result);
            const url = insightURL + `/tx/${transactionIdSendToAddress.result}`;
            const response = await fetch(url);
            const value = await response.json();
            expect(dapiOutput).to.be.deep.equal(value);

        });
    });

    describe('All APIs', () => {
        it('should sendRawTransaction', async function it() {
            this.timeout(50000);

            bobPrivateKey = new PrivateKey();
            const validPayload = new Transaction.Payload.SubTxRegisterPayload()
                .setUserName(bobUserName)
                .setPubKeyIdFromPrivateKey(bobPrivateKey).sign(bobPrivateKey);

            let inputs = await dapiClient.getUTXO(faucetAddress);

            const transaction = Transaction()
                .setType(Transaction.TYPES.TRANSACTION_SUBTX_REGISTER)
                .setExtraPayload(validPayload)
                .from(inputs.items.slice(-1)[0])
                .addFundingOutput(10000)
                .change(faucetAddress)
                .sign(faucetPrivateKey);

            const result = await dapiClient.sendRawTransaction(transaction.serialize());
            expect(result).to.be.a('string');
            expect(result).to.be.not.empty();
            bobRegTxId = result;

            bobPreviousST = result;

        });

        it('should generate', async function it() {
            const height = await dapiClient.getBestBlockHeight();
            await dapiClient.generate(1);
            const heightAfter = await dapiClient.getBestBlockHeight();
            expect(height).to.be.equal(heightAfter - 1);
            await wait(5000);
        });

        it('should estimateFee', async function it() {
            const estimateFee = await dapiClient.estimateFee(2);
            expect(estimateFee).to.be.deep.equal(1);
        });

        xit('should getUserByName & getUserById', async function it() {
            const userByName = await dapiClient.getUserByName(bobUserName);
            expect(userByName.uname).to.be.equal(bobUserName);

            const userByid = await dapiClient.getUserById(userByName.regtxid);
            expect(userByid).to.be.deep.equal(userByName);
            expect(userByid).to.be.deep.equal({
                "uname": bobUserName,
                "regtxid": bobRegTxId,
                "pubkeyid": userByName.pubkeyid,
                "credits": 10000,
                "data": "0000000000000000000000000000000000000000000000000000000000000000",
                "state": "open",
                "subtx": [
                    bobRegTxId
                ]
            });

        });

        it('should searchUsers', async function it() {
            let dapiOutput = await dapiClient.searchUsers(bobUserName);
            expect(dapiOutput).to.be.deep.equal({
                "totalCount": 0,
                "results": []
            });
        });

        it('should sendRawTransition', async function it() {

            // 1. Create ST packet
            const stPacket = dpp.packet.create(dpp.getDPContract());

            // 2. Create State Transition
            const transaction = new Transaction()
                .setType(Transaction.TYPES.TRANSACTION_SUBTX_TRANSITION);

            transaction.extraPayload
                .setRegTxId(bobPreviousST)
                .setHashPrevSubTx(bobPreviousST)
                .setHashSTPacket(stPacket.hash())
                .setCreditFee(1000)
                .sign(bobPrivateKey);

            const transitionHash = await dapiClient.sendRawTransition(
              stPacket.serialize().toString('hex'),
                transaction.serialize()
              ,
            );

            expect(transitionHash).to.be.a('string');
            expect(transitionHash).to.be.not.empty();

            bobPreviousST = transitionHash;
        });

        it('should fetchDapContract', async function it() {
            let dapContractFromDAPI;

            for (let i = 0; i <= attempts; i++) {
                try {
                    // waiting for Contacts to be added
                    dapContractFromDAPI = await dapiClient.fetchDapContract(dpp.getDPContract().getId());
                    break;
                } catch (e) {
                    await dapiClient.generate(1);
                }
            }
            let expectedContract = JSON.parse(JSON.stringify(dpp.getDPContract()));
            delete expectedContract['definitions'];
            delete expectedContract['schema'];
            expectedContract.$schema = 'https://schema.dash.org/dpp-0-4-0/meta/dp-contract';
            expect(dapContractFromDAPI).to.be.deep.equal(expectedContract);
        });

        it('should fetchDapObjects', async function it() {
            dpp.setUserId(bobRegTxId);

            const user = dpp.object.create('user', {
                avatarUrl: 'http://test.com/bob.jpg',
                about: 'This is story about me',
            });

            // 1. Create ST profile packet
            const stPacket = dpp.packet.create([user]);

            // 2. Create State Transition
            const transaction = new Transaction()
                .setType(Transaction.TYPES.TRANSACTION_SUBTX_TRANSITION);

            transaction.extraPayload
                .setRegTxId(bobRegTxId)
                .setHashPrevSubTx(bobPreviousST)
                .setHashSTPacket(stPacket.hash())
                .setCreditFee(1000)
                .sign(bobPrivateKey);

            const transitionHash = await dapiClient.sendRawTransition(
              stPacket.serialize().toString('hex'),
              transaction.serialize()
              ,
            );

            expect(transitionHash).to.be.a('string');
            expect(transitionHash).to.be.not.empty();

            bobPreviousST = transitionHash;

            let users;
            for (let i = 0; i <= attempts; i++) {
                users = await dapiClient.fetchDapObjects(
                  dpp.getDPContract().getId(),
                  'user',
                  {},
                );
                // waiting for Bob's profile to be added
                if (users.length > 0) {
                    break;
                } else {
                    await dapiClient.generate(1);
                }
            }

            expect(users).to.have.lengthOf(1);
            expect(users[0]).to.be.deep.equal(user.toJSON());
        });
    });


    xdescribe('TODO', () => {
        it('loadBloomFilter', async function it() {
        });
        it('sendRawIxTransaction', async function it() {
        });
        it('addToBloomFilter', async function it() {
        });
        it('clearBloomFilter', async function it() {
        });
        it('getSpvData', async function it() {
        });
        it('getMnListDiff', async function it() {
        });
    });

});
