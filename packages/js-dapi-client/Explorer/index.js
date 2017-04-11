exports.Explorer = function(){
    let self = this;
    return {
        API:{
            //Method that do call by themselves  */
            getStatus:require('./insightAPI/getStatus').getStatus.call(self),
            getBlock:require('./insightAPI/getBlock').getBlock.call(self),
            getHashFromHeight:require('./insightAPI/getHashFromHeight').getHashFromHeight.call(self),
            /* Methods that will use another call and works as a alias */
            getBlockConfirmations:require('./insightAPI/getBlockConfirmations').getBlockConfirmations.call(self),
            getBlockSize:require('./insightAPI/getBlockSize').getBlockSize.call(self),
            getHeightFromHash:require('./insightAPI/getHeightFromHash').getHeightFromHash.call(self),
            getLastDifficulty:require('./insightAPI/getLastDifficulty').getLastDifficulty.call(self),
            getLastBlockHeight:require('./insightAPI/getLastBlockHeight').getLastBlockHeight.call(self),

        }
    };
};