require('../../bootstrap');

const path = require('path');
const dotenvSafe = require('dotenv-safe');

const sinon = require('sinon');

const MNDiscovery = require('../../../src/MNDiscovery/index');
const fetch = require('node-fetch');
const {startDapi} = require('@dashevo/js-evo-services-ctl');
const DAPIClient = require('../../../src/index');

const {
    Transaction,
    PrivateKey,
    PublicKey,
    Address,
} = require('@dashevo/dashcore-lib');

const Schema = require('@dashevo/dash-schema/dash-schema-lib');
const DashPay = require('@dashevo/dash-schema/dash-core-daps');

const doubleSha256 = require('../../utils/doubleSha256');
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

    let dapiClient;
    let dapId;
    let dapSchema;
    let dapContract;

    let faucetPrivateKey;
    let faucetAddress;

    let bobPrivateKey;
    let bobUserName;
    let bobRegTxId;

    let bobPreviousST;

    before(async function it() {
        this.timeout(300000);
        const privKey = "cVwyvFt95dzwEqYCLd8pv9CzktajP4tWH2w9RQNPeHYA7pH35wcJ";
        faucetPrivateKey = new PrivateKey(privKey);

        const faucetPublicKey = PublicKey.fromPrivateKey(faucetPrivateKey);

        faucetAddress = Address
            .fromPublicKey(faucetPublicKey, 'testnet')
            .toString();

        bobUserName = Math.random().toString(36).substring(7);
        aliceUserName = Math.random().toString(36).substring(7);
        dapSchema = Object.assign({}, DashPay);
        dapSchema.title = `TestContacts_${bobUserName}`;

        dapContract = Schema.create.dapcontract(dapSchema);
        dapId = doubleSha256(Schema.serialize.encode(dapContract.dapcontract));

        sinon.stub(MNDiscovery.prototype, 'getRandomMasternode')
            .returns(Promise.resolve({ip: '127.0.0.1'}));

        [masterNode] = await startDapi.many(1);

        const seeds = [{ip: masterNode.dapi.container.getIp()}];
        await masterNode.dashCore.getApi().generate(1500);

        dapiClient = new DAPIClient({
            seeds,
            port: masterNode.dapi.options.getRpcPort(),
        });

        insightURL = `http://127.0.0.1:${masterNode.insight.options.getApiPort()}/insight-api-dash`;

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
            expect(dapiOutput).to.be.deep.equal([
                {
                    "address": faucetAddress,
                    "txid": coreOutput[0].txid,
                    "vout": 0,
                    "scriptPubKey": coreOutput[0].script,
                    "amount": coreOutput[0].satoshis / 100000000,
                    "satoshis": coreOutput[0].satoshis,
                    "height": coreOutput[0].height,
                    "confirmations": 20
                }
            ]);
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
            // curl --user myusername --data-binary '{"jsonrpc": "1.0", "id":"curltest", "method": "getbestblockhash", "params": [] }' -H 'content-type: text/plain;' http://127.0.0.1:27410/
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
                .from(inputs.slice(-1)[0])
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

        it('should getUserByName & getUserById', async function it() {
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
            let {stpacket: stPacket} = Schema.create.stpacket();
            stPacket = Object.assign(stPacket, dapContract);

            // 2. Create State Transition
            const transaction = new Transaction()
                .setType(Transaction.TYPES.TRANSACTION_SUBTX_TRANSITION);

            const serializedPacket = Schema.serialize.encode(stPacket);
            const stPacketHash = doubleSha256(serializedPacket);

            transaction.extraPayload
                .setRegTxId(bobPreviousST)
                .setHashPrevSubTx(bobPreviousST)
                .setHashSTPacket(stPacketHash)
                .setCreditFee(1000)
                .sign(bobPrivateKey);

            const transitionHash = await dapiClient.sendRawTransition(
                transaction.serialize(),
                serializedPacket.toString('hex'),
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
                    dapContractFromDAPI = await dapiClient.fetchDapContract(dapId);
                    break;
                } catch (e) {
                    await dapiClient.generate(1);
                }
            }

            expect(dapContractFromDAPI).to.have.property('dapname');
            expect(dapContractFromDAPI.dapname).to.be.equal(dapSchema.title);
        });

        it('should fetchDapObjects', async function it() {

            const userRequest = Schema.create.dapobject('user');
            userRequest.aboutme = 'This is story about me';
            userRequest.avatar = 'My avatar here';
            userRequest.act = 0;

            // 1. Create ST profile packet
            const {stpacket: stPacket} = Schema.create.stpacket();
            stPacket.dapobjects = [userRequest];
            stPacket.dapid = dapId;

            // 2. Create State Transition
            const transaction = new Transaction()
                .setType(Transaction.TYPES.TRANSACTION_SUBTX_TRANSITION);

            const serializedPacket = Schema.serialize.encode(stPacket);
            const stPacketHash = doubleSha256(serializedPacket);

            transaction.extraPayload
                .setRegTxId(bobRegTxId)
                .setHashPrevSubTx(bobPreviousST)
                .setHashSTPacket(stPacketHash)
                .setCreditFee(1000)
                .sign(bobPrivateKey);

            const transitionHash = await dapiClient.sendRawTransition(
                transaction.serialize(),
                serializedPacket.toString('hex'),
            );

            expect(transitionHash).to.be.a('string');
            expect(transitionHash).to.be.not.empty();

            bobPreviousST = transitionHash;

            let bobSpace;
            for (let i = 0; i <= attempts; i++) {
                bobSpace = await dapiClient.fetchDapObjects(dapId, 'user', {});
                // waiting for Bob's profile to be added
                if (bobSpace.length > 0) {
                    break;
                } else {
                    await dapiClient.generate(1);
                }
            }

            expect(bobSpace).to.have.lengthOf(1);
            expect(bobSpace[0]).to.be.deep.equal(
                {
                    act: 0,
                    idx: 0,
                    rev: 0,
                    avatar: 'My avatar here',
                    aboutme: 'This is story about me',
                    pver: null,
                    objtype: 'user',
                },
            );
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
        it('requestHistoricData', async function it() {
        });
        it('getMnListDiff', async function it() {
        });
    });

});
