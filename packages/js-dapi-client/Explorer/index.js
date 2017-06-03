exports.Explorer = function(){
    let self = this;
    return {
        API:{
            //Method that do call by themselves  */
            getStatus:require('./insightAPI/getStatus').getStatus,
            getMNlist:require('./insightAPI/getStatus').getMNlist,
            getBlock:require('./insightAPI/getBlock').getBlock,//,
            getHashFromHeight:require('./insightAPI/getHashFromHeight').getHashFromHeight,//,
            getLastBlockHash:require('./insightAPI/getLastBlockHash').getLastBlockHash,
            /**
             * @param  {String|Number} - identifier - A number (height) or a string (hash) of the starting point from which to retrieve
             * @param {Number=25} - nbOfBlock - the number of block to retrieve
             * @param {Number=1} - direction - a direction to take where 1 is asc and -1 desc.
             * @returns {Promise} - When resolved, contains a list of headers
             */
            getBlockHeaders:require('./insightAPI/getBlockHeaders').getBlockHeaders,
            /* Methods that will use another call and works as a alias */
            getBlockConfirmations:require('./insightAPI/getBlockConfirmations').getBlockConfirmations,
            getBlockSize:require('./insightAPI/getBlockSize').getBlockSize,
            getBlockBits:require('./insightAPI/getBlockBits').getBlockBits,
            getBlockChainwork:require('./insightAPI/getBlockChainwork').getBlockChainwork,
            getBlockMerkleRoot:require('./insightAPI/getBlockMerkleRoot').getBlockMerkleRoot,
            getBlockTransactions:require('./insightAPI/getBlockTransactions').getBlockTransactions,
            getBlockTime:require('./insightAPI/getBlockTime').getBlockTime,
            getBlockVersion:require('./insightAPI/getBlockVersion').getBlockVersion,
            getHeightFromHash:require('./insightAPI/getHeightFromHash').getHeightFromHash,
            getLastDifficulty:require('./insightAPI/getLastDifficulty').getLastDifficulty,
            getLastBlockHeight:require('./insightAPI/getLastBlockHeight').getLastBlockHeight,
            getLastBlock:require('./insightAPI/getLastBlock').getLastBlock,
            // address routes
            getBalance:require('./insightAPI/address').getBalance,//,
            getUTXO:require('./insightAPI/address').getUTXO,//,
            // TX routes
            send:require('./insightAPI/tx').send,
            getTx:require('./insightAPI/tx').getTx,

            // util routes
            estimateFees:require('./insightAPI/utils').estimateFees,


        }
    };
};
