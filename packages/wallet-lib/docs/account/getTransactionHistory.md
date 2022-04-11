**Usage**: `account.getTransactionHistory()`    
**Description**: Allow to get the transaction history of an account

Parameters:

| parameters        | type   | required       | Description                                      |  
|-------------------|--------|----------------| -------------------------------------------------|

Returns : sorted and classified transaction history

```js
const transactionHistory = await account.getTransactionHistory();
```

Results in

```js
[{
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
  time: Date('2021-08-17T21:35:58.000Z'),
  txId: '6f76ca8038c6cb1b373bbbf80698afdc0d638e4a223be12a4feb5fd8e1801135',
  blockHash: '000000444b3f2f02085f8befe72da5442c865c290658766cf935e1a71a4f4ba7',
  isChainLocked: true,
  isInstantLocked: true,
  satoshisBalanceImpact: -1150000000,
  feeImpact: 247
}] 
```

Where `addressType=external|internal|otherAccount|unknown`
