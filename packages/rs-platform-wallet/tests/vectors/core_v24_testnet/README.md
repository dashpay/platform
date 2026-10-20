Testnet block 1575845
(`000000787345d5662bdb4107b9749a0e8f552cee5196e9531523986d94b90bb2`), mined
on 2026-10-20 while the network ran Dash Core 24.0 and V24 was not yet
active, as consensus bytes in hex. Its coinbase carries a version 3 CbTx;
Core requires version 4 only once V24 is active.

The file is the block's 80-byte header and its one transaction, the coinbase
`443e9b24…`, both fetched from the testnet Insight API (`rawblock` for the
header, `rawtx` for the coinbase). Insight's `rawblock` re-serializes the
coinbase with the CbTx merkle roots byte-reversed, so those bytes do not
hash to the header's merkle root; the `rawtx` bytes do.
