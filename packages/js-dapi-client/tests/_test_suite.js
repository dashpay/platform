require('./index.js');
require('./Accounts/Authentication/auth.js');
require('./Accounts/User/create.js'); //sendtx gives an issue to be investigated
require('./Accounts/User/login.js');
// require('./BWS/broadcastRawTx.js'); //test does not make sense (rewrite), sendtx does not seem to work on dapi
require('./BWS/getBalance.js'); //takes too long (yb21342iADyqAotjwcn4imqjvAcdYhnzeH 37k trx)
// require('./BWS/getFeeLevels.js'); //siampm does not implement /api/utils/estimatefee
require('./BWS/getFiatRate.js');
require('./BWS/getMainAddress.js');
require('./BWS/getTx.js');
// require('./BWS/getTxHistory.js'); //takes too long (yj6xVHMyZGBdLqGUfoGc9gDvU8tHx6iqb4 2.5k trx) 
// require('./BWS/getUtxos.js'); //takes too long (yb21342iADyqAotjwcn4imqjvAcdYhnzeH)
require('./Explorer/API/getStatus.js');
require('./Explorer/API/getBlock.js');
require('./Explorer/API/getBlockBits.js');
require('./Explorer/API/getBlockChainwork.js');
require('./Explorer/API/getBlockConfirmations.js');
require('./Explorer/API/getBlockMerkleRoot.js');
require('./Explorer/API/getBlockSize.js');
require('./Explorer/API/getBlockTime.js');
require('./Explorer/API/getBlockTransactions.js');
require('./Explorer/API/getBlockVersion.js');
require('./Explorer/API/getHashFromHeight.js');
require('./Explorer/API/getHeightFromHash.js');
require('./Explorer/API/getLastBlock.js');
require('./Explorer/API/getLastBlockHash.js');
require('./Explorer/API/getLastBlockHeight.js');
require('./Explorer/API/getLastDifficulty.js');
require('./util/mnemonic.js');

// //FIXME If a block is mined during the fetching process, this data, when verified will be shifted and won't equal.
// require('./Explorer/API/getBlockHeaders.js')
