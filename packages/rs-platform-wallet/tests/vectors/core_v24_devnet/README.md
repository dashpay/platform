Transactions mined by Dash Core v24.0.0-rc.3 on a devnet where V24 activated at
height 15000, fetched through DAPI `Core/getTransaction`. Each file is the
consensus encoding in hex, as Core relayed it; the tests check it hashes to the
txid Core reported.

`block_15146.hex` is a block from the same devnet, fetched through DAPI
`Core/getBlock` (the copy rs-dapi's tests use). Its coinbase carries a version 4
CbTx, the version Core requires while V24 is active.
