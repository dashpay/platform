const axios = require('axios');
const moment = require('moment');
const Mnemonic = require('./mnemonic');
const Bitcoin = require('bitcoinjs-lib');

exports.BWS = function(){
let self = this;
    return {
        BWS:{
          getFeeLevels: function() {
              // let self = this;
              return async function(network, cb){
                  return new Promise(async function (resolve, reject) {
                      let getInsightCandidate = await self.Discover.getInsightCandidate();
                      let getInsightURI = getInsightCandidate.URI;
                      let now = moment().format("YYYY-MM-DD")
                      let lastblock =  await axios.get(`${getInsightURI}/blocks?limit=5&blockDate=${now}`).then(resp=>resp.data.blocks[0].height)
                      let url = `${getInsightURI}/utils/estimatefee?nbBlocks=${lastblock||2}`
                      return axios
                        .get(url)
                        .then(function(response){
                          // console.log(url, response.data)
                          return resolve(cb(null, response.data[lastblock]));
                        })
                        .catch(function(error){
                          if(error){
                              console.log(url, error)
                              console.error(`An error was triggered getting fee estimates `);
                              return cb(false);
                          }
                      });
                  });
              }
          },
          getUtxos: function() {
              // let self = this;
              return async function(cb,opts,addresses){
                  return new Promise(async function (resolve, reject) {
                    let getInsightCandidate = await self.Discover.getInsightCandidate();
                    let getInsightURI = getInsightCandidate.URI;
                    let url = `${getInsightURI}/addr`
                    let promises = [];


                    addresses.forEach(addr => {
                      promises.push(axios.get(`${url}/${addr}/utxo`))
                    });

                    axios.all(promises)
                    .then(res => {
                      // console.log(49, res)
                      return resolve(cb(null, res[1].data))});
                      });
                  }
            },
            getTx: function() {
                // let self = this;
                return async function(txid, cb){
                    return new Promise(async function (resolve, reject) {
                      let getInsightCandidate = await self.Discover.getInsightCandidate();
                      let getInsightURI = getInsightCandidate.URI;
                      let url = `${getInsightURI}/tx`

                      return axios
                        .get(`${url}/${txid}`)
                        .then(function(response){
                          // console.log(`${url}/${txid}`, response.data)
                          return resolve(cb(null, response.data));
                        })
                        .catch(function(error){
                          if(error){
                              console.log(url, error)
                              console.error(`An error was triggered getting tx {txid} `);
                              return cb(false);
                          }
                      });
                    })
                }
            },
            getBalance: function() {
                // let self = this;
                return async function(twoStep, cb, addy){
                    return new Promise(async function (resolve, reject) {
                      let res =  await SDK.Explorer.API.getBalance(addy || 'yj6xVHMyZGBdLqGUfoGc9gDvU8tHx6iqb4')
                      //how do you know the address to use? prob stored on the opt object that is global? use placehodler for now
                      return resolve(cb(null, res))}
                      );
                    }
              },
              broadcastRawTx: function() {
                  // let self = this;
                  return async function(opts, network, rawTx, cb){
                      return new Promise(async function (resolve, reject) {
                        let res = await SDK.Explorer.API.send(rawTx)
                        return resolve(cb(null, res))}
                        );
                    }
                },
              getFiatRate: function() {
                  // let self = this;
                  return async function(opts, ccyCode, ts, provider, cb){
                      return new Promise(async function (resolve, reject) {
                        let res = {ts: Date.now()-3000, rate: 120, fetchedOn: Date.now()}
                        return resolve(cb(null, res))
                          }
                        );
                    }
                },
              getMainAddress: function() {
                  return async function(opts, noVerify, limit, reverse, cb, rootKey, _mnemonic, _seed){
                      return new Promise(async function (resolve, reject) {
                        console.log(113, Mnemonic, Mnemonic.generateSeedFromMnemonic )
                        let bip32Seed = _seed ? _seed : Mnemonic.generateSeedFromMnemonic(_mnemonic);
                        console.log(9, 'bip32Seed',  bip32Seed)
                        let dashTestnet = {
                            messagePrefix: '\x19DarkCoin Signed Message:\n',
                            bip32: {public:70615939, private: 70615956},//Default was 50221816, but Copay use this one.
                            pubKeyHash: 140,//140=y (139=y||x)
                            scriptHash: 19,
                            wif: 239,
                            dustThreshold: 5460 // https://github.com/dashpay/dash/blob/v0.12.0.x/src/primitives/transaction.h#L144-L155
                        };
                        let dash = {
                            messagePrefix: '\u0019DarkCoin Signed Message:\n',
                            bip32: {public: 50221816, private: 50221772},
                            pubKeyHash: 76,
                            scriptHash: 16,
                            wif: 204,
                            dustThreshold: 5460
                        };
                        let bip32HDNode = Bitcoin.HDNode.fromSeedHex(bip32Seed, dashTestnet);

                        let bip32RootKey = bip32HDNode.toBase58();

                        let bip32RootAddress = bip32HDNode.getAddress();

                        let pathDashTestnet = "m/44'/1'/0'/0/";
                        let pathDashLivenet = "m/44'/5'/0'/0/";
                        let rt = [bip32HDNode.derivePath(pathDashTestnet+0).getAddress()];

                        let done = false;
                        for (i=2; limit ? (i < limit) : true; i=i+20){

                          for (j = i; j < i+20; j++){
                            let addy = bip32HDNode.derivePath(pathDashTestnet+j).getAddress();
                            let bal =  await SDK.Explorer.API.getBalance(addy);
                            if (!(bal > 0)) {
                              done = true;
                              break;
                            }
                            rt.push(addy);
                            }
                            if (done) {
                            break
                          }
                        }
                        return resolve(cb(null, rt))
                        }
                        );
                    }
                },
              getTxHistory: function() {
                  // let self = this;
                  return async function(opts, skip=0, limit=0, includeExtendedInfo, cb){
                        return new Promise(async function (resolve, reject) {
                          let getInsightCandidate = await self.Discover.getInsightCandidate();
                          let getInsightURI = getInsightCandidate.URI;
                          let url = `${getInsightURI}/addr/yj6xVHMyZGBdLqGUfoGc9gDvU8tHx6iqb4?from=${skip}&to=${limit}`

                          let promises = [];

                          return axios
                            .get(url)
                            .then(function(response){
                              return resolve(includeExtendedInfo ?
                                (()=> {response.data.transactions.forEach(
                                txId => {
                                  promises.push(axios.get(`${getInsightURI}/tx/${txId}`));
                                        })
                                  return axios.all(promises)
                                  .then(res => {
                                    const ans = res.map(r=>r.data)
                                    // console.log(118, ans)
                                    cb(null, ans)})})() :
                                 cb(null, response.data.transactions)
                              )
                            })
                            .catch(function(error){
                              if(error){
                                  console.log(url, error)
                                  console.error(`An error was triggered getting getTxHistory`);
                                  return cb(false);
                              }
                          });
                            });
                        };
                    }
              },
        }
      };


// API.getFeeLevels()('live',(err, x)=>{console.log('res', x)})
// API.getUtxos()((err, x)=>{console.log('res!!!', x)},'nada',['XfmtHzRb8TLGpE3z3bV9iMXr7N8UbNsLfk', 'Xmghk9LmasjpKbg6bBfFDMQwMapjbC33kU'])
// API.getTx()('02e7146fed1eeca237a0304d0d4252314773cc08273a37624bf4928275ccdd28',(err, x)=>{console.log('res', x)} )
