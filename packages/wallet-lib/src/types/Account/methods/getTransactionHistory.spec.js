const {expect} = require('chai');
const getTransactions = require('./getTransactions');
const getTransactionHistory = require('./getTransactionHistory');
const getTotalBalance = require('./getTotalBalance');

const getFixtureHDAccountWithStorage = require('../../../../fixtures/wallets/apart-trip-dignity/getFixtureAccountWithStorage');
const getFixturePrivateAccountWithStorage = require('../../../../fixtures/wallets/2a331817b9d6bf85100ef0/getFixtureAccountWithStorage');


const mockedHDAccount = getFixtureHDAccountWithStorage();
mockedHDAccount.getTransactions = getTransactions;

const mockedPKAccount = getFixturePrivateAccountWithStorage();
mockedPKAccount.getTransactions = getTransactions;

describe('Account - getTransactionHistory', () => {
  it('should return empty array on no transaction history', async function () {
    const mockedHDSelf = {
      ...getFixtureHDAccountWithStorage()
    }
    mockedHDSelf.getTransactions = getTransactions;
    const chainStore = mockedHDSelf.storage.getChainStore('testnet')
    chainStore.state.blockHeaders = new Map();
    chainStore.state.transactions = new Map();
    chainStore.state.addresses.forEach((address)=>{
      address.transactions = [];
      address.utxos = {};
      address.balanceSat = 0;
    })
    const transactionHistoryHD = await getTransactionHistory.call(mockedHDSelf);
    const balanceImpact = transactionHistoryHD.reduce((acc, item) => {
      return acc + item.satoshisBalanceImpact - item.feeImpact
    },0);

    const balance = getTotalBalance.call(mockedHDSelf);
    expect(balance).to.equal(balanceImpact)

    const expectedTransactionHistoryHD = [];
    expect(transactionHistoryHD).to.deep.equal(expectedTransactionHistoryHD);
  });

  it('should return valid transaction for HDWallet', async function () {
    const mockedHDSelf = {
      ...mockedHDAccount
    }
    const timestartTs = +new Date();
    const transactionHistoryHD = await getTransactionHistory.call(mockedHDSelf);
    const timeendTs = +new Date();
    const calculationTime = timeendTs - timestartTs;

    const balance = getTotalBalance.call(mockedHDSelf);

    const balanceImpact = transactionHistoryHD.reduce((acc, item) => {
      return acc + item.satoshisBalanceImpact - item.feeImpact
    },0);

    expect(balance).to.equal(balanceImpact)

    expect(calculationTime).to.be.below(60 * 1000);

    const expectedTransactionHistoryHD = [
      {
        from: [
          {
            address: 'yirJaK8KCE5YAmwvLadizqFw3TCXqBuZXL',
            addressType: 'internal'
          },
          {
            address: 'yiXh4Yo5djG6QH8WzXkKm5EFzqLRJWakXz',
            addressType: 'external'
          }
        ],
        to: [
          {
            address: 'yMX3ycrLVF2k6YxWQbMoYgs39aeTfY4wrB',
            satoshis: 1000000000,
            addressType: 'unknown'
          },
          {
            address: 'yhdRfg5gNr587dtEC4YYMcSHmLVEGqqtHc',
            satoshis: 159999359,
            addressType: 'internal'
          }
        ],
        type: 'sent',
        time: new Date(1629237076*1e3),
        txId: 'e6b6f85a18d77974f376f05d6c96d0fdde990e733664248b1a00391565af6841',
        blockHash: '000001f9c5de4d2b258a975bfbf7b9a3346890af6389512bea3cb6926b9be330',
        isChainLocked: true,
        isInstantLocked: true,
        satoshisBalanceImpact:-1000000000,
        feeImpact: 394
      },
      {
        from: [{address: 'yNCqctyQaq51WU1hN5aNwsgMsZ5fRiB7GY', addressType: 'otherAccount'}],
        to: [
          {
            address: 'yiXh4Yo5djG6QH8WzXkKm5EFzqLRJWakXz',
            satoshis: 1150000000,
            addressType: 'external'
          },
          {
            address: 'yh6Hcyipdvp6WJpQxjNbaXP4kzPQUJpY3n',
            satoshis: 49999753,
            addressType: 'otherAccount'
          }
        ],
        type: 'account_transfer',
        time: new Date(1629236158*1e3),
        txId: '6f76ca8038c6cb1b373bbbf80698afdc0d638e4a223be12a4feb5fd8e1801135',
        blockHash: '000000444b3f2f02085f8befe72da5442c865c290658766cf935e1a71a4f4ba7',
        isChainLocked: true,
        isInstantLocked: true,
        satoshisBalanceImpact: 1150000000,
        feeImpact: 0
      },
      {
        from: [{
          address: 'yj8rRKATAUHcAgXvNZekob58xKm2oNyvhv',
          addressType: 'external'
        }],
        to: [
          {
            address: 'yYJmzWey5kNecAThet5BFxAga1F4b4DKQ2',
            satoshis: 1260000000,
            addressType: 'otherAccount'
          },
          {
            address: 'yirJaK8KCE5YAmwvLadizqFw3TCXqBuZXL',
            satoshis: 9999753,
            addressType: 'internal'
          }
        ],
        type: 'account_transfer',
        time: new Date(1629234873*1e3),
        txId: '6f37b0d6284aab627c31c50e1c9d7cce39912dd4f2393f91734f794bc6408533',
        blockHash: '000000dffb05c071a8c05082a475b7ce9c1e403f3b89895a6c448fe08535a5f5',
        isChainLocked: true,
        isInstantLocked: true,
        satoshisBalanceImpact: -1260000000,
        feeImpact: 247
      },
      {
        from: [{
          address: 'yj8rRKATAUHcAgXvNZekob58xKm2oNyvhv',
          addressType: 'external'
        }],
        to: [
          {
            address: 'yj8rRKATAUHcAgXvNZekob58xKm2oNyvhv',
            satoshis: 1270000000,
            addressType: 'external'
          },
          {
            address: 'yhaAB6e8m3F8zmGX7WAVYa6eEfmSrrnY8x',
            satoshis: 400000000,
            addressType: 'external'
          },
          {
            address: 'yLk4Hw3w4zDudrDVP6W8J9TggkY57zQUki',
            satoshis: 107099720,
            addressType: 'internal'
          }
        ],
        type: 'address_transfer',
        time: new Date(1629234474*1e3),
        txId: 'c3fb3620ebd1c7678879b40df1495cc86a179b5a6f9e48ce0b687a5c6f5a1db5',
        blockHash: '000001953ea0bbb8ad04a9a1a2a707fef207ad22a712d7d3c619f0f9b63fa98c',
        isChainLocked: true,
        isInstantLocked: true,
        satoshisBalanceImpact: 0,
        feeImpact: 280
      },

      {
        from: [
          {
            address: 'ygHAVkMtYSqoTWHebDv7qkhMV6dHyuRsp2',
            addressType: 'external'
          },
          {
            address: 'ygk3GCSba2J3L9G665Snozhj9HSkh5ByVE',
            addressType: 'external'
          },
          {
            address: 'yTwEca67QSkZ6axGdpNFzWPaCj8zqYybY7',
            addressType: 'external'
          },
          {
            address: 'yercyhdN9oEkZcB9BsW5ktFaDxFEuK6qXN',
            addressType: 'external'
          },
          {
            address: 'yMLhEsiP2ajSh8STmXnNmkWXtoHsmawZxd',
            addressType: 'external'
          }
        ],
        to: [
          {
            address: 'yj8rRKATAUHcAgXvNZekob58xKm2oNyvhv',
            satoshis: 1777100000,
            addressType: 'external'
          },
          {
            address: 'yNDpPsJqXKM36zHSNEW7c1zSvNnrZ699FY',
            satoshis: 99170,
            addressType: 'internal'
          }
        ],
        type: 'address_transfer',
        time: new Date(1629216608*1e3),
        txId: 'f230a9414bf577d93d6f7f2515d9b549ede78cfba4168920892970fa8aa1eef8',
        blockHash: '00000084b4d9e887a6ad3f37c576a17d79c35ec9301e55210eded519e8cdcd3a',
        isChainLocked: true,
        isInstantLocked: true,
        satoshisBalanceImpact: 0,
        feeImpact: 830
      },
      {
        from: [{
          address: 'yP8A3cbdxRtLRduy5mXDsBnJtMzHWs6ZXr',
          addressType: "unknown"
        }],
        to: [
          {
            address: 'yY16qMW4TSiYGWUyANYWMSwgwGe36KUQsR',
            satoshis: 46810176,
            addressType: "unknown"
          },
          {
            address: 'ygHAVkMtYSqoTWHebDv7qkhMV6dHyuRsp2',
            satoshis: 729210000,
            addressType: "external"
          }
        ],
        type: 'received',
        time: new Date(1629207543*1e3),
        txId: '1cbb35edc105918b956838570f122d6f3a1fba2b67467e643e901d09f5f8ac1b',
        blockHash: '00000c1e4556add15119392ed36ec6af2640569409abfa23a9972bc3be1b3717',
        isChainLocked: true,
        isInstantLocked: true,
        satoshisBalanceImpact: 729210000,
        feeImpact: 0
      },
      {
        from: [{
          address: 'yXxUiAnB31voBDPqnwxkffcPnUvwJz6a2k',
          addressType: "unknown"
        },
        {
          address: 'yNh6Xzw4rs1kenAo8VWCswdyUnkdYXDZsg',
          addressType: "unknown"
        }],
        to: [
          {
            "address": "yXiTNo71QQAqiw2u1i6vkEEj3m6y4sEGae",
            "satoshis": 1768694,
            addressType: "unknown"
          },
          {
            "address": "yMLhEsiP2ajSh8STmXnNmkWXtoHsmawZxd",
            "satoshis": 840010000,
            addressType: "external"
          }
        ],
        time: new Date(1629126597*1e3),
        txId: "eb1a7fc8e3b43d3021653b1176f8f9b41e9667d05b65ee225d14c149a5b14f77",
        blockHash: "00000221952c2a60adcb929de837f659308cb5c6bb7783016479381fb550fbad",
        type: "received",
        isChainLocked: true,
        isInstantLocked: true,
        satoshisBalanceImpact: 840010000,
        feeImpact: 0
      },
      {
        from: [{
          address: 'yTcjWB7v7opDzpfYKpFdFEtEvSKFsh3bW3',
          addressType: "unknown"
        }],
        to: [
          {
            "address": "ygk3GCSba2J3L9G665Snozhj9HSkh5ByVE",
            "satoshis": 10000000,
            addressType: "external"
          },
          {
            "address": "yiDVYtUZ2mKV4teSJzKBArqY4BRsZoFLYs",
            "satoshis": 522649259,
            addressType: "unknown"
          }
        ],
        time: new Date(1628846998*1e3),
        txId: "7d1b78157f9f2238669f260d95af03aeefc99577ff0cddb91b3e518ee557a2fd",
        blockHash: "0000012cf6377c6cf2b317a4deed46573c09f04f6880dca731cc9ccea6691e19",
        type: "received",
        isChainLocked: true,
        isInstantLocked: true,
        satoshisBalanceImpact: 10000000,
        feeImpact: 0
      },
      {
        from: [{
          address: 'yaLhoAZ4iex2zKmfvS9rvEmxXmRiPrjHdD',
          addressType: "unknown"
        }],
        to: [
          {
            "address": "yercyhdN9oEkZcB9BsW5ktFaDxFEuK6qXN",
            "satoshis": 10000000,
            addressType: "external"
          },
          {
            "address": "yTcjWB7v7opDzpfYKpFdFEtEvSKFsh3bW3",
            "satoshis": 532649506,
            addressType: "unknown"
          }
        ],
        type: 'received',
        time: new Date(1628846768*1e3),
        txId: 'd37b6c7dd449d605bea9997af8bbeed2f3fbbcb23a4068b1f1ad694db801912d',
        blockHash: '000000b6006c758eda23ec7e2a640a0bf2c6a0c44827be216faff6bf4fd388e8',
        isChainLocked: true,
        isInstantLocked: true,
        satoshisBalanceImpact: 10000000,
        feeImpact: 0
      },
      {
        from: [
          {
            address: 'ygrRyPRf9vSHnP1ieoRRvY9THtFbTMc66e',
            addressType: "unknown"
          },
          {
            address: 'yhDaDMNRUAB93S2ZcprNLuEGHPG4VT8kYL',
            addressType: "unknown"
          },
          {
            address: 'ygZ5fgrtGQDtwsN8K7sftSNPXN4Srhz99s',
            addressType: "unknown"
          },
          {
            address: 'yb39TanhfUKeqaBtzqDvAE3ad9UsDuj3Fd',
            addressType: "unknown"
          },
          {
            address: 'yToX9gDE6tn2Sv1zhq88WNfJSomeHee3rR',
            addressType: "unknown"
          },
          {
            address: 'yViAv63brJ5kB7Gyc7yX2c7rJ9NuykCzRh',
            addressType: "unknown"
          },
          {
            address: 'yfnJMvdE32izNQP68PhMPiHAeJKYo2PBdH',
            addressType: "unknown"
          },
        ],
        to: [
          {
            "address": "ySE2UYPf7PWMJ5oYikSscVifzQEoGiGRmd",
            "satoshis": 1823313,
            addressType: "unknown"
          },
          {
            "address": "yTwEca67QSkZ6axGdpNFzWPaCj8zqYybY7",
            "satoshis": 187980000,
            addressType: "external"
          }
        ],
        type: 'received',
        time: new Date(1628846677*1e3),
        txId: 'a43845e580ad01f31bc06ce47ab39674e40316c4c6b765b6e54d6d35777ef456',
        blockHash: '000001deee9f99e8219a9abcaaea135dbaae8a9b0f1ea214e6b6a37a5c5b115d',
        isChainLocked: true,
        isInstantLocked: true,
        satoshisBalanceImpact: 187980000,
        feeImpact: 0
      }
    ]
    expect(transactionHistoryHD).to.deep.equal(expectedTransactionHistoryHD);
  });
  it('should correctly deal with multiple HDWallet accounts', async function () {
    const mockedHDSelf = {
      ...mockedHDAccount
    }
    mockedHDSelf.index = 1;
    mockedHDSelf.accountPath = `m/44'/1'/1'`;
    const transactionHistoryHD = await getTransactionHistory.call(mockedHDSelf);

    const balance = getTotalBalance.call(mockedHDSelf);

    const balanceImpact = transactionHistoryHD.reduce((acc, item) => {
      return acc + item.satoshisBalanceImpact - item.feeImpact
    },0);

    expect(balance).to.equal(balanceImpact)

    const expectedTransactionHistoryHD = [
      {
        from: [
          {
            address: 'yYJmzWey5kNecAThet5BFxAga1F4b4DKQ2',
            addressType: 'external',
          },
        ],
        to: [
          {
            address: 'yNCqctyQaq51WU1hN5aNwsgMsZ5fRiB7GY',
            satoshis: 1200000000,
            addressType: 'external',
          },
          {
            address: 'yXMrw79LPgu78EJsfGGYpm6fXKc1EMnQ49',
            satoshis: 59999753,
            addressType: 'internal',
          },
        ],
        type: 'address_transfer',
        time: new Date(9999999999*1e3),
        txId: '9cd3d44a87a7f99a33aebc6957105d5fb41698ef642189a36bac59ec0b5cd840',
        blockHash: '0000016fb685b4b1efed743d2263de34a9f8323ed75e732654b1b951c5cb4dde',
        isChainLocked: true,
        isInstantLocked: true,
        satoshisBalanceImpact: 0,
        feeImpact: 247
      },
      {
        from: [ { address: 'yNCqctyQaq51WU1hN5aNwsgMsZ5fRiB7GY', addressType: 'external' } ],
        to: [
          {
            address: 'yiXh4Yo5djG6QH8WzXkKm5EFzqLRJWakXz',
            satoshis: 1150000000,
            addressType: 'otherAccount'
          },
          {
            address: 'yh6Hcyipdvp6WJpQxjNbaXP4kzPQUJpY3n',
            satoshis: 49999753,
            addressType: 'internal'
          }
        ],
        type: 'account_transfer',
        time: new Date(1629236158*1e3),
        txId: '6f76ca8038c6cb1b373bbbf80698afdc0d638e4a223be12a4feb5fd8e1801135',
        blockHash: '000000444b3f2f02085f8befe72da5442c865c290658766cf935e1a71a4f4ba7',
        isChainLocked: true,
        isInstantLocked: true,
        satoshisBalanceImpact: -1150000000,
        feeImpact: 247
      },
      {
        from: [ { address: 'yj8rRKATAUHcAgXvNZekob58xKm2oNyvhv', addressType: 'otherAccount' } ],
        to: [
          {
            address: 'yYJmzWey5kNecAThet5BFxAga1F4b4DKQ2',
            satoshis: 1260000000,
            addressType: 'external'
          },
          {
            address: 'yirJaK8KCE5YAmwvLadizqFw3TCXqBuZXL',
            satoshis: 9999753,
            addressType: 'otherAccount'
          }
        ],
        type: 'account_transfer',
        time: new Date(1629234873*1e3),
        txId: '6f37b0d6284aab627c31c50e1c9d7cce39912dd4f2393f91734f794bc6408533',
        blockHash: '000000dffb05c071a8c05082a475b7ce9c1e403f3b89895a6c448fe08535a5f5',
        isChainLocked: true,
        isInstantLocked: true,
        satoshisBalanceImpact: 1260000000,
        feeImpact: 0
      }
    ]
    expect(transactionHistoryHD).to.deep.equal(expectedTransactionHistoryHD);
  });
  it('should correctly compute transaction history for single address based wallet', async function (){
    const mockedPKSelf = {
      ...mockedPKAccount
    }

    const transactionHistoryPK = await getTransactionHistory.call(mockedPKSelf);

    const balanceImpact = transactionHistoryPK.reduce((acc, item) => {
      return acc + item.satoshisBalanceImpact - item.feeImpact
    },0);

    const balance = getTotalBalance.call(mockedPKSelf);
    expect(balance).to.equal(balanceImpact)

    const expectedTransactionHistoryPK = [
      {
        from: [ {
          address: 'ycDeuTfs4U77bTb5cq17dame28zdWHVYfk',
          addressType: 'external'
        } ],
        to: [
          {
            address: 'yP8A3cbdxRtLRduy5mXDsBnJtMzHWs6ZXr',
            satoshis: 450000,
            addressType: 'unknown'
          },
          {
            address: 'ycDeuTfs4U77bTb5cq17dame28zdWHVYfk',
            satoshis: 8999753,
            addressType: 'external'
          }
        ],
        type: 'sent',
        time: new Date(1629510092*1e3),
        txId: '47d13f7f713f4258953292c2298c1d91e2d6dee309d689f3c8b44ccf457bab52',
        blockHash: '0000007b7356e715b43ed7d5b7135fb9a2bf403e079bbcf7faec0f0da5c40117',
        isChainLocked: true,
        isInstantLocked: true,
        satoshisBalanceImpact: -450000,
        feeImpact: 247
      },
      {
        from: [ {
          address: 'ycDeuTfs4U77bTb5cq17dame28zdWHVYfk',
          addressType: 'external'
        } ],
        to: [
          {
            address: 'ycDeuTfs4U77bTb5cq17dame28zdWHVYfk',
            addressType: 'external',
            satoshis: 9450000
          },
          {
            address: 'ycDeuTfs4U77bTb5cq17dame28zdWHVYfk',
            addressType: 'external',
            satoshis: 699999753
          }
        ],
        type: 'address_transfer',
        time: new Date(1629509216*1e3),
        txId: 'd48f415f08fb795d43b216cf56e9ef10e059d4009cfc8fc90edfc0d3850813af',
        blockHash: '0000018b88fe43d07c3d63050aa82271698dc406dd08388529205dd837bf92dc',
        isChainLocked: true,
        isInstantLocked: true,
        satoshisBalanceImpact: 0,
        feeImpact: 247
      },
      {
        from: [
          {
            address: 'yXpVMRLKnH9e9Bdcd68e8iA3rxAerzwKop',
            addressType: 'unknown'
          },
          {
            address: 'yeryenDBwJbe7rqdL5uv7iLiJAWSU1iTe2',
            addressType: 'unknown'
          }
        ],
        to: [
          {
            address: 'yanVwuG1csehvH7PoWHxmYmjtojXBLnoYP',
            addressType: 'unknown',
            satoshis: 4840346
          },
          {
            address: 'ycDeuTfs4U77bTb5cq17dame28zdWHVYfk',
            satoshis: 709450000,
            addressType: 'external'
          }
        ],
        type: 'received',
        time: new Date(1629503698*1e3),
        txId: '0dcdaa9bf5b3596be1bcf22113e39026fd49d24b47190e2c7423be936cb116a7',
        blockHash: '000000299efeefa87dc15474fd0423c136798975b779a2bb8aa5bb2f50509afb',
        isChainLocked: true,
        isInstantLocked: true,
        satoshisBalanceImpact: 709450000,
        feeImpact: 0
      }
    ]

    expect(transactionHistoryPK).to.deep.equal(expectedTransactionHistoryPK);

  })
});
