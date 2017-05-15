exports.Explorer = function(){
    let self = this;
    return {
        API:{
            //Method that do call by themselves  */
            getStatus:require('./insightAPI/getStatus').getStatus.call(self),
            getMNlist:require('./insightAPI/getStatus').getMNlist.call(self),
            getBlock:require('./insightAPI/getBlock').getBlock.call(self),
            getHashFromHeight:require('./insightAPI/getHashFromHeight').getHashFromHeight.call(self),
            getLastBlockHash:require('./insightAPI/getLastBlockHash').getLastBlockHash.call(self),
            /**
             * @param  {String|Number} - identifier - A number (height) or a string (hash) of the starting point from which to retrieve
             * @param {Number=25} - nbOfBlock - the number of block to retrieve
             * @param {Number=1} - direction - a direction to take where 1 is asc and -1 desc.
             * @returns {Promise} - When resolved, contains a list of headers
             */
            getBlockHeaders:require('./insightAPI/getBlockHeaders').getBlockHeaders.call(self),
            /* Methods that will use another call and works as a alias */
            getBlockConfirmations:require('./insightAPI/getBlockConfirmations').getBlockConfirmations.call(self),
            getBlockSize:require('./insightAPI/getBlockSize').getBlockSize.call(self),
            getBlockBits:require('./insightAPI/getBlockBits').getBlockBits.call(self),
            getBlockChainwork:require('./insightAPI/getBlockChainwork').getBlockChainwork.call(self),
            getBlockMerkleRoot:require('./insightAPI/getBlockMerkleRoot').getBlockMerkleRoot.call(self),
            getBlockTransactions:require('./insightAPI/getBlockTransactions').getBlockTransactions.call(self),
            getBlockTime:require('./insightAPI/getBlockTime').getBlockTime.call(self),
            getBlockVersion:require('./insightAPI/getBlockVersion').getBlockVersion.call(self),
            getHeightFromHash:require('./insightAPI/getHeightFromHash').getHeightFromHash.call(self),
            getLastDifficulty:require('./insightAPI/getLastDifficulty').getLastDifficulty.call(self),
            getLastBlockHeight:require('./insightAPI/getLastBlockHeight').getLastBlockHeight.call(self),
            getLastBlock:require('./insightAPI/getLastBlock').getLastBlock.call(self),
            // address routes
            getBalance:require('./insightAPI/address').getBalance.call(self),
            getUTXO:require('./insightAPI/address').getUTXO.call(self),
            send:require('./insightAPI/tx').send.call(self),
            // util routes
            estimateFees:require('./insightAPI/utils').estimateFees.call(self),


        }
    };
};
