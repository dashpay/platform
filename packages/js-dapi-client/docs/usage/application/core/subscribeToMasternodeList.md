**Usage**: `await client.core.subscribeToMasternodeList(options = {})`\
**Description**: Returns a ClientReadableStream streaming of masternode list diffs ([DIP-4](https://github.com/dashpay/dips/blob/master/dip-0004.md)). As a first message it returns a diff from the first block to the current tip and a diff for each new chainlocked block. 

Returns : Promise<EventEmitter>|!grpc.web.ClientReadableStream<!MasternodeListResponse>

Example :

```js
const { SimplifiedMNList, SimplifiedMNListDiff } = require('@dashevo/dashcore-lib');

const stream = await client.subscribeToMasternodeList();

const list = new SimplifiedMNList();

stream
      .on('data', (response) => {
        const diffBuffer = Buffer.from(response.getMasternodeListDiff_asU8());
        const diff = new SimplifiedMNListDiff(diffBuffer);
        list.applyDiff(diff);
      })
    .on('error', (err) => {
        // do something with err
      });
```
