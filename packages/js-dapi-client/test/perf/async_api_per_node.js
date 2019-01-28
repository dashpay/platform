require('../bootstrap');

const path = require('path');

const sinon = require('sinon');

const Schema = require('@dashevo/dash-schema/dash-schema-lib');
const DashPay = require('@dashevo/dash-schema/dash-core-daps');
const {
    Transaction,
    PrivateKey,
} = require('@dashevo/dashcore-lib');

const doubleSha256 = require('../utils/doubleSha256');
const wait = require('../utils/wait');
const MNDiscovery = require('../../src/MNDiscovery/index');

const average = arr => arr.reduce((p, c) => p + c, 0) / arr.length;

require('dotenv').config({path: path.resolve(__dirname, '.env')});


const DAPIClient = require('../../src/index');

describe("Performance", function () {
    const timeoutTest = 320000;
    const numRequests = 100;
    const numPartRequests = 30; // for getUTXO requests
    const numLoops = 5;
    const faucetAddress = process.env.faucetAddress;
    const privKey = process.env.privKey;
    const faucetPrivateKey = new PrivateKey(privKey);
    let dapiClient;


    before("set dapi node", function () {

        const seeds = [{ip: process.env.DAPI_IP}];
        sinon.stub(MNDiscovery.prototype, 'getRandomMasternode')
            .returns(Promise.resolve({ip: process.env.DAPI_IP}));
        dapiClient = new DAPIClient({seeds, port: 3000});
        spy = sinon.spy(dapiClient, 'makeRequestToRandomDAPINode');
    });


    beforeEach(async () => {
        spy.resetHistory();
        // wait when dapi restored after any crashes ( getUTXO for example)
        await wait(20000);
    });

    after(() => {
        spy.restore();
    });

    async function runPromise(queries) {
        const start = new Date();
        return await Promise.all(queries)
            .then(result => {
                const delta = new Date() - start;
                console.log(delta);
                return {time: delta, result: result};
            })
            .catch(err => {
                return Promise.reject(err)
            });
    }

    describe('Address', () => {
        // https://dashpay.atlassian.net/browse/EV-1208 dapi&dashd crashed on devnet with 20 async getUTXO requests
        it("getUTXO", async function it() {
            this.timeout(timeoutTest * 2);
            let results = [];
            for (var i = 0; i < numLoops; i += 1) {
                const queries = new Array(numPartRequests);
                for (let index = 0; index < numPartRequests; ++index) {
                    queries[index] = dapiClient.getUTXO(faucetAddress);

                }
                await runPromise(queries).then(function (result) {
                    results.push(result.time);
                }, function (failure) {
                    expect(failure, 'Errors found').to.be.undefined;
                });
            }
            const result = average(results);
            expect(spy.callCount).to.be.equal(numLoops * numPartRequests);
            expect(result).to.not.be.NaN;
            expect(result).to.be.a('number');
            console.log("average:", result);
        });

        it("getAddressSummary", async function it() {
            this.timeout(timeoutTest);
            let results = [];
            for (var i = 0; i < numLoops; i += 1) {
                const queries = new Array(numRequests);
                for (let index = 0; index < numRequests; ++index) {
                    queries[index] = dapiClient.getAddressSummary(faucetAddress);

                }
                await runPromise(queries).then(function (result) {
                    results.push(result.time);
                }, function (failure) {
                    expect(failure, 'Errors found').to.be.undefined;
                });
            }
            const result = average(results);
            expect(spy.callCount).to.be.equal(numLoops * numRequests);
            expect(result).to.not.be.NaN;
            expect(result).to.be.a('number');
            console.log("average:", result);

        });

        it("getAddressUnconfirmedBalance", async function it() {
            this.timeout(timeoutTest);
            let results = [];
            for (var i = 0; i < numLoops; i += 1) {
                const queries = new Array(numRequests);
                for (let index = 0; index < numRequests; ++index) {
                    queries[index] = dapiClient.getAddressUnconfirmedBalance(faucetAddress);

                }
                await runPromise(queries).then(function (result) {
                    results.push(result.time);
                }, function (failure) {
                    expect(failure, 'Errors found').to.be.undefined;
                });
            }
            const result = average(results);
            expect(result).to.not.be.NaN;
            expect(result).to.be.a('number');
            expect(spy.callCount).to.be.equal(numLoops * numRequests);
            console.log("average:", result);

        });

        it("getAddressTotalReceived", async function it() {
            this.timeout(timeoutTest);
            let results = [];
            for (var i = 0; i < numLoops; i += 1) {
                const queries = new Array(numRequests);
                for (let index = 0; index < numRequests; ++index) {
                    queries[index] = dapiClient.getAddressTotalReceived(faucetAddress);

                }
                await runPromise(queries).then(function (result) {
                    results.push(result.time);
                }, function (failure) {
                    expect(failure, 'Errors found').to.be.undefined;
                });
            }
            const result = average(results);
            expect(spy.callCount).to.be.equal(numLoops * numRequests);
            expect(result).to.not.be.NaN;
            expect(result).to.be.a('number');
            console.log("average:", result);

        });

        it("getAddressTotalSent", async function it() {
            this.timeout(timeoutTest);
            let results = [];
            for (var i = 0; i < numLoops; i += 1) {
                const queries = new Array(numRequests);
                for (let index = 0; index < numRequests; ++index) {
                    queries[index] = dapiClient.getAddressTotalSent(faucetAddress);

                }
                await runPromise(queries).then(function (result) {
                    results.push(result.time);
                }, function (failure) {
                    expect(failure, 'Errors found').to.be.undefined;
                });
            }
            const result = average(results);
            expect(spy.callCount).to.be.equal(numLoops * numRequests);
            expect(result).to.not.be.NaN;
            expect(result).to.be.a('number');
            console.log("average per loop:", result);

        });

        it("getTransactionsByAddress", async function it() {
            this.timeout(timeoutTest);
            let results = [];
            for (var i = 0; i < numLoops; i += 1) {
                const queries = new Array(numRequests);
                for (let index = 0; index < numRequests; ++index) {
                    queries[index] = dapiClient.getTransactionsByAddress(faucetAddress);

                }
                await runPromise(queries).then(function (result) {
                    results.push(result.time);
                }, function (failure) {
                    expect(failure, 'Errors found').to.be.undefined;
                });
            }
            const result = average(results);
            expect(spy.callCount).to.be.equal(numLoops * numRequests);
            expect(result).to.not.be.NaN;
            expect(result).to.be.a('number');
            console.log("average per loop:", result);
        });
    });

    describe('Block', () => {
        it("getBestBlockHeight", async function it() {
            this.timeout(timeoutTest);
            let results = [];
            for (var i = 0; i < numLoops; i += 1) {
                const queries = new Array(numRequests);
                for (let index = 0; index < numRequests; ++index) {
                    queries[index] = dapiClient.getBestBlockHeight();

                }
                await runPromise(queries).then(function (result) {
                    results.push(result.time);
                }, function (failure) {
                    expect(failure, 'Errors found').to.be.undefined;
                });
            }
            const result = average(results);
            expect(spy.callCount).to.be.equal(numLoops * numRequests);
            expect(result).to.not.be.NaN;
            expect(result).to.be.a('number');
            console.log("average:", result);
        });

        it("getBlockHash", async function it() {
            this.timeout(timeoutTest);
            const height = await dapiClient.getBestBlockHeight();
            let results = [];
            for (var i = 0; i < numLoops; i += 1) {
                const queries = new Array(numRequests);
                for (let index = 0; index < numRequests; ++index) {
                    queries[index] = dapiClient.getBlockHash(height);

                }
                await runPromise(queries).then(function (result) {
                    results.push(result.time);
                }, function (failure) {
                    expect(failure, 'Errors found').to.be.undefined;
                });
            }
            const result = average(results);
            expect(spy.callCount).to.be.equal(numLoops * numRequests + 1);
            expect(result).to.not.be.NaN;
            expect(result).to.be.a('number');
            console.log("average:", result);

        });

        it("getBlockHeaders", async function it() {
            this.timeout(timeoutTest);
            const height = await dapiClient.getBestBlockHeight();
            let results = [];
            for (var i = 0; i < numLoops; i += 1) {
                const queries = new Array(numRequests);
                for (let index = 0; index < numRequests; ++index) {
                    queries[index] = dapiClient.getBlockHeaders(height, 3);

                }
                await runPromise(queries).then(function (result) {
                    results.push(result.time);
                }, function (failure) {
                    expect(failure, 'Errors found').to.be.undefined;
                });
            }
            const result = average(results);
            expect(result).to.not.be.NaN;
            expect(result).to.be.a('number');
            expect(spy.callCount).to.be.equal(numLoops * numRequests + 1);
            console.log("average:", result);

        });

        it("getBlocks", async function it() {
            this.timeout(timeoutTest);
            const today = new Date().toISOString().substring(0, 10);
            let results = [];
            for (var i = 0; i < numLoops; i += 1) {
                const queries = new Array(numRequests);
                for (let index = 0; index < numRequests; ++index) {
                    queries[index] = dapiClient.getBlocks(today, 1);

                }
                await runPromise(queries).then(function (result) {
                    results.push(result.time);
                }, function (failure) {
                    expect(failure, 'Errors found').to.be.undefined;
                });
            }
            const result = average(results);
            expect(spy.callCount).to.be.equal(numLoops * numRequests);
            expect(result).to.not.be.NaN;
            expect(result).to.be.a('number');
            console.log("average:", result);

        });

        // https://dashpay.atlassian.net/browse/EV-1207 dapi-client: getRawBlock kills insight-api
        it("getRawBlock", async function it() {
            this.timeout(timeoutTest);
            const height = await dapiClient.getBestBlockHeight();
            const blockHash = await dapiClient.getBlockHash(height);
            let results = [];
            for (var i = 0; i < numLoops; i += 1) {
                const queries = new Array(numRequests);
                for (let index = 0; index < numRequests; ++index) {
                    queries[index] = dapiClient.getRawBlock(blockHash);

                }
                await runPromise(queries).then(function (result) {
                    results.push(result.time);
                }, function (failure) {
                    expect(failure, 'Errors found').to.be.undefined;
                });
            }
            const result = average(results);
            expect(spy.callCount).to.be.equal(numLoops * numRequests + 2);
            expect(result).to.not.be.NaN;
            expect(result).to.be.a('number');
            console.log("average:", result);

        });

        it("getHistoricBlockchainDataSyncStatus", async function it() {
            this.timeout(timeoutTest);
            let results = [];
            for (var i = 0; i < numLoops; i += 1) {
                const queries = new Array(numRequests);
                for (let index = 0; index < numRequests; ++index) {
                    queries[index] = dapiClient.getHistoricBlockchainDataSyncStatus();

                }
                await runPromise(queries).then(function (result) {
                    results.push(result.time);
                }, function (failure) {
                    expect(failure, 'Errors found').to.be.undefined;
                });
            }
            const result = average(results);
            expect(spy.callCount).to.be.equal(numLoops * numRequests);
            expect(result).to.not.be.NaN;
            expect(result).to.be.a('number');
            console.log("average:", result);

        });

    });

    describe('Transaction', () => {
        it("getTransaction", async function it() {
            this.timeout(timeoutTest);
            const trxs = await dapiClient.getTransactionsByAddress(faucetAddress);
            let results = [];
            for (var i = 0; i < numLoops; i += 1) {
                const queries = new Array(numRequests);
                for (let index = 0; index < numRequests; ++index) {
                    queries[index] = dapiClient.getTransaction(trxs.items[i % trxs.items.length].txid);
                }
                await runPromise(queries).then(function (result) {
                    results.push(result.time);
                }, function (failure) {
                    expect(failure, 'Errors found').to.be.undefined;
                });
            }
            const result = average(results);
            expect(spy.callCount).to.be.equal(numLoops * numRequests + 1);
            expect(result).to.not.be.NaN;
            expect(result).to.be.a('number');
            console.log("average:", result);

        });

        it("getTransactionById", async function it() {
            this.timeout(timeoutTest);
            const trxs = await dapiClient.getTransactionsByAddress(faucetAddress);
            let results = [];
            for (var i = 0; i < numLoops; i += 1) {
                const queries = new Array(numRequests);
                for (let index = 0; index < numRequests; ++index) {
                    queries[index] = dapiClient.getTransactionById(trxs.items[i % trxs.items.length].txid);
                }
                await runPromise(queries).then(function (result) {
                    results.push(result.time);
                }, function (failure) {
                    expect(failure, 'Errors found').to.be.undefined;
                });
            }
            const result = average(results);
            expect(spy.callCount).to.be.equal(numLoops * numRequests + 1);
            expect(result).to.not.be.NaN;
            expect(result).to.be.a('number');
            console.log("average:", result);
        });
    });

    describe('All APIs', () => {
        let bobPrivateKeys = [];
        let bobUserNames = [];
        let bobUserIds = [];
        let bobRegTxIds = [];
        let dapIds = [];
        const seeds = [{ip: '52.39.47.232'}];

        it('sendRawTransaction', async function it() {
            this.timeout(timeoutTest);
            let results = [];
            for (var i = 0; i < numLoops; i += 1) {
                const inputs = await dapiClient.getUTXO(faucetAddress);
                const queries = new Array(1);
                const bobUserName = Math.random().toString(36).substring(7);
                const bobPrivateKey = new PrivateKey();
                for (let index = 0; index < 1; ++index) {

                    const validPayload = new Transaction.Payload.SubTxRegisterPayload()
                        .setUserName(bobUserName)
                        .setPubKeyIdFromPrivateKey(bobPrivateKey).sign(bobPrivateKey);

                    const transaction = Transaction()
                        .setType(Transaction.TYPES.TRANSACTION_SUBTX_REGISTER)
                        .setExtraPayload(validPayload)
                        .from(inputs.slice(-1)[0])
                        .addFundingOutput(10000)
                        .change(faucetAddress)
                        .sign(faucetPrivateKey);

                    queries[index] = dapiClient.sendRawTransaction(transaction.serialize());

                }
                await runPromise(queries).then(function (result) {
                    results.push(result.time);
                    bobPrivateKeys.push(bobPrivateKey);
                    bobUserNames.push(bobUserName);
                    bobRegTxIds = bobRegTxIds.concat(result.result.map(function (item) {
                        return item.txid
                    }));
                }, function (failure) {
                    expect(failure, 'Errors found').to.be.undefined;
                });
            }
            const result = average(results);
            expect(spy.callCount).to.be.equal(numLoops * 2);
            expect(result).to.not.be.NaN;
            expect(result).to.be.a('number');
            console.log("average:", result);

        });

        it('estimateFee', async function it() {
            this.timeout(timeoutTest);
            let results = [];
            for (var i = 0; i < numLoops; i += 1) {
                const queries = new Array(numRequests);
                for (let index = 0; index < numRequests; ++index) {
                    queries[index] = dapiClient.estimateFee(Math.floor(Math.random() * 100) + 1);

                }
                await runPromise(queries).then(function (result) {
                    results.push(result.time);
                }, function (failure) {
                    expect(failure, 'Errors found').to.be.undefined;
                });
            }
            const result = average(results);
            expect(spy.callCount).to.be.equal(numLoops * numRequests);
            expect(result).to.not.be.NaN;
            expect(result).to.be.a('number');
            console.log("average:", result);
        });

        it('getUserByName', async function it() {
            this.timeout(timeoutTest);
            let results = [];
            for (var i = 0; i < numLoops; i += 1) {
                const queries = new Array(numRequests);
                for (let index = 0; index < numRequests; ++index) {
                    queries[index] = dapiClient.getUserByName(bobUserNames[Math.floor(Math.random() * bobUserNames.length)]);

                }
                await runPromise(queries).then(function (result) {
                    results.push(result.time);
                    bobUserIds = bobUserIds.concat(result.result.map(function (item) {
                        return item.regtxid
                    }));
                }, function (failure) {
                    expect(failure, 'Errors found').to.be.undefined;
                });
            }
            const result = average(results);
            expect(spy.callCount).to.be.equal(numLoops * numRequests);
            expect(result).to.not.be.NaN;
            expect(result).to.be.a('number');
            console.log("average:", result);
        });

        it('getUserById', async function it() {
            this.timeout(timeoutTest);
            let results = [];
            for (var i = 0; i < numLoops; i += 1) {
                const queries = new Array(numRequests);
                for (let index = 0; index < numRequests; ++index) {
                    queries[index] = dapiClient.getUserById(bobUserIds[Math.floor(Math.random() * bobUserIds.length)]);

                }
                await runPromise(queries).then(function (result) {
                    results.push(result.time);
                }, function (failure) {
                    expect(failure, 'Errors found').to.be.undefined;
                });
            }
            const result = average(results);
            expect(spy.callCount).to.be.equal(numLoops * numRequests);
            expect(result).to.not.be.NaN;
            expect(result).to.be.a('number');
            console.log("average:", result);
        });


        it('sendRawTransition', async function it() {
            this.timeout(timeoutTest);
            let results = [];
            for (var i = 0; i < numLoops; i += 1) {
                let dapSchema = Object.assign({}, DashPay);
                dapSchema.title = `TestContacts_${bobUserNames[i]}`;

                const dapContract = Schema.create.dapcontract(dapSchema);
                const queries = new Array(1);
                const bobPrivateKey = new PrivateKey(); // TODO?
                for (let index = 0; index < 1; ++index) {

                    let {stpacket: stPacket} = Schema.create.stpacket();
                    stPacket = Object.assign(stPacket, dapContract);

                    // 2. Create State Transition
                    const transaction = new Transaction()
                        .setType(Transaction.TYPES.TRANSACTION_SUBTX_TRANSITION);

                    const serializedPacket = Schema.serialize.encode(stPacket);
                    const stPacketHash = doubleSha256(serializedPacket);

                    transaction.extraPayload
                        .setRegTxId(bobRegTxIds[i])
                        .setHashPrevSubTx(bobRegTxIds[i])
                        .setHashSTPacket(stPacketHash)
                        .setCreditFee(1000)
                        .sign(bobPrivateKeys[i]);

                    queries[index] = await dapiClient.sendRawTransition(
                        transaction.serialize(),
                        serializedPacket.toString('hex'),
                    );
                }
                await runPromise(queries).then(function (result) {
                    results.push(result.time);
                    dapIds.push(doubleSha256(Schema.serialize.encode(dapContract.dapcontract)));
                }, function (failure) {
                    expect(failure, 'Errors found').to.be.undefined;
                });
            }
            const result = average(results);
            expect(spy.callCount).to.be.equal(numLoops);
            expect(result).to.not.be.NaN;
            expect(result).to.be.a('number');
            console.log("average:", result);
        });

        it('fetchDapContract', async function it() {
            // https://dashpay.atlassian.net/browse/DD-493
            this.timeout(timeoutTest * 2);

            for (let i = 0; i <= 240; i++) {
                try {
                    // waiting for Contacts to be added
                    await dapiClient.fetchDapContract(dapIds[0]);
                    break;
                } catch (e) {
                    await wait(1000);
                }
            }
            spy.resetHistory();
            let results = [];
            for (var i = 0; i < numLoops; i += 1) {
                const queries = new Array(numRequests);
                for (let index = 0; index < numRequests; ++index) {
                    queries[index] = dapiClient.fetchDapContract(dapIds[Math.floor(Math.random() * dapIds.length)]);

                }
                await runPromise(queries).then(function (result) {
                    results.push(result.time);
                }, function (failure) {
                    expect(failure, 'Errors found').to.be.undefined;
                });
            }
            const result = average(results);
            expect(spy.callCount).to.be.equal(numLoops * numRequests);
            expect(result).to.not.be.NaN;
            expect(result).to.be.a('number');
            console.log("average:", result);

        });

        it('fetchDapObjects', async function it() {
            this.timeout(timeoutTest);
            let results = [];
            for (var i = 0; i < numLoops; i += 1) {
                const queries = new Array(numRequests);
                for (let index = 0; index < numRequests; ++index) {
                    queries[index] = dapiClient.fetchDapObjects(dapIds[Math.floor(Math.random() * dapIds.length)], 'user', {});
                }
                await runPromise(queries).then(function (result) {
                    results.push(result.time);
                }, function (failure) {
                    expect(failure, 'Errors found').to.be.undefined;
                });
            }
            const result = average(results);
            expect(spy.callCount).to.be.equal(numLoops * numRequests);
            expect(result).to.not.be.NaN;
            expect(result).to.be.a('number');
            console.log("average:", result);
        });

    });
});
