**Usage**: `account.getTransactionHistory()`    
**Description**: Allow to get the transaction history of an account

Parameters:   

| parameters        | type   | required       | Description                                      |  
|-------------------|--------|----------------| -------------------------------------------------|

Returns : sorted and classified transaction history   

```js
const transactionHistory = account.getTransactionHistory();

/**
 * [
    {
      from: [ { address: 'ySBsUTdfKSzqg5SHDFa6SZLRFzbbaBsrMX' } ],
      to: [{ 
          address: 'yUSaLsYdFxuuWxNJFEvoJNAcJk3KTotLpU',
          satoshis: 3999989000
        }
      ],
      type: 'sent',
      time: 1634325172,
      txId: '97932c88eda0423578f26d32a0a1ba21b5792721f345c135bc8eb2cb4864184c',
      blockHash: '000000f3a03c0c7c89dcd089f87edcb18bdd95051e85bc27e7de73666a071698',
      isChainLocked: true,
      isInstantLocked: true
    }
 ]
 */
```
