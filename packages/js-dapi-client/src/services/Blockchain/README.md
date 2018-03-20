### Blockchain : 

This is arhived service, nothing using it right now. Probably need to be removed.

Unstable stage for usage right now.

- init
- addBlock
- getBlock
- getLastBlock
- restore
- _normalizeHeader
- expectNewDifficult


TBD : 

DAPI-SDK has a internal Blockchain. These function will use the internal blockchain when possible and will retrieve when it won't.

- `SDK.Blockchain.init([options])` - Initialize the blockchain in order to be used. Optional default can be changed by passing one of these options :
    - options :
        - `autoConnect` - `Boolean` by default `true`. Disabling it will prevent the automatic socket connection.
        - `numberOfHeadersToFetch` - `Number` by default `100`, allow to specify how many headers to fetch at init.
        - `fullFetch` - `Boolean` by default `false`. Activating it allow to fetch all the blockchain headers from genesis to last tip. (event `fullFetched` emitted when end)
        This way you can setup a full blockchain fetch (numberOfHeadersFetched will then be ignored).

//- `SDK.Blockchain.expectNextDifficulty()` - Will expect the likely difficulty `Number` of the next block.
//- `SDK.Blockchain.validateBlocks(hash|height, [nbBlocks,[direction]])` - Will validate 25 or `Number` of block headers from an height `Number` or a Hash `String` in a `Number` direction.
//- `SDK.Blockchain.getBlock(height)` - Will return a block by it's height `Number`.
//- `SDK.Blockchain.getLastBlock()` - Will return the last block stored.
//- `SDK.Blockchain.addBlock(block)` - Will add a block headers.

The blockchain provide you also some events such as
    - `ready`
    - `socket.connected` - Where the argument provided is the socket itself.
    - `socket.block` - Where the argument provided is the block.
    - `socket.tx` - Where the argument provided is the TX.
