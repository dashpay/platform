exports.Explorer = function(){
    return {
        API:{
            //Method that do call by themselves  */
            getStatus:require('./API/getStatus').getStatus,
            getMasternodeList:require('./API/getMasternodeList').getMasternodeList,
            getBlock:require('./API/getBlock').getBlock,//,
            getHashFromHeight:require('./API/getHashFromHeight').getHashFromHeight,//,
            getLastBlockHash:require('./API/getLastBlockHash').getLastBlockHash,
            /**
             * @param  {String|Number} - identifier - A number (height) or a string (hash) of the starting point from which to retrieve
             * @param {Number=25} - nbOfBlock - the number of block to retrieve
             * @param {Number=1} - direction - a direction to take where 1 is asc and -1 desc.
             * @returns {Promise} - When resolved, contains a list of headers
             */
            getBlockHeaders:require('./API/getBlockHeaders').getBlockHeaders,
            /* Methods that will use another call and works as a alias */
            getBlockConfirmations:require('./API/getBlockConfirmations').getBlockConfirmations,
            getBlockSize:require('./API/getBlockSize').getBlockSize,
            getBlockBits:require('./API/getBlockBits').getBlockBits,
            getBlockChainwork:require('./API/getBlockChainwork').getBlockChainwork,
            getBlockMerkleRoot:require('./API/getBlockMerkleRoot').getBlockMerkleRoot,
            getBlockTransactions:require('./API/getBlockTransactions').getBlockTransactions,
            getBlockTime:require('./API/getBlockTime').getBlockTime,
            getBlockVersion:require('./API/getBlockVersion').getBlockVersion,
            getHeightFromHash:require('./API/getHeightFromHash').getHeightFromHash,
            getLastDifficulty:require('./API/getLastDifficulty').getLastDifficulty,
            getLastBlockHeight:require('./API/getLastBlockHeight').getLastBlockHeight,
            getLastBlock:require('./API/getLastBlock').getLastBlock,
            // address routes
            getBalance:require('./API/address').getBalance,//,
            getUTXO:require('./API/address').getUTXO,//,
            // TX routes
            send:require('./API/tx').send,
            getTx:require('./API/tx').getTx,

            // util routes
            estimateFees:require('./API/utils').estimateFees,


        }
    };
};
