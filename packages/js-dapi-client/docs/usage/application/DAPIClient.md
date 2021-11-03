**Usage**: `new DAPIClient(options)`  
**Description**: This method creates a new DAPIClient instance.

Parameters:

| parameters                                | type                | required[def value]         | Description                                                                                                                                                                    |
|-------------------------------------------|---------------------|-----------------------------| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **options**                               | Object              |  |   |
| **options.dapiAddressProvider**           | DAPIAddressProvider | no[ListDAPIAddressProvider] | Allow to override the default dapiAddressProvider (do not allow seeds or dapiAddresses params)  |
| **options.seeds**                         | string[]            | no[seeds]                   | Allow to override default seeds (to connect to specific node) |
| **options.network**                       | string|Network      | no[=evonet]                 | Allow to setup the network to be used (livenet, testnet, evonet,..) |
| **options.timeout**                       | number              | no[=2000]                   | Used to specify the timeout time in milliseconds. |
| **options.retries**                       | number              | no[=3]                      | Used to specify the number of retries before aborting and erroring a request. |
| **options.baseBanTime**                   | number              | no[=6000]                   |  |

Returns : DAPIClient instance.

```js
const DAPIClient = require('@dashevo/dapi-client');
const client = new DAPIClient({
  timeout: 5000,
  retries: 3,
  network: 'livenet'
});
```

**Notes**: 
- Accessing the SimplifiedMasternodeListDAPIAddressProvider (or its overwrote instance), can be accessed via `client.dapiAddressProvider`.  
 
