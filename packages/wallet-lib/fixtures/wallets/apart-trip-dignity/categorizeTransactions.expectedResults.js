const transactionsWithMetadataFixtures = require("./transactions-with-metadata.json");
const expectedResultTx1 = {
  from: [
    {address: 'ygrRyPRf9vSHnP1ieoRRvY9THtFbTMc66e'},
    {address: 'yhDaDMNRUAB93S2ZcprNLuEGHPG4VT8kYL'},
    {address: 'ygZ5fgrtGQDtwsN8K7sftSNPXN4Srhz99s'},
    {address: 'yb39TanhfUKeqaBtzqDvAE3ad9UsDuj3Fd'},
    {address: 'yToX9gDE6tn2Sv1zhq88WNfJSomeHee3rR'},
    {address: 'yViAv63brJ5kB7Gyc7yX2c7rJ9NuykCzRh'},
    {address: 'yfnJMvdE32izNQP68PhMPiHAeJKYo2PBdH'}
  ],
  to: [
    {
      address: 'ySE2UYPf7PWMJ5oYikSscVifzQEoGiGRmd',
      satoshis: 1823313
    },
    {
      address: 'yTwEca67QSkZ6axGdpNFzWPaCj8zqYybY7',
      satoshis: 187980000
    }
  ],
  transaction: transactionsWithMetadataFixtures[2][0],
  type: 'received',
  blockHash: '000001deee9f99e8219a9abcaaea135dbaae8a9b0f1ea214e6b6a37a5c5b115d',
  height: 555506,
  isInstantLocked: true,
  isChainLocked: true
}
const expectedResultTx2 = {
  from: [{address: 'yaLhoAZ4iex2zKmfvS9rvEmxXmRiPrjHdD'}],
  to: [
    {
      address: 'yercyhdN9oEkZcB9BsW5ktFaDxFEuK6qXN',
      satoshis: 10000000
    },
    {
      address: 'yTcjWB7v7opDzpfYKpFdFEtEvSKFsh3bW3',
      satoshis: 532649506
    }
  ],

  transaction: transactionsWithMetadataFixtures[4][0],
  type: 'received',
  blockHash: '000000b6006c758eda23ec7e2a640a0bf2c6a0c44827be216faff6bf4fd388e8',
  height: 555507,
  isInstantLocked: true,
  isChainLocked: true
}
const expectedResultTx3 = {
  from: [ { address: 'yTcjWB7v7opDzpfYKpFdFEtEvSKFsh3bW3' } ],
  to: [
    {
      address: 'ygk3GCSba2J3L9G665Snozhj9HSkh5ByVE',
      satoshis: 10000000
    },
    {
      address: 'yiDVYtUZ2mKV4teSJzKBArqY4BRsZoFLYs',
      satoshis: 522649259
    }
  ],
  transaction: transactionsWithMetadataFixtures[5][0],
  type: 'received',
  blockHash: '0000012cf6377c6cf2b317a4deed46573c09f04f6880dca731cc9ccea6691e19',
  height: 555508,
  isInstantLocked: true,
  isChainLocked: true
}
const expectedResultTx4 = {
  from: [
    { address: 'yXxUiAnB31voBDPqnwxkffcPnUvwJz6a2k' },
    { address: 'yNh6Xzw4rs1kenAo8VWCswdyUnkdYXDZsg' }
  ],
  to: [
    {
      address: 'yXiTNo71QQAqiw2u1i6vkEEj3m6y4sEGae',
      satoshis: 1768694
    },
    {
      address: 'yMLhEsiP2ajSh8STmXnNmkWXtoHsmawZxd',
      satoshis: 840010000
    }
  ],
  transaction: transactionsWithMetadataFixtures[1][0],
  type: 'received',
  blockHash: '00000221952c2a60adcb929de837f659308cb5c6bb7783016479381fb550fbad',
  height: 557481,
  isInstantLocked: true,
  isChainLocked: true

}
const expectedResultTx5 = {
  from: [{address: 'yP8A3cbdxRtLRduy5mXDsBnJtMzHWs6ZXr'}],
  to: [
    {
      address: 'yY16qMW4TSiYGWUyANYWMSwgwGe36KUQsR',
      satoshis: 46810176
    },
    {
      address: 'ygHAVkMtYSqoTWHebDv7qkhMV6dHyuRsp2',
      satoshis: 729210000
    }
  ],
  transaction: transactionsWithMetadataFixtures[0][0],
  type: 'received',
  blockHash: '00000c1e4556add15119392ed36ec6af2640569409abfa23a9972bc3be1b3717',
  height: 558036,
  isInstantLocked: true,
  isChainLocked: true
};
const expectedResultTx6 = {
  from: [
    { address: 'ygHAVkMtYSqoTWHebDv7qkhMV6dHyuRsp2' },
    { address: 'ygk3GCSba2J3L9G665Snozhj9HSkh5ByVE' },
    { address: 'yTwEca67QSkZ6axGdpNFzWPaCj8zqYybY7' },
    { address: 'yercyhdN9oEkZcB9BsW5ktFaDxFEuK6qXN' },
    { address: 'yMLhEsiP2ajSh8STmXnNmkWXtoHsmawZxd' }
  ],
  to: [
    {
      address: 'yj8rRKATAUHcAgXvNZekob58xKm2oNyvhv',
      satoshis: 1777100000
    },
    {
      address: 'yNDpPsJqXKM36zHSNEW7c1zSvNnrZ699FY',
      satoshis: 99170
    }
  ],
  transaction: transactionsWithMetadataFixtures[3][0],
  type: 'address_transfer',
  blockHash: '00000084b4d9e887a6ad3f37c576a17d79c35ec9301e55210eded519e8cdcd3a',
  height: 558102,
  isInstantLocked: true,
  isChainLocked: true
};
const expectedResultTx7 = {
  from: [ { address: 'yj8rRKATAUHcAgXvNZekob58xKm2oNyvhv' } ],
  to: [
    {
      address: 'yj8rRKATAUHcAgXvNZekob58xKm2oNyvhv',
      satoshis: 1270000000
    },
    {
      address: 'yhaAB6e8m3F8zmGX7WAVYa6eEfmSrrnY8x',
      satoshis: 400000000
    },
    {
      address: 'yLk4Hw3w4zDudrDVP6W8J9TggkY57zQUki',
      satoshis: 107099720
    }
  ],
  transaction: transactionsWithMetadataFixtures[6][0],
  type: 'address_transfer',
  blockHash: '000001953ea0bbb8ad04a9a1a2a707fef207ad22a712d7d3c619f0f9b63fa98c',
  height: 558229,
  isInstantLocked: true,
  isChainLocked: true
};
const expectedResultTx8 = {
  from: [ { address: 'yj8rRKATAUHcAgXvNZekob58xKm2oNyvhv' } ],
  to: [
    {
      address: 'yYJmzWey5kNecAThet5BFxAga1F4b4DKQ2',
      satoshis: 1260000000
    },
    {
      address: 'yirJaK8KCE5YAmwvLadizqFw3TCXqBuZXL',
      satoshis: 9999753
    }
  ],
  transaction: transactionsWithMetadataFixtures[7][0],
  type: 'account_transfer',
  blockHash: '000000dffb05c071a8c05082a475b7ce9c1e403f3b89895a6c448fe08535a5f5',
  height: 558230,
  isInstantLocked: true,
  isChainLocked: true
};
const expectedResultTx9 = {
  from: [ { address: 'yYJmzWey5kNecAThet5BFxAga1F4b4DKQ2' } ],
  to: [
    {
      address: 'yNCqctyQaq51WU1hN5aNwsgMsZ5fRiB7GY',
      satoshis: 1200000000
    },
    {
      address: 'yXMrw79LPgu78EJsfGGYpm6fXKc1EMnQ49',
      satoshis: 59999753
    }
  ],
  transaction: transactionsWithMetadataFixtures[10][0],
  type: 'unknown',
  blockHash: '0000016fb685b4b1efed743d2263de34a9f8323ed75e732654b1b951c5cb4dde',
  height: 558236,
  isInstantLocked: true,
  isChainLocked: true
};
const expectedResultTx10 = {
  from: [ { address: 'yNCqctyQaq51WU1hN5aNwsgMsZ5fRiB7GY' } ],
  to: [
    {
      address: 'yiXh4Yo5djG6QH8WzXkKm5EFzqLRJWakXz',
      satoshis: 1150000000
    },
    {
      address: 'yh6Hcyipdvp6WJpQxjNbaXP4kzPQUJpY3n',
      satoshis: 49999753
    }
  ],
  transaction: transactionsWithMetadataFixtures[8][0],
  type: 'account_transfer',
  blockHash: '000000444b3f2f02085f8befe72da5442c865c290658766cf935e1a71a4f4ba7',
  height: 558242,
  isInstantLocked: true,
  isChainLocked: true
};

const expectedResultTx11 = {
  from: [
    { address: 'yirJaK8KCE5YAmwvLadizqFw3TCXqBuZXL' },
    { address: 'yiXh4Yo5djG6QH8WzXkKm5EFzqLRJWakXz' }
  ],
  to: [
    {
      address: 'yMX3ycrLVF2k6YxWQbMoYgs39aeTfY4wrB',
      satoshis: 1000000000
    },
    {
      address: 'yhdRfg5gNr587dtEC4YYMcSHmLVEGqqtHc',
      satoshis: 159999359
    }
  ],
  transaction: transactionsWithMetadataFixtures[9][0],
  type: 'sent',
  blockHash: '000001f9c5de4d2b258a975bfbf7b9a3346890af6389512bea3cb6926b9be330',
  height: 558246,
  isInstantLocked: true,
  isChainLocked: true
};

module.exports = [
  expectedResultTx1,
  expectedResultTx2,
  expectedResultTx3,
  expectedResultTx4,
  expectedResultTx5,
  expectedResultTx6,
  expectedResultTx7,
  expectedResultTx8,
  expectedResultTx9,
  expectedResultTx10,
  expectedResultTx11,
]
